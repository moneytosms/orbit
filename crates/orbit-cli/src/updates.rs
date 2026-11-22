use crate::out::{Done, fail, internal};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const DEFAULT_RELEASES: &str = "https://api.github.com/repos/moneytosms/orbit/releases";

#[derive(Clone, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Clone, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub fn self_update(dry_run: bool) -> Done {
    if std::env::consts::OS != "linux" {
        return Err(fail(
            "unsupported_platform",
            "prebuilt CLI updates are available for Linux x86_64 and aarch64",
            Some("install a new build from source or use the Linux release"),
        ));
    }
    let releases = std::env::var("ORBIT_RELEASE_API").unwrap_or_else(|_| DEFAULT_RELEASES.into());
    let exe = std::env::current_exe().map_err(internal)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(internal)?;
    runtime.block_on(update(&releases, &exe, dry_run))
}

async fn latest_release(releases: &str) -> Result<Release, String> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?
        .get(format!("{}/latest", releases.trim_end_matches('/')))
        .header(
            "User-Agent",
            format!("orbit-cli/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    response.json().await.map_err(|e| e.to_string())
}

async fn update(releases: &str, exe: &Path, dry_run: bool) -> Done {
    let release = latest_release(releases).await.map_err(|e| {
        fail(
            "update_check_failed",
            e,
            Some("check your connection and try again"),
        )
    })?;
    update_release(release, exe, dry_run).await
}

async fn update_release(release: Release, exe: &Path, dry_run: bool) -> Done {
    let latest = release.tag_name.trim_start_matches('v');
    if compare_versions(latest, env!("CARGO_PKG_VERSION")).is_none() {
        return Err(fail(
            "update_invalid_version",
            format!("invalid release tag `{}`", release.tag_name),
            None,
        ));
    }
    if compare_versions(latest, env!("CARGO_PKG_VERSION")) != Some(std::cmp::Ordering::Greater) {
        return Ok((
            format!("Orbit CLI {} is up to date", env!("CARGO_PKG_VERSION")),
            json!({ "current": env!("CARGO_PKG_VERSION"), "latest": latest, "updated": false, "dry_run": dry_run }),
        ));
    }

    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => {
            return Err(fail(
                "unsupported_platform",
                format!("no prebuilt CLI for {}", std::env::consts::ARCH),
                None,
            ));
        }
    };
    let archive_name = format!("orbit-{latest}-linux-{arch}.tar.gz");
    let archive_url = asset_url(&release, &archive_name)?;
    let sums_url = asset_url(&release, "SHA256SUMS")?;
    let (archive, sums) =
        tokio::try_join!(download(&archive_url), download(&sums_url)).map_err(|e| {
            fail(
                "update_download_failed",
                e,
                Some("the installed CLI was left unchanged"),
            )
        })?;
    let expected = checksum(&sums, &archive_name).ok_or_else(|| {
        fail(
            "update_checksum_missing",
            format!("SHA256SUMS has no entry for {archive_name}"),
            None,
        )
    })?;
    let actual = digest(&archive);
    if actual != expected {
        return Err(fail(
            "update_checksum_mismatch",
            format!("checksum mismatch for {archive_name}; the installed CLI was left unchanged"),
            Some("retry after the release assets have finished uploading"),
        ));
    }

    let temp = tempfile::tempdir().map_err(internal)?;
    let archive_path = temp.path().join(&archive_name);
    fs::write(&archive_path, archive).map_err(internal)?;
    let extracted = Command::new("tar")
        .args(["-xzf", archive_path.to_string_lossy().as_ref(), "-C"])
        .arg(temp.path())
        .arg(format!(
            "{}/orbit",
            archive_name.trim_end_matches(".tar.gz")
        ))
        .status()
        .map_err(internal)?;
    if !extracted.success() {
        return Err(fail(
            "update_archive_invalid",
            "release archive could not be unpacked",
            None,
        ));
    }
    let staged = temp
        .path()
        .join(archive_name.trim_end_matches(".tar.gz"))
        .join("orbit");
    if !staged.is_file() {
        return Err(fail(
            "update_archive_invalid",
            "release archive did not contain the orbit executable",
            None,
        ));
    }

    if dry_run {
        return Ok((
            format!("verified Orbit CLI {latest}; dry run, executable unchanged"),
            json!({ "current": env!("CARGO_PKG_VERSION"), "latest": latest, "updated": false, "dry_run": true, "checksum_verified": true }),
        ));
    }
    replace_executable(exe, &staged).map_err(|e| {
        fail(
            "update_replace_failed",
            format!("could not replace CLI: {e}"),
            Some("the installed CLI was left unchanged"),
        )
    })?;
    Ok((
        format!("updated Orbit CLI to {latest}"),
        json!({ "current": env!("CARGO_PKG_VERSION"), "latest": latest, "updated": true, "dry_run": false, "checksum_verified": true }),
    ))
}

async fn download(url: &str) -> Result<Vec<u8>, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| e.to_string())
}

fn asset_url(release: &Release, name: &str) -> Result<String, crate::out::Fail> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == name)
        .map(|asset| asset.browser_download_url.clone())
        .ok_or_else(|| {
            fail(
                "update_asset_missing",
                format!("release is missing {name}"),
                None,
            )
        })
}

