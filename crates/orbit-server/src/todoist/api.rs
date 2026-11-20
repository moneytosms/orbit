//! Todoist API v1 reads for the import: paginated lists, 429 backoff, attachment downloads.
//! The token lives only in this module's `Todoist` for the length of one fetch.
use crate::blobs;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path, time::Duration};

pub const BASE: &str = "https://api.todoist.com/api/v1";
/// 429 retries per request before giving up.
const RETRIES: u32 = 5;
const MAX_WAIT_S: u64 = 60;

#[derive(Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub child_order: i64,
    #[serde(default)]
    pub inbox_project: bool,
}

#[derive(Deserialize, Clone)]
pub struct Section {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(default)]
    pub section_order: i64,
}

#[derive(Deserialize, Clone)]
pub struct Due {
    pub date: String,
    #[serde(default)]
    pub string: Option<String>,
    #[serde(default)]
    pub is_recurring: bool,
}

#[derive(Deserialize, Clone)]
pub struct Deadline {
    pub date: String,
}

#[derive(Deserialize, Clone)]
pub struct Length {
    pub amount: u32,
    pub unit: String,
}

#[derive(Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    #[serde(default)]
    pub section_id: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub content: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub labels: Vec<String>,
    /// API 4 is the app's p1.
    #[serde(default = "one")]
    pub priority: u8,
    #[serde(default)]
    pub due: Option<Due>,
    #[serde(default)]
    pub deadline: Option<Deadline>,
    #[serde(default)]
    pub duration: Option<Length>,
    #[serde(default)]
    pub child_order: i64,
    #[serde(default)]
    pub checked: bool,
}

fn one() -> u8 {
    1
}

#[derive(Deserialize, Clone)]
pub struct Reminder {
    pub item_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub minute_offset: Option<u32>,
    #[serde(default)]
    pub due: Option<Due>,
    #[serde(default)]
    pub is_deleted: bool,
}

#[derive(Deserialize)]
struct RawComment {
    content: String,
    #[serde(default)]
    posted_at: String,
    #[serde(default)]
    file_attachment: Option<RawFile>,
    #[serde(default)]
    is_deleted: bool,
}

#[derive(Deserialize)]
struct RawFile {
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    file_type: String,
    #[serde(default)]
    file_url: String,
}

pub struct File {
    pub name: String,
    pub image: bool,
    pub url: String,
    /// The blob hash once downloaded; `None` keeps the URL.
    pub blob: Option<String>,
}

pub struct Comment {
    /// `YYYY-MM-DD`.
    pub date: String,
    pub content: String,
    pub file: Option<File>,
}

#[derive(Default)]
pub struct Snapshot {
    pub projects: Vec<Project>,
    pub sections: Vec<Section>,
    pub tasks: Vec<Task>,
    pub labels: Vec<String>,
    pub reminders: Vec<Reminder>,
    /// By task id.
    pub task_comments: HashMap<String, Vec<Comment>>,
    /// By project id.
    pub project_comments: HashMap<String, Vec<Comment>>,
}

struct Todoist<'a> {
    http: reqwest::Client,
    base: &'a str,
    token: &'a str,
}

