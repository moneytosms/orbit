//! `orbit export --md`: Areas and Projects as headings, tasks as checkbox lines, built from the
//! local replica so it works offline. Trashed rows are left out; subtasks nest two spaces deeper.
use orbit_core::{INBOX, Row, Task, Uuid};
use std::collections::{HashMap, HashSet};

/// Renders the replica as Markdown. Empty when there is nothing to show.
pub fn render(rows: &[Row]) -> String {
    let mut areas = Vec::new();
    let mut projects = Vec::new();
    let mut sections = Vec::new();
    let mut tasks: Vec<&Task> = Vec::new();
    for row in rows {
        match row {
            Row::Area(a) if a.trashed_at.is_none() && a.id != INBOX => areas.push(a),
            Row::Project(p) if p.trashed_at.is_none() => projects.push(p),
            Row::Section(s) if s.trashed_at.is_none() => sections.push(s),
            Row::Task(t) if t.trashed_at.is_none() => tasks.push(t),
            _ => {}
        }
    }
    let mut containers: HashSet<Uuid> = areas.iter().map(|a| a.id).collect();
    containers.extend(projects.iter().map(|p| p.id));
    containers.insert(INBOX);
    let section_parent: HashMap<Uuid, Uuid> =
        sections.iter().map(|s| (s.id, s.parent_id)).collect();
    let task_ids: HashSet<Uuid> = tasks.iter().map(|t| t.id).collect();

    // The container a task's parent resolves to: an Area or Project, directly or through a Section.
    // `None` for a parent that is a Task (a subtask) or nothing live (an orphan, shown in the Inbox).
    let container_of = |parent: Uuid| -> Option<Uuid> {
        if containers.contains(&parent) {
            Some(parent)
        } else {
            section_parent
                .get(&parent)
                .copied()
                .filter(|p| containers.contains(p))
        }
    };

    let mut by_parent: HashMap<Uuid, Vec<&Task>> = HashMap::new();
    for t in &tasks {
        by_parent.entry(t.parent_id).or_default().push(t);
    }
    for kids in by_parent.values_mut() {
        kids.sort_by(|a, b| (a.order.as_str(), a.id).cmp(&(b.order.as_str(), b.id)));
    }
    let mut section_list = sections.clone();
    section_list.sort_by(|a, b| (a.order.as_str(), a.id).cmp(&(b.order.as_str(), b.id)));

    // Tasks directly in a container, then those in its Sections in Section order.
    let container_tasks = |container: Uuid| -> Vec<&Task> {
        let mut out: Vec<&Task> = by_parent.get(&container).cloned().unwrap_or_default();
        for s in section_list.iter().filter(|s| s.parent_id == container) {
            out.extend(by_parent.get(&s.id).into_iter().flatten().copied());
        }
        out
    };

    let mut out: Vec<String> = Vec::new();
    let section = |out: &mut Vec<String>, heading: String, list: Vec<&Task>| {
        if !out.is_empty() {
            out.push(String::new());
        }
        out.push(heading);
        push_tasks(out, &list, &by_parent, 0);
    };

    // Inbox first, only when it has tasks: its own tasks plus any task whose parent is gone.
    let mut inbox = container_tasks(INBOX);
    inbox.extend(
        tasks
            .iter()
            .filter(|t| !task_ids.contains(&t.parent_id) && container_of(t.parent_id).is_none())
            .copied(),
    );
    inbox.sort_by(|a, b| (a.order.as_str(), a.id).cmp(&(b.order.as_str(), b.id)));
    if !inbox.is_empty() {
        section(&mut out, "# Inbox".into(), inbox);
    }

    areas.sort_by(|a, b| (a.order.as_str(), a.id).cmp(&(b.order.as_str(), b.id)));
    projects.sort_by(|a, b| (a.order.as_str(), a.id).cmp(&(b.order.as_str(), b.id)));
    let area_ids: HashSet<Uuid> = areas.iter().map(|a| a.id).collect();
    for area in &areas {
        section(
            &mut out,
            format!("# {}", area.name),
            container_tasks(area.id),
        );
        for project in projects.iter().filter(|p| p.parent_id == area.id) {
            section(
                &mut out,
                format!("## {}", project.name),
                container_tasks(project.id),
            );
        }
    }
    // Projects whose Area is gone still export, under their own heading.
    for project in projects.iter().filter(|p| !area_ids.contains(&p.parent_id)) {
        section(
            &mut out,
            format!("## {}", project.name),
            container_tasks(project.id),
        );
    }

    if out.is_empty() {
        return String::new();
    }
    out.push(String::new());
    out.join("\n")
}

