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

/// The id of the completion record `cmd_uuid` logs for `item_id`, so a rejected guess can be dropped.
#[wasm_bindgen]
pub fn completion_id(item_id: &str, cmd_uuid: &str) -> Result<String, JsError> {
    Ok(orbit_core::completion_id(item_id.parse()?, cmd_uuid.parse()?).to_string())
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

/// The ids of the rows in `rows_json` (a Row array) that are trashed, archived or under such a row.
#[wasm_bindgen]
pub fn hidden_ids(rows_json: &str) -> Result<Vec<String>, JsError> {
    let rows: Vec<Row> = serde_json::from_str(rows_json)?;
    let by_id: std::collections::HashMap<_, _> = rows.iter().map(|r| (r.id(), r.clone())).collect();
    let lookup = |id| by_id.get(&id).cloned();
    Ok(rows
        .iter()
        .filter(|r| orbit_core::trash::hidden(r, &lookup))
        .map(|r| r.id().to_string())
        .collect())
}

/// Runs a View (JSON) over `rows_json` (a Row array); returns `[{key, ids}]`. `today` is `YYYY-MM-DD`.
#[wasm_bindgen]
pub fn evaluate_view(view_json: &str, rows_json: &str, today: &str) -> Result<String, JsError> {
    let view: orbit_core::view::View = serde_json::from_str(view_json)?;
    let rows: Vec<Row> = serde_json::from_str(rows_json)?;
    Ok(serde_json::to_string(&orbit_core::view::evaluate(
        &view, &rows, today,
    ))?)
}

/// The built-in Views as a JSON array.
#[wasm_bindgen]
pub fn builtin_views() -> Result<String, JsError> {
    Ok(serde_json::to_string(&orbit_core::view::builtins())?)
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

/// Typed filter text to a filter tree (JSON). `known_json` is `{labels:[{id,name}]}`; a parse error is a JSON
/// `{start,end,message}` in the thrown message, with char offsets.
#[wasm_bindgen]
pub fn parse_filter(text: &str, known_json: &str, today: &str) -> Result<String, JsError> {
    let known: orbit_core::filter_text::Known = serde_json::from_str(known_json)?;
    match orbit_core::filter_text::parse(text, &known, today) {
        Ok(node) => Ok(serde_json::to_string(&node)?),
        Err(e) => Err(JsError::new(&serde_json::to_string(&e)?)),
    }
}

/// A filter tree (JSON) as typed filter text.
#[wasm_bindgen]
pub fn print_filter(node_json: &str, known_json: &str) -> Result<String, JsError> {
    let node: orbit_core::view::Node = serde_json::from_str(node_json)?;
    let known: orbit_core::filter_text::Known = serde_json::from_str(known_json)?;
    Ok(orbit_core::filter_text::print(&node, &known))
}

/// The Check-in id for a Routine and `YYYY-MM-DD` day, shared by every Device.
#[wasm_bindgen]
pub fn checkin_id(routine_id: &str, date: &str) -> Result<String, JsError> {
    Ok(orbit_core::routine::checkin_id(routine_id.parse()?, date).to_string())
}

/// Streaks, completion rate and per-day states of Routine `routine_id` over `rows_json` (a Row array).
#[wasm_bindgen]
pub fn routine_stats(rows_json: &str, routine_id: &str, today: &str) -> Result<String, JsError> {
    let rows: Vec<Row> = serde_json::from_str(rows_json)?;
    let id: orbit_core::Uuid = routine_id.parse()?;
    let Some(Row::Routine(r)) = rows.iter().find(|r| r.id() == id) else {
        return Err(JsError::new("no such routine"));
    };
    let checkins: Vec<_> = rows
        .iter()
        .filter_map(|r| match r {
            Row::Checkin(c) if c.routine_id == id => Some(c.clone()),
            _ => None,
        })
        .collect();
    let mut stats = serde_json::to_value(orbit_core::routine::stats(r, &checkins, today))?;
    stats["ended"] = orbit_core::routine::is_ended(r, today).into();
    stats["paused"] = orbit_core::routine::is_paused(r, today).into();
    Ok(stats.to_string())
}

/// The Day-start date at `now_ms`; `utc_offset_min` is the synced zone's offset at that moment.
#[wasm_bindgen]
pub fn local_day(now_ms: f64, utc_offset_min: i32, day_start: &str) -> String {
    orbit_core::routine::local_day(now_ms as u64, utc_offset_min, day_start)
}

/// The What's next list for `rows_json`, as `[{ id, tier, reason, breadcrumb }]`.
#[wasm_bindgen]
pub fn whats_next(
    rows_json: &str,
    today: &str,
    now_time: &str,
    cfg_json: &str,
) -> Result<String, JsError> {
    let rows: Vec<Row> = serde_json::from_str(rows_json)?;
    let cfg: orbit_core::next::Cfg = serde_json::from_str(cfg_json)?;
    Ok(serde_json::to_string(&orbit_core::next::whats_next(
        &rows, today, now_time, &cfg,
    ))?)
}

/// The ids of the items whose bodies link to `id`, over `rows_json` (a Row array); trashed sources left out.
#[wasm_bindgen]
pub fn backlinks(rows_json: &str, id: &str) -> Result<Vec<String>, JsError> {
    let rows: Vec<Row> = serde_json::from_str(rows_json)?;
    let id: orbit_core::Uuid = id.parse().map_err(|_| JsError::new("bad id"))?;
    Ok(orbit_core::link::backlinks(&rows, id)
        .iter()
        .map(|u| u.to_string())
        .collect())
}

/// The reminders to show at `now_local` (`YYYY-MM-DDTHH:MM`) over `rows_json` (a Row array),
/// soonest first, as a JSON array of fires. `cfg_json` is `{ "default_reminder": bool }`.
#[wasm_bindgen]
pub fn reminder_schedule(
    rows_json: &str,
    now_local: &str,
    cfg_json: &str,
) -> Result<String, JsError> {
    let rows: Vec<Row> = serde_json::from_str(rows_json)?;
    let cfg: orbit_core::reminder::Config = serde_json::from_str(cfg_json)?;
    Ok(serde_json::to_string(&orbit_core::reminder::schedule(
        &rows, now_local, &cfg,
    ))?)
}

/// The Placement id for a Roadmap and an item, shared by every Device.
#[wasm_bindgen]
pub fn placement_id(roadmap_id: &str, item_id: &str) -> Result<String, JsError> {
    Ok(orbit_core::roadmap::placement_id(roadmap_id.parse()?, item_id.parse()?).to_string())
}

/// Phases of Roadmap `roadmap_id` over `rows_json` (a Row array), each with its progress, current flag and items.
#[wasm_bindgen]
pub fn roadmap_summary(rows_json: &str, roadmap_id: &str, today: &str) -> Result<String, JsError> {
    let rows: Vec<Row> = serde_json::from_str(rows_json)?;
    let summary = orbit_core::roadmap::summary(&rows, roadmap_id.parse()?, today);
    Ok(serde_json::to_string(&summary)?)
}

/// The search index: one instance lives in JS, fed row by row.
#[wasm_bindgen]
pub struct SearchIndex(orbit_core::search::Index);

#[wasm_bindgen]
impl SearchIndex {
    #[wasm_bindgen(constructor)]
    pub fn new() -> SearchIndex {
        SearchIndex(orbit_core::search::Index::new())
    }

    pub fn upsert(&mut self, row_json: &str) -> Result<(), JsError> {
        let row: Row = serde_json::from_str(row_json)?;
        self.0.upsert(&row);
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<(), JsError> {
        self.0
            .remove(id.parse().map_err(|_| JsError::new("bad id"))?);
        Ok(())
    }

    /// `[{id, kind, title, score}]`, best first. Empty `text` lists recents.
    pub fn query(&self, text: &str, now_ms: f64, limit: usize) -> Result<String, JsError> {
        Ok(serde_json::to_string(&self.0.query(
            text,
            now_ms as u64,
            limit,
        ))?)
    }

    pub fn record_open(&mut self, id: &str, now_ms: f64) -> Result<(), JsError> {
        self.0.record_open(
            id.parse().map_err(|_| JsError::new("bad id"))?,
            now_ms as u64,
        );
        Ok(())
    }

    /// Restores a saved open record for `id`.
    pub fn set_opens(&mut self, id: &str, count: u32, last_ms: f64) -> Result<(), JsError> {
        self.0.set_opens(
            id.parse().map_err(|_| JsError::new("bad id"))?,
            count,
            last_ms as u64,
        );
        Ok(())
    }
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}
