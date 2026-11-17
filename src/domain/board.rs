//! Resolution of an issue's board (`resolveBoard` from `ui/app.js`).

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

/// Jira boards read from each board's configuration on 25-Sep-2026 (`BOARD_NAMES`).
const BOARD_NAMES: [(i64, &str); 18] = [
    (10, "Tablero Kanban"), (455, "Tablero AFO"), (92, "Tablero DAI"), (14, "Tablero AT"),
    (4, "Tablero BA"), (3, "Tablero PP"), (12, "Tablero IMP"), (59, "Tablero RE"),
    (23, "Tablero ADN"), (16, "Kanban-GPS"), (5, "Tablero GPS"), (29, "TLP-TRACK"),
    (15, "Kanban-TLP"), (9, "TLP-FENIX-OP"), (224, "Tickets Tolepu"), (48, "TLP-FENIX COM"),
    (39, "Tablero CTT Kanban"), (25, "Tablero Integración RPA - Echologistics"),
];
const KANBAN_PROJECTS: [&str; 5] = ["GPS", "APP", "TLP", "CTT", "RPA"];
const COMPONENT_BOARDS: [(&str, &str); 4] = [
    ("Aforo", "Tablero AFO"), ("DAI", "Tablero DAI"),
    ("DAI - Release 2.0", "Tablero DAI"), ("Regímenes Especiales", "Tablero RE"),
];
const PROJECT_BOARDS: [(&str, &str); 1] = [("ADN", "Tablero ADN")];
/// Maximum depth when inheriting the board from the parent.
const MAX_DEPTH: usize = 2;

fn kanban_types_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)^(tarea|task|levantamiento)$").unwrap())
}

fn sprint_cleanup_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)[_ ]*Sprint.*$").unwrap())
}

/// A sprint as `resolveBoard` needs it. `name` and `board_id` hold the text that
/// JS would show in a template (`null` / `undefined`) when they are missing.
#[derive(Debug, Clone, PartialEq)]
pub struct SprintInfo {
    pub id: i64,
    pub end: String,
    pub name: Option<String>,
    pub display_name: String,
    pub board_id: Option<i64>,
    pub display_board_id: String,
}

