//! Versioned Orbit roadmap graph text format (`%%orbit:v1`): a Mermaid flowchart subset
//! plus `%%orbit` directives that carry Orbit metadata. Parsing is strict about the version.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const NODE_KINDS: [&str; 8] = [
    "task",
    "project",
    "goal",
    "milestone",
    "decision",
    "note",
    "text",
    "heading",
];
const EDGE_KINDS: [&str; 4] = ["then", "blocks", "depends_on", "decides"];

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CodeNode {
    pub id: Option<String>,
    pub kind: String,
    #[serde(default, alias = "ref")]
    pub ref_id: Option<String>,
    #[serde(default)]
    pub title: String,
    pub x: f64,
    pub y: f64,
    /// Key of a band in the same graph.
    #[serde(default)]
    pub band_id: Option<String>,
    /// 1-based source line; set by `parse`, ignored by `export`.
    #[serde(default)]
    pub line: usize,
}
impl PartialEq for CodeNode {
    fn eq(&self, other: &Self) -> bool {
        (
            &self.id,
            &self.kind,
            &self.ref_id,
            &self.title,
            self.x,
            self.y,
            &self.band_id,
        ) == (
            &other.id,
            &other.kind,
            &other.ref_id,
            &other.title,
            other.x,
            other.y,
            &other.band_id,
        )
    }
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CodeEdge {
    pub id: Option<String>,
    pub source: String,
    pub target: String,
    pub kind: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub taken: bool,
    /// 1-based source line; set by `parse`, ignored by `export`.
    #[serde(default)]
    pub line: usize,
}
impl PartialEq for CodeEdge {
    fn eq(&self, other: &Self) -> bool {
        (
            &self.id,
            &self.source,
            &self.target,
            &self.kind,
            &self.label,
            self.taken,
        ) == (
            &other.id,
            &other.source,
            &other.target,
            &other.kind,
            &other.label,
            other.taken,
        )
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CodeBand {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RoadmapCode {
    pub name: Option<String>,
    pub parent: Option<String>,
    pub nodes: BTreeMap<String, CodeNode>,
    pub edges: Vec<CodeEdge>,
    pub bands: BTreeMap<String, CodeBand>,
}
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

/// Directive JSON for a node; the kind comes from the Mermaid class, the title from the label.
#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct NodeMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(
        default,
        rename = "ref",
        alias = "ref_id",
        skip_serializing_if = "Option::is_none"
    )]
    ref_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    band_id: Option<String>,
}

/// Directive JSON for an edge; endpoints come from the directive, the label from the Mermaid arrow.
#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct EdgeMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    taken: bool,
}

struct Declaration {
    key: String,
    class: Option<String>,
    title: String,
    line: usize,
}
struct DirectiveEdge {
    edge: CodeEdge,
    label_set: bool,
    matched: bool,
}

fn err<T>(line: usize, message: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError {
        line,
        message: message.into(),
    })
}
fn is_uuid(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok()
}
fn identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
/// Splits `key {json}` into the head before the first `{` and the JSON object.
fn split_metadata(rest: &str) -> Option<(&str, &str)> {
    let brace = rest.find('{')?;
    Some((rest[..brace].trim(), &rest[brace..]))
}