impl Todoist<'_> {
    /// GET with the token, waiting out 429s (the `Retry-After` header, else 1, 2, 4... seconds).
    async fn get(
        &self,
        url: &str,
        query: &[(&str, &str)],
        auth: bool,
    ) -> Result<reqwest::Response, String> {
        for attempt in 0..=RETRIES {
            let mut req = self.http.get(url).query(query);
            if auth {
                req = req.bearer_auth(self.token);
            }
            let res = req
                .send()
                .await
                .map_err(|e| format!("network: {}", e.without_url()))?;
            if res.status() == 429 && attempt < RETRIES {
                let wait = res
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1u64 << attempt)
                    .min(MAX_WAIT_S);
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            return match res.status().as_u16() {
                200..=299 => Ok(res),
                401 | 403 => Err("Todoist rejected the token".into()),
                429 => Err("Todoist is rate limiting this token, try again later".into()),
                s => Err(format!("Todoist answered HTTP {s}")),
            };
        }
        unreachable!("the last attempt returns")
    }

    /// Every item of a paginated list endpoint.
    async fn list<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        extra: &[(&str, &str)],
    ) -> Result<Vec<T>, String> {
        let mut out = Vec::new();
        let mut cursor = String::new();
        loop {
            let mut query = vec![("limit", "200")];
            query.extend_from_slice(extra);
            if !cursor.is_empty() {
                query.push(("cursor", &cursor));
            }
            let page: Value = self
                .get(&format!("{}{path}", self.base), &query, true)
                .await?
                .json()
                .await
                .map_err(|e| format!("{path}: bad JSON: {}", e.without_url()))?;
            let (items, next) = match page {
                Value::Array(a) => (a, None),
                mut v => (
                    v["results"].take().as_array().cloned().unwrap_or_default(),
                    v["next_cursor"].as_str().map(String::from),
                ),
            };
            for item in items {
                out.push(
                    serde_json::from_value(item)
                        .map_err(|e| format!("{path}: unexpected shape: {e}"))?,
                );
            }
            match next {
                Some(n) if !n.is_empty() => cursor = n,
                _ => return Ok(out),
            }
        }
    }

    /// Downloads an attachment into the blob store. The token only goes to Todoist's own hosts.
    async fn download(&self, f: &RawFile, data: &Path, max: usize) -> Option<String> {
        let origin = self
            .base
            .splitn(4, '/')
            .take(3)
            .collect::<Vec<_>>()
            .join("/");
        let host = f.file_url.split('/').nth(2).unwrap_or("");
        let trusted = f.file_url.starts_with(&origin) || host.ends_with(".todoist.com");
        let bytes = self
            .get(&f.file_url, &[], trusted)
            .await
            .ok()?
            .bytes()
            .await
            .ok()?;
        if bytes.len() > max {
            return None;
        }
        let hash = hex::encode(Sha256::digest(&bytes));
        blobs::store(data, &hash, &bytes).ok()?;
        Some(hash)
    }

    async fn comments(
        &self,
        key: &str,
        id: &str,
        data: &Path,
        max: usize,
    ) -> Result<Vec<Comment>, String> {
        let raw: Vec<RawComment> = self.list("/comments", &[(key, id)]).await?;
        let mut out = Vec::new();
        for c in raw.into_iter().filter(|c| !c.is_deleted) {
            let file = match c.file_attachment.filter(|f| !f.file_url.is_empty()) {
                Some(f) => Some(File {
                    blob: self.download(&f, data, max).await,
                    image: f.file_type.starts_with("image/"),
                    name: if f.file_name.is_empty() {
                        "attachment".into()
                    } else {
                        f.file_name
                    },
                    url: f.file_url,
                }),
                None => None,
            };
            if file.is_some() || !c.content.is_empty() {
                out.push(Comment {
                    date: c.posted_at.chars().take(10).collect(),
                    content: c.content,
                    file,
                });
            }
        }
        Ok(out)
    }
}

/// Reads every active project, section, task, label, reminder and comment, downloading attachments
/// into the blob store. Fails with a message that never contains the token.
pub async fn fetch(
    base: &str,
    token: &str,
    data: &Path,
    max_blob: usize,
) -> Result<Snapshot, String> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let t = Todoist {
        http,
        base: base.trim_end_matches('/'),
        token,
    };
    let mut s = Snapshot {
        projects: t.list("/projects", &[]).await?,
        sections: t.list("/sections", &[]).await?,
        tasks: t
            .list::<Task>("/tasks", &[])
            .await?
            .into_iter()
            .filter(|t| !t.checked)
            .collect(),
        labels: t
            .list::<Label>("/labels", &[])
            .await?
            .into_iter()
            .map(|l| l.name)
            .collect(),
        // Plan-dependent: a plan without reminders may answer with an error, which is not fatal.
        reminders: t
            .list::<Reminder>("/reminders", &[])
            .await
            .unwrap_or_default(),
        ..Default::default()
    };
    s.reminders.retain(|r| !r.is_deleted);
    for p in &s.projects {
        let c = t.comments("project_id", &p.id, data, max_blob).await?;
        if !c.is_empty() {
            s.project_comments.insert(p.id.clone(), c);
        }
    }
    for task in &s.tasks {
        let c = t.comments("task_id", &task.id, data, max_blob).await?;
        if !c.is_empty() {
            s.task_comments.insert(task.id.clone(), c);
        }
    }
    Ok(s)
}

#[derive(Deserialize)]
struct Label {
    name: String,
}
