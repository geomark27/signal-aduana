//! Resolución del tablero de una tarea (`resolveBoard` de `ui/app.js`).

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

/// Tableros de Jira leídos de la configuración de cada tablero el 25-sep-2026 (`BOARD_NAMES`).
const NOMBRES_TABLERO: [(i64, &str); 18] = [
    (10, "Tablero Kanban"), (455, "Tablero AFO"), (92, "Tablero DAI"), (14, "Tablero AT"),
    (4, "Tablero BA"), (3, "Tablero PP"), (12, "Tablero IMP"), (59, "Tablero RE"),
    (23, "Tablero ADN"), (16, "Kanban-GPS"), (5, "Tablero GPS"), (29, "TLP-TRACK"),
    (15, "Kanban-TLP"), (9, "TLP-FENIX-OP"), (224, "Tickets Tolepu"), (48, "TLP-FENIX COM"),
    (39, "Tablero CTT Kanban"), (25, "Tablero Integración RPA - Echologistics"),
];
const PROYECTOS_KANBAN: [&str; 5] = ["GPS", "APP", "TLP", "CTT", "RPA"];
const COMPONENTES: [(&str, &str); 4] = [
    ("Aforo", "Tablero AFO"), ("DAI", "Tablero DAI"),
    ("DAI - Release 2.0", "Tablero DAI"), ("Regímenes Especiales", "Tablero RE"),
];
const PROYECTOS: [(&str, &str); 1] = [("ADN", "Tablero ADN")];
/// Profundidad máxima al heredar el tablero del padre.
const PROFUNDIDAD_MAX: usize = 2;

fn re_tipos_kanban() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)^(tarea|task|levantamiento)$").unwrap())
}

fn re_limpiar_sprint() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)[_ ]*Sprint.*$").unwrap())
}

/// Un sprint tal como lo necesita `resolveBoard`. `name` y `board_id` guardan el texto que
/// JS mostraría en una plantilla (`null` / `undefined`) cuando faltan.
#[derive(Debug, Clone, PartialEq)]
pub struct SprintDato {
    pub id: i64,
    pub end: String,
    pub name: Option<String>,
    pub nombre_visible: String,
    pub board_id: Option<i64>,
    pub board_id_visible: String,
}