fn checksum(sums: &[u8], name: &str) -> Option<String> {
    std::str::from_utf8(sums).ok()?.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let hash = fields.next()?;
        (fields.next()? == name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

fn digest(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn replace_executable(exe: &Path, staged: &Path) -> std::io::Result<()> {
    let parent = exe.parent().unwrap_or_else(|| Path::new("."));
    let replacement = parent.join(format!(".orbit-update-{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&replacement)?;
    let result = (|| {
        std::io::copy(&mut fs::File::open(staged)?, &mut file)?;
        file.sync_all()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&replacement, fs::Permissions::from_mode(0o755))?;
        }
        fs::rename(&replacement, exe)
    })();
    if result.is_err() {
        let _ = fs::remove_file(replacement);
    }
    result
}

fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let parse = |s: &str| -> Option<Vec<u64>> {
        let parts: Vec<_> = s
            .split('.')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        (parts.len() == 3).then_some(parts)
    };
    Some(parse(left)?.cmp(&parse(right)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;
    use axum::routing::get;
    use serde_json::json;

    #[test]
    fn checksum_requires_exact_asset_name_and_sha256_shape() {
        let body = format!("{}  orbit-1.2.3-linux-x86_64.tar.gz\n", "a".repeat(64));
        assert_eq!(
            checksum(body.as_bytes(), "orbit-1.2.3-linux-x86_64.tar.gz"),
            Some("a".repeat(64))
        );
        assert_eq!(
            checksum(body.as_bytes(), "orbit-1.2.3-linux-aarch64.tar.gz"),
            None
        );
        assert_eq!(checksum(b"bad  archive.tar.gz\n", "archive.tar.gz"), None);
    }

    #[test]
    fn compares_lockstep_semver_parts() {
        assert_eq!(
            compare_versions("0.10.0", "0.9.9"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            compare_versions("0.2.0", "0.2.0"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(compare_versions("latest", "0.2.0"), None);
    }

    async fn fake_release(archive: Vec<u8>, sum: String) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let latest_url = base.clone();
        let version = next_version();
        let archive_route = archive.clone();
        let sums_route = sum.clone();
        let app = axum::Router::new()
            .route("/latest", get(move || {
                let base = latest_url.clone();
                let version = version.clone();
                async move {
                    Json(json!({ "tag_name": format!("v{version}"), "assets": [
                        { "name": format!("orbit-{version}-linux-{}.tar.gz", std::env::consts::ARCH), "browser_download_url": format!("{base}/archive") },
                        { "name": "SHA256SUMS", "browser_download_url": format!("{base}/sums") }
                    ] }))
                }
            }))
            .route("/archive", get(move || async move { archive_route }))
            .route("/sums", get(move || async move { sums_route }));
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        base
    }

    fn fixture_archive(dir: &Path) -> Vec<u8> {
        let name = format!("orbit-{}-linux-{}", next_version(), std::env::consts::ARCH);
        fs::create_dir_all(dir.join(&name)).unwrap();
        fs::write(dir.join(&name).join("orbit"), b"new Orbit CLI").unwrap();
        let archive = dir.join("fixture.tar.gz");
        assert!(
            Command::new("tar")
                .args(["-czf"])
                .arg(&archive)
                .arg("-C")
                .arg(dir)
                .arg(name)
                .status()
                .unwrap()
                .success()
        );
        fs::read(archive).unwrap()
    }

    fn next_version() -> String {
        let mut parts: Vec<u64> = env!("CARGO_PKG_VERSION")
            .split('.')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .unwrap();
        parts[2] += 1;
        format!("{}.{}.{}", parts[0], parts[1], parts[2])
    }

    #[tokio::test]
    async fn staged_release_dry_run_verifies_without_replacing_executable() {
        let dir = tempfile::tempdir().unwrap();
        let archive = fixture_archive(dir.path());
        let version = next_version();
        let name = format!("orbit-{version}-linux-{}.tar.gz", std::env::consts::ARCH);
        let server = fake_release(archive.clone(), format!("{}  {name}\n", digest(&archive))).await;
        let target = dir.path().join("orbit");
        fs::write(&target, b"old runner").unwrap();
        let result = update(&server, &target, true).await.unwrap();
        assert!(result.0.contains(&format!("verified Orbit CLI {version}")));
        assert_eq!(fs::read(&target).unwrap(), b"old runner");
        update(&server, &target, false).await.unwrap();
        assert_eq!(fs::read(target).unwrap(), b"new Orbit CLI");
    }

    #[tokio::test]
    async fn corrupted_staged_release_keeps_existing_executable() {
        let dir = tempfile::tempdir().unwrap();
        let archive = fixture_archive(dir.path());
        let name = format!(
            "orbit-{}-linux-{}.tar.gz",
            next_version(),
            std::env::consts::ARCH
        );
        let server = fake_release(archive, format!("{}  {name}\n", "0".repeat(64))).await;
        let target = dir.path().join("orbit");
        fs::write(&target, b"old runner").unwrap();
        let result = update(&server, &target, false).await.unwrap_err();
        assert_eq!(result.code, "update_checksum_mismatch");
        assert_eq!(fs::read(target).unwrap(), b"old runner");
    }
}