fn push_tasks(
    out: &mut Vec<String>,
    tasks: &[&Task],
    by_parent: &HashMap<Uuid, Vec<&Task>>,
    depth: usize,
) {
    for t in tasks {
        let mark = if t.completed_at.is_some() { "x" } else { " " };
        let due = match &t.due {
            None => String::new(),
            Some(d) => match &d.time {
                Some(time) => format!(" (due {} {time})", d.date),
                None => format!(" (due {})", d.date),
            },
        };
        out.push(format!("{}- [{mark}] {}{due}", "  ".repeat(depth), t.title));
        if let Some(kids) = by_parent.get(&t.id) {
            push_tasks(out, kids, by_parent, depth + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn row(v: Value) -> Row {
        serde_json::from_value(v).unwrap()
    }

    fn id(n: u128) -> String {
        Uuid::from_u128(n).to_string()
    }

    #[test]
    fn empty_replica_renders_nothing() {
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn areas_projects_sections_and_subtasks() {
        let home = id(0x10);
        let garden = id(0x11);
        let shed = id(0x12);
        let rows = vec![
            row(json!({ "kind": "area", "id": home, "name": "Home", "order": "a" })),
            row(
                json!({ "kind": "project", "id": garden, "parent_id": home, "name": "Garden", "order": "a" }),
            ),
            row(
                json!({ "kind": "section", "id": shed, "parent_id": garden, "name": "Shed", "order": "a" }),
            ),
            row(
                json!({ "kind": "task", "id": id(0x20), "parent_id": home, "title": "Fix gate", "order": "a", "completed_at": null }),
            ),
            row(
                json!({ "kind": "task", "id": id(0x21), "parent_id": garden, "title": "Mow", "order": "a",
                "completed_at": null, "due": { "date": "2026-11-26", "time": "09:00" } }),
            ),
            row(
                json!({ "kind": "task", "id": id(0x22), "parent_id": shed, "title": "Oil hinges", "order": "a", "completed_at": 5 }),
            ),
            row(
                json!({ "kind": "task", "id": id(0x23), "parent_id": id(0x22), "title": "Buy oil", "order": "a", "completed_at": null }),
            ),
        ];
        assert_eq!(
            render(&rows),
            "# Home\n- [ ] Fix gate\n\n\
             ## Garden\n- [ ] Mow (due 2026-11-26 09:00)\n- [x] Oil hinges\n  - [ ] Buy oil\n"
        );
    }

    #[test]
    fn inbox_trashed_rows_and_orphans() {
        let rows = vec![
            row(
                json!({ "kind": "task", "id": id(0x30), "parent_id": INBOX.to_string(), "title": "Loose", "order": "a", "completed_at": null }),
            ),
            row(
                json!({ "kind": "task", "id": id(0x31), "parent_id": id(0x99), "title": "Orphan", "order": "b", "completed_at": null }),
            ),
            row(
                json!({ "kind": "task", "id": id(0x32), "parent_id": INBOX.to_string(), "title": "Gone", "order": "c", "completed_at": null, "trashed_at": 7 }),
            ),
            row(
                json!({ "kind": "area", "id": id(0x40), "name": "Work", "order": "a", "trashed_at": 9 }),
            ),
        ];
        assert_eq!(render(&rows), "# Inbox\n- [ ] Loose\n- [ ] Orphan\n");
    }

    #[test]
    fn a_completed_subtask_keeps_its_box_and_due_time_is_optional() {
        let rows = vec![
            row(
                json!({ "kind": "task", "id": id(0x50), "parent_id": INBOX.to_string(), "title": "Pack", "order": "a", "completed_at": null, "due": { "date": "2026-12-01" } }),
            ),
            row(
                json!({ "kind": "task", "id": id(0x51), "parent_id": id(0x50), "title": "Socks", "order": "a", "completed_at": 3 }),
            ),
        ];
        assert_eq!(
            render(&rows),
            "# Inbox\n- [ ] Pack (due 2026-12-01)\n  - [x] Socks\n"
        );
    }
}