/// Campos de una tarea que intervienen en la resolución del tablero.
#[derive(Debug, Clone, Default)]
pub struct DatosTablero {
    pub project: Option<String>,
    pub tipo: Option<String>,
    pub comp: Option<String>,
    pub subtask: bool,
    pub parent: Option<String>,
    pub sprints: Vec<SprintDato>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resolucion {
    pub board: String,
    pub how: String,
}

fn buscar<'a>(tabla: &[(&'a str, &'a str)], clave: Option<&str>) -> Option<&'a str> {
    let c = clave?;
    tabla.iter().find(|(k, _)| *k == c).map(|(_, v)| *v)
}

/// El sprint con mayor `end` (comparación de texto como `localeCompare`) y, a igual `end`, mayor `id`.
/// En empate total gana el último, como `sort(...).pop()` (el orden de JS es estable).
fn ultimo_sprint(sprints: &[SprintDato]) -> &SprintDato {
    let mut mejor = &sprints[0];
    for s in &sprints[1..] {
        let o = super::semana::comparar_locale(&s.end, &mejor.end).then(s.id.cmp(&mejor.id));
        if o != std::cmp::Ordering::Less {
            mejor = s;
        }
    }
    mejor
}

/// `resolveBoard(i, byKey, depth)`. `por_clave` indexa todas las tareas (la última con una clave repetida gana).
pub fn resolver_tablero(i: &DatosTablero, por_clave: &HashMap<&str, &DatosTablero>, profundidad: usize) -> Resolucion {
    if !i.sprints.is_empty() {
        let s = ultimo_sprint(&i.sprints);
        let por_id = s.board_id.and_then(|b| NOMBRES_TABLERO.iter().find(|(k, _)| *k == b).map(|(_, v)| v.to_string()));
        let board = por_id.unwrap_or_else(|| match s.name.as_deref() {
            Some(n) if !n.is_empty() => re_limpiar_sprint().replace(n, "").into_owned(),
            _ => format!("Tablero {}", s.board_id_visible),
        });
        return Resolucion { board, how: format!("sprint «{}»", s.nombre_visible) };
    }
    if let Some(b) = buscar(&PROYECTOS, i.project.as_deref()) {
        return Resolucion { board: b.into(), how: format!("proyecto {}", i.project.as_deref().unwrap_or_default()) };
    }
    if i.subtask && profundidad < PROFUNDIDAD_MAX {
        if let Some(padre) = i.parent.as_deref().filter(|p| !p.is_empty()) {
            if let Some(datos) = por_clave.get(padre) {
                let r = resolver_tablero(datos, por_clave, profundidad + 1);
                return Resolucion { board: r.board, how: format!("tarea padre {padre}") };
            }
        }
    }
    if i.project.as_deref().is_some_and(|p| PROYECTOS_KANBAN.contains(&p)) && re_tipos_kanban().is_match(i.tipo.as_deref().unwrap_or("")) {
        return Resolucion { board: "Tablero Kanban".into(), how: "filtro del Tablero Kanban".into() };
    }
    if let Some(b) = buscar(&COMPONENTES, i.comp.as_deref()) {
        return Resolucion { board: b.into(), how: format!("backlog por componente {}", i.comp.as_deref().unwrap_or_default()) };
    }
    Resolucion { board: "Sin tablero".into(), how: "no coincide con ningún filtro conocido".into() }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn sprint(id: i64, end: &str, name: &str, board_id: Option<i64>) -> SprintDato {
        SprintDato {
            id,
            end: end.into(),
            name: Some(name.into()),
            nombre_visible: name.into(),
            board_id,
            board_id_visible: board_id.map_or("null".into(), |b| b.to_string()),
        }
    }

    fn base() -> DatosTablero {
        DatosTablero { project: Some("XYZ".into()), tipo: Some("Historia".into()), comp: Some("Sin componente".into()), ..Default::default() }
    }

    fn resolver(i: &DatosTablero) -> Resolucion {
        resolver_tablero(i, &HashMap::new(), 0)
    }

    #[test]
    fn sprint_conocido_por_board_id() {
        let mut i = base();
        i.sprints = vec![sprint(1, "2026-10-01", "AFO Sprint 3", Some(455))];
        let r = resolver(&i);
        assert_eq!(r.board, "Tablero AFO");
        assert_eq!(r.how, "sprint «AFO Sprint 3»");
    }

    #[test]
    fn sprint_desconocido_limpia_el_nombre() {
        let mut i = base();
        i.sprints = vec![sprint(1, "2026-10-01", "Mi equipo_Sprint 12", Some(999))];
        assert_eq!(resolver(&i).board, "Mi equipo");
        i.sprints = vec![sprint(1, "2026-10-01", "Mi equipo  sprint 12 extra", None)];
        assert_eq!(resolver(&i).board, "Mi equipo");
        i.sprints = vec![sprint(1, "2026-10-01", "Sprint 4", Some(999))];
        assert_eq!(resolver(&i).board, ""); // el JS deja el nombre vacío
        let mut s = sprint(1, "2026-10-01", "", Some(999));
        s.name = Some(String::new());
        i.sprints = vec![s];
        assert_eq!(resolver(&i).board, "Tablero 999");
    }

    #[test]
    fn elige_el_sprint_de_mayor_fin_y_luego_mayor_id() {
        let mut i = base();
        i.sprints = vec![
            sprint(7, "2026-11-01T00:00:00.000Z", "A", Some(455)),
            sprint(2, "2026-11-20T00:00:00.000Z", "B", Some(92)),
            sprint(9, "2026-10-01T00:00:00.000Z", "C", Some(10)),
        ];
        assert_eq!(resolver(&i).board, "Tablero DAI");
        i.sprints = vec![sprint(5, "2026-11-20", "A", Some(455)), sprint(8, "2026-11-20", "B", Some(92)), sprint(3, "2026-11-20", "C", Some(10))];
        assert_eq!(resolver(&i).board, "Tablero DAI");
        // Sprint sin fin: pierde frente a uno con fin.
        i.sprints = vec![sprint(99, "", "A", Some(455)), sprint(1, "2026-01-01", "B", Some(92))];
        assert_eq!(resolver(&i).board, "Tablero DAI");
    }

    #[test]
    fn proyecto_kanban_componente_y_sin_tablero() {
        let mut i = base();
        i.project = Some("ADN".into());
        assert_eq!(resolver(&i), Resolucion { board: "Tablero ADN".into(), how: "proyecto ADN".into() });
        i.project = Some("GPS".into());
        i.tipo = Some("TAREA".into());
        assert_eq!(resolver(&i), Resolucion { board: "Tablero Kanban".into(), how: "filtro del Tablero Kanban".into() });
        i.tipo = Some("Levantamiento".into());
        assert_eq!(resolver(&i).board, "Tablero Kanban");
        i.tipo = Some("Historia".into());
        i.comp = Some("DAI - Release 2.0".into());
        assert_eq!(resolver(&i), Resolucion { board: "Tablero DAI".into(), how: "backlog por componente DAI - Release 2.0".into() });
        i.comp = Some("Otro".into());
        assert_eq!(resolver(&i).board, "Sin tablero");
        i.tipo = None;
        i.project = Some("GPS".into());
        assert_eq!(resolver(&i).board, "Sin tablero");
    }

    #[test]
    fn subtarea_hereda_del_padre_con_limite_de_profundidad() {
        let mut abuelo = base();
        abuelo.project = Some("ADN".into());
        let mut padre = base();
        padre.subtask = true;
        padre.parent = Some("A-1".into());
        let mut hijo = base();
        hijo.subtask = true;
        hijo.parent = Some("P-1".into());
        let mut nieto = base();
        nieto.subtask = true;
        nieto.parent = Some("H-1".into());

        let mapa: HashMap<&str, &DatosTablero> = HashMap::from([("A-1", &abuelo), ("P-1", &padre), ("H-1", &hijo)]);
        // hijo -> padre -> abuelo (profundidades 0, 1, 2): el abuelo se resuelve en profundidad 2, por proyecto.
        let r = resolver_tablero(&hijo, &mapa, 0);
        assert_eq!(r, Resolucion { board: "Tablero ADN".into(), how: "tarea padre P-1".into() });
        // nieto -> hijo -> padre -> (abuelo no se alcanza: padre en profundidad 2 ya no mira a su padre).
        let r = resolver_tablero(&nieto, &mapa, 0);
        assert_eq!(r.board, "Sin tablero");
        assert_eq!(r.how, "tarea padre H-1");
        // Padre que no existe: cae a las reglas siguientes.
        let r = resolver_tablero(&hijo, &HashMap::new(), 0);
        assert_eq!(r.board, "Sin tablero");
        assert_eq!(r.how, "no coincide con ningún filtro conocido");
    }
}