/// Fields of an issue involved in resolving its board.
#[derive(Debug, Clone, Default)]
pub struct BoardData {
    pub project: Option<String>,
    pub issue_type: Option<String>,
    pub comp: Option<String>,
    pub subtask: bool,
    pub parent: Option<String>,
    pub sprints: Vec<SprintInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoardResolution {
    pub board: String,
    pub how: String,
}

fn find_board<'a>(table: &[(&'a str, &'a str)], key: Option<&str>) -> Option<&'a str> {
    let c = key?;
    table.iter().find(|(k, _)| *k == c).map(|(_, v)| *v)
}

/// The sprint with the greatest `end` (text comparison like `localeCompare`) and, on equal `end`, the greatest `id`.
/// On a full tie the last one wins, like `sort(...).pop()` (JS sort is stable).
fn latest_sprint(sprints: &[SprintInfo]) -> &SprintInfo {
    let mut best = &sprints[0];
    for s in &sprints[1..] {
        let o = super::week::compare_locale(&s.end, &best.end).then(s.id.cmp(&best.id));
        if o != std::cmp::Ordering::Less {
            best = s;
        }
    }
    best
}

/// `resolveBoard(i, byKey, depth)`. `by_key` indexes all issues (the last one with a repeated key wins).
pub fn resolve_board(i: &BoardData, by_key: &HashMap<&str, &BoardData>, depth: usize) -> BoardResolution {
    if !i.sprints.is_empty() {
        let s = latest_sprint(&i.sprints);
        let by_id = s.board_id.and_then(|b| BOARD_NAMES.iter().find(|(k, _)| *k == b).map(|(_, v)| v.to_string()));
        let board = by_id.unwrap_or_else(|| match s.name.as_deref() {
            Some(n) if !n.is_empty() => sprint_cleanup_re().replace(n, "").into_owned(),
            _ => format!("Tablero {}", s.display_board_id),
        });
        return BoardResolution { board, how: format!("sprint «{}»", s.display_name) };
    }
    if let Some(b) = find_board(&PROJECT_BOARDS, i.project.as_deref()) {
        return BoardResolution { board: b.into(), how: format!("proyecto {}", i.project.as_deref().unwrap_or_default()) };
    }
    if i.subtask && depth < MAX_DEPTH {
        if let Some(parent) = i.parent.as_deref().filter(|p| !p.is_empty()) {
            if let Some(data) = by_key.get(parent) {
                let r = resolve_board(data, by_key, depth + 1);
                return BoardResolution { board: r.board, how: format!("tarea padre {parent}") };
            }
        }
    }
    if i.project.as_deref().is_some_and(|p| KANBAN_PROJECTS.contains(&p)) && kanban_types_re().is_match(i.issue_type.as_deref().unwrap_or("")) {
        return BoardResolution { board: "Tablero Kanban".into(), how: "filtro del Tablero Kanban".into() };
    }
    if let Some(b) = find_board(&COMPONENT_BOARDS, i.comp.as_deref()) {
        return BoardResolution { board: b.into(), how: format!("backlog por componente {}", i.comp.as_deref().unwrap_or_default()) };
    }
    BoardResolution { board: "Sin tablero".into(), how: "no coincide con ningún filtro conocido".into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprint(id: i64, end: &str, name: &str, board_id: Option<i64>) -> SprintInfo {
        SprintInfo {
            id,
            end: end.into(),
            name: Some(name.into()),
            display_name: name.into(),
            board_id,
            display_board_id: board_id.map_or("null".into(), |b| b.to_string()),
        }
    }

    fn base() -> BoardData {
        BoardData { project: Some("XYZ".into()), issue_type: Some("Historia".into()), comp: Some("Sin componente".into()), ..Default::default() }
    }

    fn resolve(i: &BoardData) -> BoardResolution {
        resolve_board(i, &HashMap::new(), 0)
    }

    #[test]
    fn sprint_known_by_board_id() {
        let mut i = base();
        i.sprints = vec![sprint(1, "2026-10-01", "AFO Sprint 3", Some(455))];
        let r = resolve(&i);
        assert_eq!(r.board, "Tablero AFO");
        assert_eq!(r.how, "sprint «AFO Sprint 3»");
    }

    #[test]
    fn unknown_sprint_cleans_the_name() {
        let mut i = base();
        i.sprints = vec![sprint(1, "2026-10-01", "Mi equipo_Sprint 12", Some(999))];
        assert_eq!(resolve(&i).board, "Mi equipo");
        i.sprints = vec![sprint(1, "2026-10-01", "Mi equipo  sprint 12 extra", None)];
        assert_eq!(resolve(&i).board, "Mi equipo");
        i.sprints = vec![sprint(1, "2026-10-01", "Sprint 4", Some(999))];
        assert_eq!(resolve(&i).board, ""); // the JS leaves the name empty
        let mut s = sprint(1, "2026-10-01", "", Some(999));
        s.name = Some(String::new());
        i.sprints = vec![s];
        assert_eq!(resolve(&i).board, "Tablero 999");
    }

    #[test]
    fn picks_the_sprint_with_the_latest_end_then_the_highest_id() {
        let mut i = base();
        i.sprints = vec![
            sprint(7, "2026-11-01T00:00:00.000Z", "A", Some(455)),
            sprint(2, "2026-11-20T00:00:00.000Z", "B", Some(92)),
            sprint(9, "2026-10-01T00:00:00.000Z", "C", Some(10)),
        ];
        assert_eq!(resolve(&i).board, "Tablero DAI");
        i.sprints = vec![sprint(5, "2026-11-20", "A", Some(455)), sprint(8, "2026-11-20", "B", Some(92)), sprint(3, "2026-11-20", "C", Some(10))];
        assert_eq!(resolve(&i).board, "Tablero DAI");
        // A sprint without an end loses against one with an end.
        i.sprints = vec![sprint(99, "", "A", Some(455)), sprint(1, "2026-01-01", "B", Some(92))];
        assert_eq!(resolve(&i).board, "Tablero DAI");
    }

    #[test]
    fn kanban_project_component_and_no_board() {
        let mut i = base();
        i.project = Some("ADN".into());
        assert_eq!(resolve(&i), BoardResolution { board: "Tablero ADN".into(), how: "proyecto ADN".into() });
        i.project = Some("GPS".into());
        i.issue_type = Some("TAREA".into());
        assert_eq!(resolve(&i), BoardResolution { board: "Tablero Kanban".into(), how: "filtro del Tablero Kanban".into() });
        i.issue_type = Some("Levantamiento".into());
        assert_eq!(resolve(&i).board, "Tablero Kanban");
        i.issue_type = Some("Historia".into());
        i.comp = Some("DAI - Release 2.0".into());
        assert_eq!(resolve(&i), BoardResolution { board: "Tablero DAI".into(), how: "backlog por componente DAI - Release 2.0".into() });
        i.comp = Some("Otro".into());
        assert_eq!(resolve(&i).board, "Sin tablero");
        i.issue_type = None;
        i.project = Some("GPS".into());
        assert_eq!(resolve(&i).board, "Sin tablero");
    }

    #[test]
    fn subtask_inherits_from_the_parent_with_a_depth_limit() {
        let mut grandparent = base();
        grandparent.project = Some("ADN".into());
        let mut parent = base();
        parent.subtask = true;
        parent.parent = Some("A-1".into());
        let mut child = base();
        child.subtask = true;
        child.parent = Some("P-1".into());
        let mut grandchild = base();
        grandchild.subtask = true;
        grandchild.parent = Some("H-1".into());

        let map: HashMap<&str, &BoardData> = HashMap::from([("A-1", &grandparent), ("P-1", &parent), ("H-1", &child)]);
        // child -> parent -> grandparent (depths 0, 1, 2): the grandparent resolves at depth 2, by project.
        let r = resolve_board(&child, &map, 0);
        assert_eq!(r, BoardResolution { board: "Tablero ADN".into(), how: "tarea padre P-1".into() });
        // grandchild -> child -> parent -> (grandparent is not reached: parent at depth 2 no longer looks at its parent).
        let r = resolve_board(&grandchild, &map, 0);
        assert_eq!(r.board, "Sin tablero");
        assert_eq!(r.how, "tarea padre H-1");
        // A parent that does not exist: falls through to the next rules.
        let r = resolve_board(&child, &HashMap::new(), 0);
        assert_eq!(r.board, "Sin tablero");
        assert_eq!(r.how, "no coincide con ningún filtro conocido");
    }
}
