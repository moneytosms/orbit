//! JS bindings over orbit-core. Values cross as JSON strings.
use orbit_core::{Command, Hlc, INBOX, Row};
use wasm_bindgen::prelude::*;

/// `random` must be 10 bytes from `crypto.getRandomValues`.
#[wasm_bindgen]
pub fn new_id(now_ms: f64, random: &[u8]) -> Result<String, JsError> {
    let random = random
        .try_into()
        .map_err(|_| JsError::new("random must be 10 bytes"))?;
    Ok(orbit_core::new_id(now_ms as u64, random).to_string())
}

#[wasm_bindgen]
pub fn next_hlc(prev: &str, now_ms: f64) -> Result<String, JsError> {
    let prev: Hlc = serde_json::from_str(prev)?;
    Ok(serde_json::to_string(&prev.next(now_ms as u64))?)
}

/// Predicts a command's effect on its target row. Throws the error code on rejection.
#[wasm_bindgen]
pub fn apply(existing: Option<String>, command: &str) -> Result<String, JsError> {
    let existing: Option<Row> = existing.map(|s| serde_json::from_str(&s)).transpose()?;
    let cmd: Command = serde_json::from_str(command)?;
    match orbit_core::apply(existing, &cmd) {
        Ok(row) => Ok(serde_json::to_string(&row)?),
        Err(code) => Err(JsError::new(
            serde_json::to_value(code)?.as_str().unwrap_or("error"),
        )),
    }
}

/// The completion record `command` (a task, goal or milestone completion) logs on `existing`, if any.
#[wasm_bindgen]
pub fn completion_for(existing: Option<String>, command: &str) -> Result<Option<String>, JsError> {
    let existing: Option<Row> = existing.map(|s| serde_json::from_str(&s)).transpose()?;
    let cmd: Command = serde_json::from_str(command)?;
    let Some(old) = existing else {
        return Ok(None);
    };
    match orbit_core::completion_for(&old, &cmd) {
        Some(c) => Ok(Some(serde_json::to_string(&Row::Completion(c))?)),
        None => Ok(None),
    }
}

/// `{progress, behind}` for goal `goal_id` over `rows_json` (a Row array); `today` is `YYYY-MM-DD`.
#[wasm_bindgen]
pub fn goal_progress(rows_json: &str, goal_id: &str, today: &str) -> Result<String, JsError> {
    let rows: Vec<Row> = serde_json::from_str(rows_json)?;
    let id: orbit_core::Uuid = goal_id.parse().map_err(|_| JsError::new("bad goal id"))?;
    let Some(Row::Goal(goal)) = rows.iter().find(|r| r.id() == id) else {
        return Err(JsError::new("no such goal"));
    };
    let progress = orbit_core::goal::progress(goal, &rows);
    let behind = orbit_core::goal::behind_pace(goal, progress, today);
    Ok(serde_json::json!({ "progress": progress, "behind": behind }).to_string())
}

#[wasm_bindgen]
pub fn inbox_id() -> String {
    INBOX.to_string()
}

/// The HLC receive step over a response's `clock`.
#[wasm_bindgen]
pub fn observe_hlc(mine: &str, seen: &str, now_ms: f64) -> Result<String, JsError> {
    let (mine, seen): (Hlc, Hlc) = (serde_json::from_str(mine)?, serde_json::from_str(seen)?);
    Ok(serde_json::to_string(&mine.observe(&seen, now_ms as u64))?)
}

/// A fractional-index key strictly between `a` and `b` (`None` is an open end).
#[wasm_bindgen]
pub fn order_between(a: Option<String>, b: Option<String>) -> String {
    orbit_core::order::between(a.as_deref(), b.as_deref())
}

/// Quick-add parse of `input`; `now` is local `YYYY-MM-DDTHH:MM`, `known_json` a `Known`.
#[wasm_bindgen]
pub fn parse_quick_add(input: &str, now: &str, known_json: &str) -> Result<String, JsError> {
    let known: orbit_core::parse::Known = serde_json::from_str(known_json)?;
    Ok(serde_json::to_string(&orbit_core::parse::parse(
        input, now, &known,
    ))?)
}

/// Every bundled palette as a JSON array: name, grade, offered strategies, featured flag and swatch colours.
#[wasm_bindgen]
pub fn theme_list() -> String {
    let featured = orbit_core::theme::featured();
    let list: Vec<serde_json::Value> = orbit_core::theme::palettes()
        .iter()
        .map(|(name, p)| {
            serde_json::json!({
                "name": name,
                "grade": orbit_core::theme::grade(p),
                "offered": orbit_core::theme::offered(p),
                "featured": featured.contains(name),
                "bg": p.bg,
                "text": p.text,
                "main": p.main,
            })
        })
        .collect();
    serde_json::to_string(&list).expect("theme list serialises")
}

/// The app.css token overrides for palette `name` under `strategy` (`direct|derived|accent`), as a JSON object.
#[wasm_bindgen]
pub fn theme_tokens(name: &str, strategy: &str) -> Result<String, JsError> {
    let palettes = orbit_core::theme::palettes();
    let palette = palettes
        .get(name)
        .ok_or_else(|| JsError::new("unknown theme"))?;
    let strategy: orbit_core::theme::Strategy =
        serde_json::from_value(serde_json::Value::String(strategy.to_string()))?;
    Ok(serde_json::to_string(&orbit_core::theme::tokens(
        palette, strategy,
    ))?)
}