/// Parses `%%orbit:v1` text. Errors carry the 1-based line they come from.
pub fn parse(text: &str) -> Result<RoadmapCode, ParseError> {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .map(|(i, s)| (i + 1, s.trim()))
        .filter(|(_, s)| !s.is_empty())
        .collect();
    let Some(&(first_line, first)) = lines.first() else {
        return err(1, "Expected `%%orbit:v1` as the first line");
    };
    if first != "%%orbit:v1" {
        return err(first_line, "Expected `%%orbit:v1` as the first line");
    }
    let mut graph = RoadmapCode {
        name: None,
        parent: None,
        nodes: BTreeMap::new(),
        edges: Vec::new(),
        bands: BTreeMap::new(),
    };
    let mut declarations: Vec<Declaration> = Vec::new();
    let mut node_meta: BTreeMap<String, (NodeMeta, usize)> = BTreeMap::new();
    let mut directive_edges: Vec<DirectiveEdge> = Vec::new();
    let mut pending_edges: Vec<(String, String, String, usize)> = Vec::new();

    for &(line_no, line) in lines.iter().skip(1) {
        if line == "%%orbit:v1" {
            return err(line_no, "Duplicate version directive");
        }
        if let Some(rest) = line.strip_prefix("%%orbit roadmap ") {
            let rest = rest.trim();
            if rest.starts_with('{') {
                let meta: serde_json::Value =
                    serde_json::from_str(rest).map_err(|e| ParseError {
                        line: line_no,
                        message: format!("Invalid roadmap metadata: {e}"),
                    })?;
                graph.name = meta.get("name").and_then(|v| v.as_str()).map(str::to_owned);
                graph.parent = meta
                    .get("parent")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned);
                continue;
            }
            let tail = if rest.starts_with('"') {
                let mut stream = serde_json::Deserializer::from_str(rest).into_iter::<String>();
                let name = match stream.next() {
                    Some(Ok(name)) => name,
                    _ => return err(line_no, "Expected the roadmap name in quotes"),
                };
                graph.name = Some(name);
                &rest[stream.byte_offset()..]
            } else {
                rest
            };
            for token in tail.split_whitespace() {
                let Some((key, value)) = token.split_once('=') else {
                    return err(line_no, format!("Expected `key=value`, found `{token}`"));
                };
                // Informational only: an import never reparents the roadmap.
                if key == "parent" {
                    graph.parent = Some(value.to_owned());
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("%%orbit node ") {
            let Some((key, json)) = split_metadata(rest) else {
                return err(line_no, "Expected `%%orbit node <id> {json}`");
            };
            if !identifier(key) || node_meta.contains_key(key) {
                return err(line_no, format!("Invalid or duplicate node id `{key}`"));
            }
            let meta: NodeMeta = serde_json::from_str(json).map_err(|e| ParseError {
                line: line_no,
                message: format!("Invalid node metadata: {e}"),
            })?;
            if meta.id.as_deref().is_some_and(|id| !is_uuid(id)) {
                return err(line_no, "Invalid node id (expected a UUID)");
            }
            if meta
                .kind
                .as_deref()
                .is_some_and(|kind| !NODE_KINDS.contains(&kind))
            {
                return err(
                    line_no,
                    format!("Unknown node kind `{}`", meta.kind.unwrap_or_default()),
                );
            }
            node_meta.insert(key.to_owned(), (meta, line_no));
            continue;
        }
        if let Some(rest) = line.strip_prefix("%%orbit band ") {
            let Some((key, json)) = split_metadata(rest) else {
                return err(line_no, "Expected `%%orbit band <id> {json}`");
            };
            if !identifier(key) || graph.bands.contains_key(key) {
                return err(line_no, format!("Invalid or duplicate band id `{key}`"));
            }
            let band: CodeBand = serde_json::from_str(json).map_err(|e| ParseError {
                line: line_no,
                message: format!("Invalid band metadata: {e}"),
            })?;
            if band.id.as_deref().is_some_and(|id| !is_uuid(id)) {
                return err(line_no, "Invalid band id (expected a UUID)");
            }
            graph.bands.insert(key.to_owned(), band);
            continue;
        }
        if let Some(rest) = line.strip_prefix("%%orbit edge ") {
            let usage = "Expected `%%orbit edge [id] source -> target {json}`";
            let Some((head, json)) = split_metadata(rest) else {
                return err(line_no, usage);
            };
            let (id, source, target) = match head.split_whitespace().collect::<Vec<_>>().as_slice()
            {
                [source, "->", target] => (None, *source, *target),
                [id, source, "->", target] => (Some(*id), *source, *target),
                _ => return err(line_no, usage),
            };
            if id.is_some_and(|id| !is_uuid(id)) {
                return err(line_no, "Invalid edge id (expected a UUID)");
            }
            if !identifier(source) || !identifier(target) {
                return err(line_no, "Edge endpoints must be node ids");
            }
            let meta: EdgeMeta = serde_json::from_str(json).map_err(|e| ParseError {
                line: line_no,
                message: format!("Invalid edge metadata: {e}"),
            })?;
            let kind = meta.kind.unwrap_or_else(|| "then".into());
            if !EDGE_KINDS.contains(&kind.as_str()) {
                return err(line_no, format!("Unknown edge kind `{kind}`"));
            }
            directive_edges.push(DirectiveEdge {
                edge: CodeEdge {
                    id: id.map(str::to_owned),
                    source: source.to_owned(),
                    target: target.to_owned(),
                    kind,
                    label: meta.label.clone().unwrap_or_default(),
                    taken: meta.taken,
                    line: line_no,
                },
                label_set: meta.label.is_some(),
                matched: false,
            });
            continue;
        }
        if line.starts_with("%%") {
            return err(line_no, "Unknown Orbit directive");
        }
        if line.starts_with("flowchart ")
            || line == "flowchart"
            || line.starts_with("graph ")
            || line == "graph"
        {
            continue;
        }
        if let Some((key, class, title)) = declaration(line) {
            if declarations.iter().any(|d| d.key == key) {
                return err(line_no, format!("Duplicate node declaration `{key}`"));
            }
            if class.is_some_and(|class| !NODE_KINDS.contains(&class)) {
                return err(
                    line_no,
                    format!("Unknown node kind `{}`", class.unwrap_or_default()),
                );
            }
            declarations.push(Declaration {
                key: key.to_owned(),
                class: class.map(str::to_owned),
                title,
                line: line_no,
            });
            continue;
        }
        if let Some((source, label, target)) = mermaid_edge(line) {
            pending_edges.push((source, label, target, line_no));
            continue;
        }
        return err(line_no, "Unsupported flowchart statement");
    }

    // An arrow in the diagram labels the first unmatched directive edge with the same endpoints;
    // otherwise it is a new `then` edge.
    for (source, label, target, line_no) in pending_edges {
        match directive_edges
            .iter_mut()
            .find(|d| !d.matched && d.edge.source == source && d.edge.target == target)
        {
            Some(directive) => {
                directive.matched = true;
                if !directive.label_set {
                    directive.edge.label = label;
                }
            }
            None => directive_edges.push(DirectiveEdge {
                edge: CodeEdge {
                    id: None,
                    source,
                    target,
                    kind: "then".into(),
                    label,
                    taken: false,
                    line: line_no,
                },
                label_set: true,
                matched: true,
            }),
        }
    }
    graph.edges = directive_edges.into_iter().map(|d| d.edge).collect();

    for (key, (_, line_no)) in &node_meta {
        if !declarations.iter().any(|d| &d.key == key) {
            return err(
                *line_no,
                format!("Node `{key}` has metadata but no declaration"),
            );
        }
    }
    let mut grid = 0usize;
    for decl in declarations {
        let meta = node_meta
            .remove(&decl.key)
            .map(|(meta, _)| meta)
            .unwrap_or_default();
        let kind = match (&meta.kind, &decl.class) {
            (Some(kind), Some(class)) if kind != class => {
                return err(
                    decl.line,
                    format!("Node kind `{kind}` conflicts with `:::{class}`"),
                );
            }
            (Some(kind), _) => kind.clone(),
            (None, Some(class)) => class.clone(),
            (None, None) => "note".into(),
        };
        if let Some(band) = meta
            .band_id
            .as_ref()
            .filter(|band| !graph.bands.contains_key(*band))
        {
            return err(decl.line, format!("Unknown band `{band}`"));
        }
        let (x, y) = match (meta.x, meta.y) {
            (Some(x), Some(y)) => (x, y),
            (x, y) => {
                let slot = grid;
                grid += 1;
                (
                    x.unwrap_or((slot % 4) as f64 * 240.0),
                    y.unwrap_or((slot / 4) as f64 * 120.0),
                )
            }
        };
        let ref_id = meta.ref_id.filter(|id| is_uuid(id));
        graph.nodes.insert(
            decl.key,
            CodeNode {
                id: meta.id,
                kind,
                ref_id,
                title: decl.title,
                x,
                y,
                band_id: meta.band_id,
                line: decl.line,
            },
        );
    }

    for edge in &graph.edges {
        if !graph.nodes.contains_key(&edge.source) || !graph.nodes.contains_key(&edge.target) {
            return err(edge.line, "Edge references an undeclared node");
        }
    }
    let mut taken_sources = BTreeSet::new();
    for edge in graph.edges.iter().filter(|e| e.taken) {
        if graph.nodes[&edge.source].kind != "decision" {
            return err(edge.line, "Only decision nodes can take a branch");
        }
        if !taken_sources.insert(edge.source.as_str()) {
            return err(edge.line, "Decision already has a taken branch");
        }
    }
    Ok(graph)
}

/// `key["title"]:::class`, `key{"title"}:::class` or `key(title)`, with an optional `:::class`.
fn declaration(s: &str) -> Option<(&str, Option<&str>, String)> {
    let end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    let key = &s[..end];
    if !identifier(key) {
        return None;
    }
    let rest = s[end..].trim_start();
    let close = match rest.chars().next()? {
        '[' => ']',
        '{' => '}',
        '(' => ')',
        _ => return None,
    };
    let inner_end = rest.rfind(close)?;
    let inner = rest[1..inner_end].trim();
    let title = if inner.starts_with('"') {
        serde_json::from_str::<String>(inner).ok()?
    } else {
        inner.to_owned()
    };
    let tail = rest[inner_end + 1..].trim();
    let class = if tail.is_empty() {
        None
    } else {
        Some(tail.strip_prefix(":::").filter(|c| identifier(c))?)
    };
    Some((key, class, title))
}

/// `a --> b` or `a -- label --> b`; the label may be a JSON string, so it can contain `-->`.
fn mermaid_edge(s: &str) -> Option<(String, String, String)> {
    let end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))?;
    let source = &s[..end];
    if !identifier(source) {
        return None;
    }
    let mut rest = s[end..].trim_start();
    let mut label = String::new();
    if !rest.starts_with("-->") {
        rest = rest.strip_prefix("--")?.trim_start();
        if rest.starts_with('"') {
            let mut stream = serde_json::Deserializer::from_str(rest).into_iter::<String>();
            label = stream.next()?.ok()?;
            rest = rest[stream.byte_offset()..].trim_start();
        } else {
            let arrow = rest.find("-->")?;
            label = rest[..arrow].trim().to_owned();
            rest = &rest[arrow..];
        }
    }
    let target = rest.strip_prefix("-->")?.trim();
    if !identifier(target) {
        return None;
    }
    Some((source.to_owned(), label, target.to_owned()))
}

