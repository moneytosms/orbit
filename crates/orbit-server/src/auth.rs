//! Single-user auth: a first-run setup code, argon2id password, optional TOTP, and Devices
//! that each hold one hashed 256-bit token (Bearer for native, HttpOnly cookie for web).
use crate::{St, lock, now_ms, open};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::{
    Json,
    extract::{ConnectInfo, FromRequestParts, Path, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};
use totp_rs::{Builder, Secret, Totp};

const COOKIE: &str = "orbit";
const MIN_PASSWORD: usize = 8;

pub(crate) fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS user (id INTEGER PRIMARY KEY CHECK (id = 1), password TEXT NOT NULL, totp TEXT, totp_pending TEXT);
         CREATE TABLE IF NOT EXISTS devices (id TEXT PRIMARY KEY, name TEXT NOT NULL, platform TEXT NOT NULL,
           created INTEGER NOT NULL, last_seen INTEGER NOT NULL, token TEXT NOT NULL UNIQUE);",
    )
}

/// A fresh one-time code if no user exists yet.
pub(crate) fn setup_code(conn: &Connection) -> rusqlite::Result<Option<String>> {
    let has_user: bool = conn.query_row("SELECT COUNT(*) > 0 FROM user", [], |r| r.get(0))?;
    Ok((!has_user).then(|| hex::encode(random::<8>())))
}

/// Host-side last resort: new password, TOTP off, every Device revoked.
pub fn reset_password(
    data_dir: &std::path::Path,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if password.chars().count() < MIN_PASSWORD {
        return Err(format!("password needs at least {MIN_PASSWORD} characters").into());
    }
    let conn = open(data_dir)?;
    migrate(&conn)?;
    let n = conn.execute(
        "UPDATE user SET password = ?, totp = NULL, totp_pending = NULL",
        [hash_password(password)],
    )?;
    if n == 0 {
        return Err("no user yet: finish first-run setup in the web app".into());
    }
    conn.execute("DELETE FROM devices", [])?;
    Ok(())
}

/// An error response with `{"error": code}`; database errors become a logged 500.
pub(crate) struct Fail(Box<Response>);
type Res<T = Response> = Result<T, Fail>;

impl IntoResponse for Fail {
    fn into_response(self) -> Response {
        *self.0
    }
}
impl From<rusqlite::Error> for Fail {
    fn from(e: rusqlite::Error) -> Self {
        eprintln!("auth: {e}");
        Fail(Box::new(StatusCode::INTERNAL_SERVER_ERROR.into_response()))
    }
}
fn fail(status: StatusCode, code: &str) -> Fail {
    Fail(Box::new(
        (status, Json(json!({ "error": code }))).into_response(),
    ))
}

/// The signed-in Device; rejects with 401 when the token is missing, unknown or revoked.
pub(crate) struct Device(pub String);

