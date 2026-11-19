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