fn json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

/// Canonical serialization. Node and band keys must be identifiers; an id-less edge stays id-less.
pub fn export(graph: &RoadmapCode) -> Result<String, String> {
    let mut out = String::from("%%orbit:v1\nflowchart LR\n");
    if graph.name.is_some() || graph.parent.is_some() {
        out.push_str("%%orbit roadmap");
        if let Some(name) = &graph.name {
            out.push_str(&format!(" {}", json(name)?));
        }
        if let Some(parent) = &graph.parent {
            if parent.is_empty() || parent.contains(char::is_whitespace) {
                return Err("Roadmap parent must not be empty or contain spaces".into());
            }
            out.push_str(&format!(" parent={parent}"));
        }
        out.push('\n');
    }
    for (key, band) in &graph.bands {
        if !identifier(key) {
            return Err(format!("Invalid band key `{key}`"));
        }
        out.push_str(&format!("%%orbit band {key} {}\n", json(band)?));
    }
    for (key, node) in &graph.nodes {
        if !identifier(key) {
            return Err(format!("Invalid node key `{key}`"));
        }
        if !NODE_KINDS.contains(&node.kind.as_str()) {
            return Err(format!("Unknown node kind `{}`", node.kind));
        }
        let meta = NodeMeta {
            id: node.id.clone(),
            kind: None,
            ref_id: node.ref_id.clone(),
            x: Some(node.x),
            y: Some(node.y),
            band_id: node.band_id.clone(),
        };
        out.push_str(&format!("%%orbit node {key} {}\n", json(&meta)?));
        let label = json(&node.title)?;
        let shape = if node.kind == "decision" {
            format!("{{{label}}}")
        } else {
            format!("[{label}]")
        };
        out.push_str(&format!("{key}{shape}:::{}\n", node.kind));
    }
    for edge in &graph.edges {
        if !graph.nodes.contains_key(&edge.source) || !graph.nodes.contains_key(&edge.target) {
            return Err("Edge references an unknown node".into());
        }
        if !EDGE_KINDS.contains(&edge.kind.as_str()) {
            return Err(format!("Unknown edge kind `{}`", edge.kind));
        }
        let id = match &edge.id {
            Some(id) if is_uuid(id) => format!("{id} "),
            Some(id) => return Err(format!("Invalid edge id `{id}`")),
            None => String::new(),
        };
        let meta = EdgeMeta {
            kind: Some(edge.kind.clone()),
            label: None,
            taken: edge.taken,
        };
        out.push_str(&format!(
            "%%orbit edge {id}{} -> {} {}\n",
            edge.source,
            edge.target,
            json(&meta)?
        ));
        let arrow = if edge.label.is_empty() {
            "-->".to_owned()
        } else {
            format!("-- {} -->", json(&edge.label)?)
        };
        out.push_str(&format!("{} {arrow} {}\n", edge.source, edge.target));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC_EXAMPLE: &str = r#"%%orbit:v1
flowchart LR
  %%orbit roadmap "Upskill" parent=area:College
  %%orbit node t1 {"ref":"task","x":120,"y":40}
  %%orbit node d1 {"kind":"decision","x":420,"y":40}
  %%orbit edge d1 -> n1 {"kind":"then","label":"if funded"}
  %%orbit band p1 {"name":"Week 1","start":"2026-11-21","end":"2026-11-27"}

  t1["Read the Rust book"]:::task
  d1{"Do I need a PhD?"}:::decision
  n1["Apply to scholarships"]:::task
  d1 -- yes --> n1
"#;

    fn node(
        id: Option<&str>,
        kind: &str,
        ref_id: Option<&str>,
        title: &str,
        x: f64,
        band_id: Option<&str>,
    ) -> CodeNode {
        CodeNode {
            id: id.map(Into::into),
            kind: kind.into(),
            ref_id: ref_id.map(Into::into),
            title: title.into(),
            x,
            y: 40.0,
            band_id: band_id.map(Into::into),
            line: 0,
        }
    }

    fn edge(
        id: Option<&str>,
        source: &str,
        target: &str,
        kind: &str,
        label: &str,
        taken: bool,
    ) -> CodeEdge {
        CodeEdge {
            id: id.map(Into::into),
            source: source.into(),
            target: target.into(),
            kind: kind.into(),
            label: label.into(),
            taken,
            line: 0,
        }
    }

    #[test]
    fn spec_example_parses() {
        let graph = parse(SPEC_EXAMPLE).unwrap();
        assert_eq!(graph.name.as_deref(), Some("Upskill"));
        assert_eq!(graph.parent.as_deref(), Some("area:College"));
        assert_eq!(graph.nodes["t1"].kind, "task");
        assert_eq!(graph.nodes["t1"].ref_id, None);
        assert_eq!(graph.nodes["n1"].kind, "task");
        assert_eq!(graph.nodes["d1"].kind, "decision");
        assert_eq!(graph.bands["p1"].name, "Week 1");
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].label, "if funded");
        assert_eq!(graph.edges[0].line, 6);
    }

    #[test]
    fn export_parse_roundtrips_nodes_edges_bands_positions_and_references() {
        let band = "3b123a91-b0d1-4edc-a149-01bcaaaccc73";
        let source = RoadmapCode {
            name: Some("Upskill \"2026\"".into()),
            parent: Some("area:College".into()),
            nodes: BTreeMap::from([
                (
                    "n0".into(),
                    node(
                        Some("d12c0a04-f9a3-4b77-9556-0ff2db02a6de"),
                        "task",
                        Some("8b9c91af-0c1e-44ef-b45d-1bb9d5aa3f06"),
                        "Read the Rust book",
                        120.0,
                        Some("p0"),
                    ),
                ),
                (
                    "n1".into(),
                    node(
                        Some("35324c15-eab5-4dd7-8014-8cfb19450c1c"),
                        "decision",
                        None,
                        "Do I need a PhD?",
                        420.0,
                        None,
                    ),
                ),
                (
                    "n2".into(),
                    node(None, "note", None, "Think about it", 600.0, None),
                ),
            ]),
            edges: vec![
                edge(
                    Some("6420b482-6321-4e31-a21f-af1303f1edb1"),
                    "n1",
                    "n0",
                    "decides",
                    "yes",
                    true,
                ),
                edge(None, "n0", "n2", "then", "", false),
                edge(None, "n2", "n0", "blocks", "a --> b -- c | d", false),
                edge(None, "n0", "n2", "depends_on", "first", false),
                edge(None, "n0", "n2", "then", "second", false),
            ],
            bands: BTreeMap::from([(
                "p0".into(),
                CodeBand {
                    id: Some(band.into()),
                    name: "Week 1".into(),
                    start: Some("2026-11-21".into()),
                    end: Some("2026-11-27".into()),
                },
            )]),
        };
        let text = export(&source).unwrap();
        let parsed = parse(&text).unwrap();
        assert_eq!(parsed, source);
        assert_eq!(parsed.edges[1].id, None);
    }

    #[test]
    fn label_with_arrow_roundtrips() {
        let source = RoadmapCode {
            name: None,
            parent: None,
            nodes: BTreeMap::from([
                ("a".into(), node(None, "text", None, "x", 0.0, None)),
                ("b".into(), node(None, "heading", None, "y", 0.0, None)),
            ]),
            edges: vec![edge(None, "a", "b", "then", "--> tricky -- label", false)],
            bands: BTreeMap::new(),
        };
        let text = export(&source).unwrap();
        assert_eq!(parse(&text).unwrap(), source);
    }

    #[test]
    fn strict_version_unknown_fields_and_line_errors() {
        assert_eq!(parse("flowchart LR\n").unwrap_err().line, 1);
        assert_eq!(parse("%%orbit:v2\n").unwrap_err().line, 1);
        let error =
            parse("%%orbit:v1\nflowchart LR\n%%orbit node a {\"kind\":\"alien\",\"x\":0,\"y\":0}")
                .unwrap_err();
        assert_eq!(error.line, 3);
        let error = parse("%%orbit:v1\nflowchart LR\n%%orbit node a {\"kind\":\"note\",\"x\":0,\"y\":0,\"unknown\":true}").unwrap_err();
        assert_eq!(error.line, 3);
    }

    #[test]
    fn errors_point_at_the_bad_line() {
        let text = "%%orbit:v1\nflowchart LR\n%%orbit node a {\"x\":0,\"y\":0}\na[\"ok\"]\n%%orbit node b {\"kind\":\"alien\",\"x\":0,\"y\":0}\nb[\"bad\"]\n";
        assert_eq!(parse(text).unwrap_err().line, 5);
        let text = "%%orbit:v1\nflowchart LR\na[\"ok\"]\nb[\"ok\"]\na => b\n";
        assert_eq!(parse(text).unwrap_err().line, 5);
    }
}