impl FromRequestParts<St> for Device {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, st: &St) -> Result<Self, StatusCode> {
        let token = presented(&parts.headers).ok_or(StatusCode::UNAUTHORIZED)?;
        let id: Option<String> = lock(&st.db)
            .query_row(
                "UPDATE devices SET last_seen = ?1 WHERE token = ?2 RETURNING id",
                (now_ms(), sha256(&token)),
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| {
                eprintln!("auth: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        id.map(Device).ok_or(StatusCode::UNAUTHORIZED)
    }
}

/// The Bearer token, else the `orbit` cookie.
fn presented(headers: &HeaderMap) -> Option<String> {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let cookie = || {
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .find_map(|c| c.trim().strip_prefix("orbit="))
    };
    bearer.or_else(cookie).map(str::to_string)
}

pub(crate) async fn state(State(st): State<St>) -> Json<serde_json::Value> {
    Json(json!({ "setup": lock(&st.setup).is_some() }))
}

#[derive(Deserialize)]
pub(crate) struct Setup {
    code: String,
    password: String,
    name: String,
    platform: String,
}

pub(crate) async fn setup(
    State(st): State<St>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(b): Json<Setup>,
) -> Res {
    let ip = client_ip(addr, &headers);
    limited(&st, ip)?;
    let mut code = lock(&st.setup);
    if code.as_deref() != Some(b.code.trim()) {
        drop(code);
        failed(&st, ip, "setup");
        return Err(fail(StatusCode::FORBIDDEN, "setup_code"));
    }
    check_strength(&b.password)?;
    let hashed = hash_password(&b.password);
    let db = lock(&st.db);
    db.execute("INSERT INTO user (id, password) VALUES (1, ?)", [hashed])?;
    *code = None;
    succeeded(&st, ip);
    issue(&db, &b.name, &b.platform)
}

#[derive(Deserialize)]
pub(crate) struct Login {
    password: String,
    totp: Option<String>,
    name: String,
    platform: String,
}

pub(crate) async fn login(
    State(st): State<St>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(b): Json<Login>,
) -> Res {
    let ip = client_ip(addr, &headers);
    limited(&st, ip)?;
    let (hash, secret) = user(&st)?;
    if !verify_password(&b.password, &hash) {
        failed(&st, ip, "login");
        return Err(fail(StatusCode::UNAUTHORIZED, "credentials"));
    }
    if let Some(secret) = secret {
        // Password was right: ask for the code without counting a failure.
        let Some(code) = b.totp.filter(|c| !c.is_empty()) else {
            return Err(fail(StatusCode::UNAUTHORIZED, "totp"));
        };
        if totp(&secret).check_current(code.trim()).is_none() {
            failed(&st, ip, "totp");
            return Err(fail(StatusCode::UNAUTHORIZED, "totp"));
        }
    }
    succeeded(&st, ip);
    issue(&lock(&st.db), &b.name, &b.platform)
}

#[derive(Deserialize)]
pub(crate) struct Password {
    current: String,
    new: String,
    #[serde(default)]
    sign_out_others: bool,
}

pub(crate) async fn password(
    State(st): State<St>,
    Device(me): Device,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(b): Json<Password>,
) -> Res {
    reauth(
        &st,
        client_ip(addr, &headers),
        &b.current,
        "password change",
    )?;
    check_strength(&b.new)?;
    let hashed = hash_password(&b.new);
    let db = lock(&st.db);
    db.execute("UPDATE user SET password = ?", [hashed])?;
    if b.sign_out_others {
        db.execute("DELETE FROM devices WHERE id != ?", [&me])?;
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Deserialize)]
pub(crate) struct Reauth {
    password: String,
}

/// Starts TOTP enrollment: a new secret that takes effect once a code from it is confirmed.
/// Needs the password, so a stolen Device token can't swap the authenticator.
pub(crate) async fn totp_start(
    State(st): State<St>,
    _: Device,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(b): Json<Reauth>,
) -> Res {
    reauth(
        &st,
        client_ip(addr, &headers),
        &b.password,
        "totp enrollment",
    )?;
    let secret = Secret::from(random::<20>()).to_base32();
    lock(&st.db).execute("UPDATE user SET totp_pending = ?", [&secret])?;
    let url = totp(&secret).to_url().map_err(|e| {
        eprintln!("auth: {e}");
        fail(StatusCode::INTERNAL_SERVER_ERROR, "totp_url")
    })?;
    Ok(Json(json!({ "secret": secret, "url": url })).into_response())
}

#[derive(Deserialize)]
pub(crate) struct Code {
    code: String,
}

pub(crate) async fn totp_confirm(State(st): State<St>, _: Device, Json(b): Json<Code>) -> Res {
    let db = lock(&st.db);
    let pending: Option<String> =
        db.query_row("SELECT totp_pending FROM user", [], |r| r.get(0))?;
    match pending {
        Some(secret) if totp(&secret).check_current(b.code.trim()).is_some() => {
            db.execute(
                "UPDATE user SET totp = totp_pending, totp_pending = NULL",
                [],
            )?;
            Ok(StatusCode::NO_CONTENT.into_response())
        }
        _ => Err(fail(StatusCode::BAD_REQUEST, "totp")),
    }
}

pub(crate) async fn devices(State(st): State<St>, Device(me): Device) -> Res {
    let db = lock(&st.db);
    let mut stmt =
        db.prepare("SELECT id, name, platform, created, last_seen FROM devices ORDER BY created")?;
    let list = stmt
        .query_map([], |r| {
            let id: String = r.get(0)?;
            Ok(
                json!({ "current": id == me, "id": id, "name": r.get::<_, String>(1)?,
                       "platform": r.get::<_, String>(2)?, "created": r.get::<_, i64>(3)?,
                       "last_seen": r.get::<_, i64>(4)? }),
            )
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Json(list).into_response())
}

#[derive(Deserialize)]
pub(crate) struct Named {
    name: String,
}

/// "Create token": a named Device for scripts, without logging in on it.
pub(crate) async fn create_token(State(st): State<St>, _: Device, Json(b): Json<Named>) -> Res {
    issue(&lock(&st.db), &b.name, "script")
}

pub(crate) async fn revoke(State(st): State<St>, _: Device, Path(id): Path<String>) -> Res {
    match lock(&st.db).execute("DELETE FROM devices WHERE id = ?", [id])? {
        0 => Err(fail(StatusCode::NOT_FOUND, "device")),
        _ => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

/// Creates a Device. Its id is the HLC device id. Web gets the token as a cookie only.
fn issue(db: &Connection, name: &str, platform: &str) -> Res {
    let name = name.trim();
    if name.is_empty() || name.len() > 100 || platform.is_empty() || platform.len() > 20 {
        return Err(fail(StatusCode::BAD_REQUEST, "device_name"));
    }
    let now = now_ms();
    let id = orbit_core::new_id(now as u64, &random()).to_string();
    let token = hex::encode(random::<32>());
    db.execute(
        "INSERT INTO devices (id, name, platform, created, last_seen, token) VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
        (&id, name, platform, now, sha256(&token)),
    )?;
    if platform == "web" {
        // No expiry: the cookie lives as long as the Device (ten years).
        let cookie =
            format!("{COOKIE}={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=315360000");
        Ok((
            [(header::SET_COOKIE, cookie)],
            Json(json!({ "device": id })),
        )
            .into_response())
    } else {
        Ok(Json(json!({ "device": id, "token": token })).into_response())
    }
}

/// The password hash and enabled TOTP secret; 403 before setup.
fn user(st: &St) -> Res<(String, Option<String>)> {
    lock(&st.db)
        .query_row("SELECT password, totp FROM user", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .optional()?
        .ok_or_else(|| fail(StatusCode::FORBIDDEN, "setup_needed"))
}

/// A signed-in Device proving it knows the password; failures count toward the IP's lockout.
fn reauth(st: &St, ip: IpAddr, password: &str, what: &str) -> Res<()> {
    limited(st, ip)?;
    let (hash, _) = user(st)?;
    if !verify_password(password, &hash) {
        failed(st, ip, what);
        return Err(fail(StatusCode::FORBIDDEN, "credentials"));
    }
    Ok(())
}

fn check_strength(password: &str) -> Res<()> {
    if password.chars().count() < MIN_PASSWORD {
        return Err(fail(StatusCode::BAD_REQUEST, "weak_password"));
    }
    Ok(())
}

fn hash_password(password: &str) -> String {
    Argon2::default()
        .hash_password(password.as_bytes())
        .expect("argon2 with default params")
        .to_string()
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}

fn totp(secret: &str) -> Totp {
    Builder::new()
        .with_secret(Secret::try_from_base32(secret).expect("stored base32"))
        .with_account_name("orbit")
        .with_issuer(Some("Orbit"))
        .build()
        .expect("20-byte secret")
}

fn sha256(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn random<const N: usize>() -> [u8; N] {
    let mut b = [0; N];
    getrandom::fill(&mut b).expect("OS randomness");
    b
}

/// The address the limiter keys on. Behind a same-host proxy or tunnel every peer is loopback, so
/// there (and only there; elsewhere the header is spoofable) trust `CF-Connecting-IP`, else the
/// first `X-Forwarded-For` entry.
fn client_ip(peer: SocketAddr, headers: &HeaderMap) -> IpAddr {
    if peer.ip().is_loopback() {
        let named = |name: &str| headers.get(name)?.to_str().ok();
        let forwarded = named("cf-connecting-ip")
            .or_else(|| named("x-forwarded-for")?.split(',').next())
            .and_then(|v| v.trim().parse().ok());
        if let Some(ip) = forwarded {
            return ip;
        }
    }
    peer.ip()
}

/// Failed attempts from one IP: the 5th locks it out for 1 min, each later one twice as long.
#[derive(Clone, Copy)]
pub(crate) struct Limit {
    fails: u32,
    last: Instant,
    until: Instant,
}

fn limited(st: &St, ip: IpAddr) -> Res<()> {
    match lock(&st.limits).get(&ip) {
        Some(l) if Instant::now() < l.until => {
            Err(fail(StatusCode::TOO_MANY_REQUESTS, "rate_limited"))
        }
        _ => Ok(()),
    }
}

fn failed(st: &St, ip: IpAddr, what: &str) {
    const FORGET: Duration = Duration::from_secs(3600);
    let now = Instant::now();
    let mut limits = lock(&st.limits);
    limits.retain(|_, l| now.duration_since(l.last) < FORGET);
    let l = limits.entry(ip).or_insert(Limit {
        fails: 0,
        last: now,
        until: now,
    });
    l.fails += 1;
    l.last = now;
    if l.fails >= 5 {
        l.until = now + Duration::from_secs(60) * 2u32.pow((l.fails - 5).min(6));
    }
    eprintln!("auth: failed {what} from {ip} ({} in a row)", l.fails);
}

fn succeeded(st: &St, ip: IpAddr) {
    lock(&st.limits).remove(&ip);
}
