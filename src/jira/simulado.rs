//! Fuente de datos ficticia para probar la aplicación sin conectarse a Jira.
//!
//! Genera, de forma determinista y relativa a `hoy`, unas 18 tareas con la misma forma JSON que la
//! API REST de Jira. Así el tablero siempre se ve «vivo» y además se ejercita `Tarea::desde_jira`.
//! Cubre los casos del tablero: cada tipo de tablero, cada estado y cada señal (vencida, inicio
//! pasado, concluida tarde, sin «Fin desarrollo», estimación superada, fin de semana, días con
//! pocas horas, documento de despliegue presente, ausente o solo como enlace remoto).
//!
//! Todo es inventado: personas, empresas, tareas y sitio (`ejemplo.atlassian.net`).

use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::Duration;

use chrono::{Datelike, Duration as Dias, NaiveDate, Utc, Weekday};
use regex::Regex;
use serde_json::{json, Map, Value};

use super::modelo::Usuario;
use super::{validar_clave, EnlaceRemoto, PaginaBusqueda, PaginaWiki};
use crate::error::{ErrorApp, Resultado};

/// Misma lista que `FERIADOS_BASE` de `ui/app.js`: en esos días el simulado no registra horas.
pub const FERIADOS_BASE: [&str; 12] = [
    "2026-01-01",
    "2026-02-16",
    "2026-02-17",
    "2026-04-03",
    "2026-05-01",
    "2026-05-25",
    "2026-08-10",
    "2026-10-09",
    "2026-11-02",
    "2026-11-03",
    "2026-12-25",
    "2027-01-01",
];

const ESPERA: Duration = Duration::from_millis(120);
const TAM_PAGINA: usize = 10;
const MAX_WORKLOGS_EN_TAREA: usize = 20;
const YO: &str = "sim-0001";
const OTRA: &str = "sim-0002";
const SITIO: &str = "https://ejemplo.atlassian.net";
/// Semanas de historial de horas (14 semanas completas hacia atrás desde el lunes actual).
const DIAS_HISTORIAL: i64 = 98;

/// Fecha de hoy en Ecuador (UTC-5, sin horario de verano).
pub fn hoy_ecuador() -> NaiveDate {
    (Utc::now() - Dias::hours(5)).date_naive()
}

fn es_feriado(d: NaiveDate) -> bool {
    let s = d.format("%Y-%m-%d").to_string();
    FERIADOS_BASE.contains(&s.as_str())
}

fn es_laborable(d: NaiveDate) -> bool {
    !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) && !es_feriado(d)
}

// ---------------------------------------------------------------------------------------------
// Definición de las tareas (fechas como días de diferencia respecto al lunes de la semana actual)
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
enum Estado {
    #[default]
    PorHacer,
    EnCurso,
    EnPruebas,
    Finalizada,
}

impl Estado {
    fn nombre_y_categoria(self) -> (&'static str, &'static str) {
        match self {
            Estado::PorHacer => ("Por hacer", "new"),
            Estado::EnCurso => ("En curso", "indeterminate"),
            Estado::EnPruebas => ("En pruebas", "indeterminate"),
            Estado::Finalizada => ("Finalizada", "done"),
        }
    }
}

#[derive(Clone, Copy, Default)]
enum Estimacion {
    #[default]
    Ninguna,
    /// Horas fijas.
    Horas(i64),
    /// Múltiplo de lo realmente gastado (menor que 1 simula una estimación superada).
    Factor(f64),
}

#[derive(Clone, Copy, Default, PartialEq)]
enum Documento {
    #[default]
    Ninguno,
    Confluence,
    SoloEnlace,
}

#[derive(Clone, Copy)]
struct DefSprint {
    id: i64,
    nombre: &'static str,
    tablero: i64,
    estado: &'static str,
    /// Día de cierre (diferencia con el lunes actual).
    fin: i64,
}

#[derive(Default)]
struct Def {
    clave: &'static str,
    resumen: &'static str,
    estado: Estado,
    componente: Option<&'static str>,
    sprints: &'static [DefSprint],
    /// `Some(clave)` marca una subtarea de esa tarea.
    subtarea_de: Option<&'static str>,
    prioridad: &'static str,
    creada: i64,
    vence: Option<i64>,
    inicio: Option<i64>,
    fin_dev: Option<i64>,
    /// Resolución cuando no hay `fin_dev` (caso «concluida sin Fin desarrollo»).
    resuelta: Option<i64>,
    /// Días (inclusive) en los que se registran horas propias; `None` si la tarea no tiene horas.
    ventana: Option<(i64, i64)>,
    estimacion: Estimacion,
    puntos: Option<f64>,
    etiquetas: &'static [&'static str],
    pr: bool,
    doc: Documento,
}

const SPRINT_AFO_22: DefSprint = DefSprint { id: 701, nombre: "AFO Sprint 22", tablero: 455, estado: "closed", fin: -10 };
const SPRINT_AFO_23: DefSprint = DefSprint { id: 702, nombre: "AFO Sprint 23", tablero: 455, estado: "active", fin: 4 };
const SPRINT_DAI_15: DefSprint = DefSprint { id: 801, nombre: "DAI Sprint 15", tablero: 92, estado: "closed", fin: -3 };

const AFO_101_SPRINTS: &[DefSprint] = &[SPRINT_AFO_23];
const AFO_102_SPRINTS: &[DefSprint] = &[SPRINT_AFO_22, SPRINT_AFO_23];
const DAI_SPRINTS: &[DefSprint] = &[SPRINT_DAI_15];

fn definiciones() -> Vec<Def> {
    vec![
        // --- Tablero por sprint: AFO (455) ---
        Def {
            clave: "AFO-101",
            resumen: "Ajuste del cálculo de tributos en la liquidación de aforo",
            estado: Estado::EnCurso,
            sprints: AFO_101_SPRINTS,
            prioridad: "Alta",
            creada: -40,
            vence: Some(6),
            inicio: Some(-35),
            ventana: Some((-35, 4)),
            estimacion: Estimacion::Factor(0.6), // supera la estimación en más de 20 %
            puntos: Some(8.0),
            etiquetas: &["aforo", "tributos"],
            pr: true,
            doc: Documento::Confluence,
            ..Def::default()
        },
        Def {
            clave: "AFO-102",
            resumen: "Validación de partidas arancelarias en la declaración aduanera",
            estado: Estado::EnPruebas,
            sprints: AFO_102_SPRINTS,
            prioridad: "Media",
            creada: -30,
            vence: Some(8),
            inicio: Some(-21),
            fin_dev: Some(-1),
            ventana: Some((-21, 4)),
            estimacion: Estimacion::Factor(1.1),
            puntos: Some(5.0),
            pr: true,
            doc: Documento::Confluence,
            ..Def::default()
        },
        // Subtarea: el tablero se resuelve por la tarea padre.
        Def {
            clave: "AFO-103",
            resumen: "Pruebas unitarias del cálculo de tributos",
            estado: Estado::EnCurso,
            subtarea_de: Some("AFO-101"),
            prioridad: "Media",
            creada: -12,
            vence: Some(5),
            inicio: Some(-10),
            ventana: Some((-10, 4)),
            estimacion: Estimacion::Factor(1.15),
            ..Def::default()
        },
        // --- Tablero por sprint: DAI (92) ---
        Def {
            clave: "DAI-201",
            resumen: "Integración del manifiesto de carga con el sistema aduanero",
            estado: Estado::Finalizada,
            sprints: DAI_SPRINTS,
            prioridad: "Alta",
            creada: -55,
            vence: Some(-6),
            inicio: Some(-49),
            fin_dev: Some(-4), // concluida 2 días después del vencimiento
            ventana: Some((-49, -4)),
            estimacion: Estimacion::Factor(1.1),
            puntos: Some(13.0),
            pr: true,
            doc: Documento::Confluence,
            ..Def::default()
        },
        Def {
            clave: "DAI-202",
            resumen: "Reporte de declaraciones pendientes por distrito aduanero",
            estado: Estado::Finalizada,
            sprints: DAI_SPRINTS,
            prioridad: "Media",
            creada: -48,
            vence: Some(-8),
            inicio: Some(-42),
            resuelta: Some(-9), // finalizada SIN Fecha Fin Desarrollo
            ventana: Some((-42, -10)),
            estimacion: Estimacion::Factor(1.1),
            pr: true,
            doc: Documento::Confluence,
            ..Def::default()
        },
        // --- Tablero por proyecto: ADN ---
        Def {
            clave: "ADN-301",
            resumen: "Migración de la plantilla de notificaciones a clientes",
            estado: Estado::PorHacer, // su fecha de inicio ya pasó y sigue «Por hacer»
            prioridad: "Baja",
            creada: -14,
            vence: Some(10),
            inicio: Some(-5),
            estimacion: Estimacion::Horas(8),
            ..Def::default()
        },
        Def {
            clave: "ADN-302",
            resumen: "Conciliación automática de guías de carga",
            estado: Estado::EnCurso,
            prioridad: "Alta",
            creada: -35,
            vence: Some(-3), // abierta y vencida
            inicio: Some(-28),
            ventana: Some((-28, 4)),
            estimacion: Estimacion::Factor(1.1),
            pr: true,
            doc: Documento::Confluence,
            ..Def::default()
        },
        // --- Filtro del Tablero Kanban (proyectos APP/GPS, tipo Tarea) ---
        Def {
            clave: "APP-401",
            resumen: "Automatización del envío de documentos a bodega",
            estado: Estado::EnCurso,
            prioridad: "Media",
            creada: -20,
            vence: Some(12),
            inicio: Some(-14),
            ventana: Some((-14, 4)),
            estimacion: Estimacion::Factor(1.2),
            pr: true,
            doc: Documento::Confluence,
            ..Def::default()
        },
        // El documento de despliegue solo existe como enlace remoto.
        Def {
            clave: "APP-402",
            resumen: "Notificación automática de arribo de contenedores",
            estado: Estado::Finalizada,
            prioridad: "Media",
            creada: -70,
            vence: Some(-4),
            inicio: Some(-63),
            fin_dev: Some(-5),
            ventana: Some((-63, -5)),
            estimacion: Estimacion::Factor(1.1),
            pr: true,
            doc: Documento::SoloEnlace,
            ..Def::default()
        },
        // Concluida hace poco, con PR y SIN documento de despliegue.
        Def {
            clave: "APP-403",
            resumen: "Cálculo de recargos por almacenaje en patio",
            estado: Estado::Finalizada,
            prioridad: "Alta",
            creada: -32,
            vence: Some(-3),
            inicio: Some(-28),
            fin_dev: Some(-3),
            ventana: Some((-28, -3)),
            estimacion: Estimacion::Factor(1.1),
            pr: true,
            doc: Documento::Ninguno,
            ..Def::default()
        },
        Def {
            clave: "APP-404",
            resumen: "Conciliación de facturas de transporte terrestre",
            estado: Estado::PorHacer, // inicia hoy
            prioridad: "Baja",
            creada: -5,
            vence: Some(15),
            inicio: Some(i64::MIN), // marcador: se resuelve a «hoy» al generar
            estimacion: Estimacion::Horas(6),
            ..Def::default()
        },
        Def {
            clave: "GPS-501",
            resumen: "Alerta de vencimiento de garantías de transporte",
            estado: Estado::Finalizada,
            prioridad: "Media",
            creada: -85,
            vence: Some(-7),
            inicio: Some(-77),
            fin_dev: Some(-6),
            ventana: Some((-77, -6)),
            estimacion: Estimacion::Factor(1.1),
            ..Def::default()
        },
        // --- Backlog por componente ---
        Def {
            clave: "OPE-601",
            resumen: "Revisión del checklist de inspección de contenedores",
            estado: Estado::EnCurso,
            componente: Some("Aforo"),
            prioridad: "Media",
            creada: -60,
            vence: Some(9),
            inicio: Some(-56),
            ventana: Some((-56, 4)),
            estimacion: Estimacion::Factor(1.1),
            ..Def::default()
        },
        Def {
            clave: "OPE-602",
            resumen: "Parametrización de reglas para el régimen de consumo",
            estado: Estado::EnCurso,
            componente: Some("DAI"),
            prioridad: "Media",
            creada: -33,
            vence: Some(11),
            inicio: Some(-30),
            ventana: Some((-30, 4)),
            estimacion: Estimacion::Factor(1.1),
            ..Def::default()
        },
        Def {
            clave: "OPE-603",
            resumen: "Control de saldos de depósito temporal",
            estado: Estado::Finalizada,
            componente: Some("Regímenes Especiales"),
            prioridad: "Baja",
            creada: -95,
            vence: Some(-30),
            inicio: Some(-91),
            fin_dev: Some(-31),
            ventana: Some((-91, -31)),
            estimacion: Estimacion::Factor(1.1),
            ..Def::default()
        },
        // --- Reunión, soporte y capacitación (sin PR, sin tablero) ---
        Def {
            clave: "OPE-701",
            resumen: "Reunión semanal de seguimiento de operaciones",
            estado: Estado::EnCurso,
            prioridad: "Baja",
            creada: -105,
            ventana: Some((-105, 4)),
            ..Def::default()
        },
        Def {
            clave: "OPE-702",
            resumen: "Soporte a usuarios de la plataforma de operaciones",
            estado: Estado::EnCurso,
            prioridad: "Media",
            creada: -105,
            ventana: Some((-105, 4)),
            estimacion: Estimacion::Factor(1.3),
            ..Def::default()
        },
        Def {
            clave: "OPE-703",
            resumen: "Capacitación en el nuevo módulo de regímenes especiales",
            estado: Estado::Finalizada,
            prioridad: "Baja",
            creada: -50,
            vence: Some(-34),
            inicio: Some(-45),
            fin_dev: Some(-35),
            ventana: Some((-45, -35)),
            estimacion: Estimacion::Factor(1.1),
            ..Def::default()
        },
    ]
}

// ---------------------------------------------------------------------------------------------
// Registros de horas
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct Reg {
    id: u64,
    autor: &'static str,
    dia: NaiveDate,
    hora: u32,
    segundos: i64,
}

impl Reg {
    /// `2026-11-10T08:30:00.000-0500`
    fn inicio_jira(&self) -> String {
        format!("{}T{:02}:30:00.000-0500", self.dia.format("%Y-%m-%d"), self.hora)
    }

    fn inicio_ms(&self) -> i64 {
        let local = self.dia.and_hms_opt(self.hora, 30, 0).expect("hora válida");
        local.and_utc().timestamp_millis() + 5 * 3_600_000
    }

    fn a_json(&self) -> Value {
        json!({
            "id": self.id.to_string(),
            "author": { "accountId": self.autor, "displayName": if self.autor == YO { "Persona de Prueba" } else { "Otra Persona" } },
            "started": self.inicio_jira(),
            "timeSpentSeconds": self.segundos,
            "timeSpent": texto_duracion(self.segundos),
        })
    }
}

fn texto_duracion(seg: i64) -> String {
    let (h, m) = (seg / 3600, (seg % 3600) / 60);
    match (h, m) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// Reparte las horas propias de cada día laborable entre 1 o 2 tareas activas.
fn generar_registros(defs: &[Def], lunes: NaiveDate, hoy: NaiveDate) -> Vec<Vec<Reg>> {
    let mut por_tarea: Vec<Vec<Reg>> = vec![Vec::new(); defs.len()];
    let mut siguiente_id: u64 = 100_000;
    let mut nuevo = |autor, dia, hora, segundos| {
        siguiente_id += 1;
        Reg { id: siguiente_id, autor, dia, hora, segundos }
    };

    // En la semana actual: el primer día laborable ya pasado queda con 0 h y el segundo con 4 h.
    let pasados_semana: Vec<NaiveDate> =
        (0..5).map(|n| lunes + Dias::days(n)).filter(|d| *d < hoy && es_laborable(*d)).collect();

    let mut dia = lunes - Dias::days(DIAS_HISTORIAL);
    while dia < hoy {
        if es_laborable(dia) {
            let total: i64 = match pasados_semana.iter().position(|d| *d == dia) {
                Some(0) => 0,
                Some(1) => 4 * 3600,
                _ => 8 * 3600,
            };
            let desfase = (dia - lunes).num_days();
            let activas: Vec<usize> = defs
                .iter()
                .enumerate()
                .filter(|(_, d)| d.ventana.is_some_and(|(a, b)| desfase >= a && desfase <= b))
                .map(|(i, _)| i)
                .collect();
            if total > 0 && !activas.is_empty() {
                let orden = dia.num_days_from_ce() as usize;
                let n = activas.len();
                // Se elige la tarea activa con menos registros hasta el momento (así todas reciben horas).
                let menos_cargada = |excluir: Option<usize>| {
                    activas.iter().copied().filter(|i| Some(*i) != excluir).min_by_key(|i| (por_tarea[*i].len(), *i))
                };
                let a = menos_cargada(None).expect("hay tareas activas");
                if n == 1 || orden % 3 == 0 {
                    por_tarea[a].push(nuevo(YO, dia, 8, total));
                } else {
                    // El soporte recurrente es siempre el segundo reparto: acumula más de 20 registros.
                    let soporte = activas.iter().copied().find(|i| defs[*i].clave == "OPE-702");
                    let b = soporte.filter(|i| *i != a).or_else(|| menos_cargada(Some(a))).unwrap_or(a);
                    let mayor = total * 5 / 8;
                    por_tarea[a].push(nuevo(YO, dia, 8, mayor));
                    por_tarea[b].push(nuevo(YO, dia, 13, total - mayor));
                }
            }
        }
        dia += Dias::days(1);
    }

    let indice = |clave: &str| defs.iter().position(|d| d.clave == clave).expect("tarea definida");
    // Horas en sábado (cae en la semana anterior): fuera de la semana laboral.
    por_tarea[indice("AFO-101")].push(nuevo(YO, lunes - Dias::days(2), 9, 3 * 3600));
    // Horas de otra persona en una tarea compartida: la interfaz debe ignorarlas.
    for desfase in [-6, -5] {
        let d = lunes + Dias::days(desfase);
        por_tarea[indice("APP-401")].push(nuevo(OTRA, d, 10, 2 * 3600));
    }
    for regs in &mut por_tarea {
        regs.sort_by_key(|r| (r.dia, r.hora, r.id));
    }
    por_tarea
}

// ---------------------------------------------------------------------------------------------
// Fuente simulada
// ---------------------------------------------------------------------------------------------

struct TareaSim {
    clave: &'static str,
    resumen: &'static str,
    /// Campos de Jira, salvo `worklog` (se arma al responder para respetar el tope de 20).
    campos: Map<String, Value>,
    registros: Vec<Reg>,
    pr: bool,
    doc: Documento,
}

impl TareaSim {
    fn issue(&self, campos: &[&str]) -> Value {
        let mut out = Map::new();
        for c in campos {
            let valor = if *c == "worklog" {
                let primeros: Vec<Value> =
                    self.registros.iter().take(MAX_WORKLOGS_EN_TAREA).map(Reg::a_json).collect();
                json!({
                    "startAt": 0,
                    "maxResults": MAX_WORKLOGS_EN_TAREA,
                    "total": self.registros.len(),
                    "worklogs": primeros,
                })
            } else {
                self.campos.get(*c).cloned().unwrap_or(Value::Null)
            };
            out.insert((*c).to_string(), valor);
        }
        let id = 10_000 + self.clave.rsplit('-').next().and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
        json!({ "id": id.to_string(), "key": self.clave, "fields": out })
    }

    fn tiene_horas_propias_desde(&self, desde: Option<NaiveDate>) -> bool {
        self.registros.iter().any(|r| r.autor == YO && desde.map_or(true, |d| r.dia >= d))
    }

    fn pagina_wiki(&self, posicion: usize) -> PaginaWiki {
        PaginaWiki {
            titulo: format!("Documento de Despliegue - {} - {}", self.clave, self.resumen),
            url: format!("{SITIO}/wiki/spaces/DEV/pages/{}/Documento+de+Despliegue", 9_000 + posicion),
        }
    }
}

pub struct JiraSimulado {
    correo: String,
    token_valido: bool,
    tareas: Vec<TareaSim>,
}

fn nombre_proyecto(prefijo: &str) -> &'static str {
    match prefijo {
        "AFO" => "Aforo Digital",
        "DAI" => "Declaración Aduanera",
        "ADN" => "Analítica y Datos",
        "APP" => "Aplicaciones Internas",
        "GPS" => "Rastreo de Flota",
        _ => "Operaciones",
    }
}

fn fecha_hora(d: NaiveDate, hora: u32) -> String {
    format!("{}T{:02}:00:00.000-0500", d.format("%Y-%m-%d"), hora)
}

impl JiraSimulado {
    pub fn nuevo(correo: &str, token_valido: bool, hoy: NaiveDate) -> Self {
        let lunes = hoy - Dias::days(i64::from(hoy.weekday().num_days_from_monday()));
        let defs = definiciones();
        let registros = generar_registros(&defs, lunes, hoy);
        let dia = |desfase: i64| {
            // Marcador «hoy» de `APP-404`.
            if desfase == i64::MIN {
                hoy
            } else {
                lunes + Dias::days(desfase)
            }
        };
        let tareas = defs
            .iter()
            .zip(registros)
            .map(|(d, regs)| {
                let (estado, categoria) = d.estado.nombre_y_categoria();
                let prefijo = d.clave.split('-').next().unwrap_or("");
                let gastado: i64 = regs.iter().map(|r| r.segundos).sum();
                let estimado: i64 = match d.estimacion {
                    Estimacion::Ninguna => 0,
                    Estimacion::Horas(h) => h * 3600,
                    Estimacion::Factor(f) => ((gastado as f64 * f / 3600.0).ceil() as i64) * 3600,
                };
                let resolucion = d.fin_dev.or(d.resuelta).map(dia);
                let es_subtarea = d.subtarea_de.is_some();
                let mut c = Map::new();
                c.insert("summary".into(), json!(d.resumen));
                c.insert("status".into(), json!({ "name": estado, "statusCategory": { "key": categoria } }));
                c.insert("project".into(), json!({ "key": prefijo, "name": nombre_proyecto(prefijo) }));
                c.insert(
                    "issuetype".into(),
                    json!({ "name": if es_subtarea { "Subtarea" } else { "Tarea" }, "subtask": es_subtarea }),
                );
                c.insert("priority".into(), json!({ "name": d.prioridad }));
                c.insert("created".into(), json!(fecha_hora(dia(d.creada), 9)));
                let actualizada = match (d.estado, resolucion) {
                    (Estado::Finalizada, Some(r)) => fecha_hora(r, 16),
                    _ => fecha_hora(hoy, 10),
                };
                c.insert("updated".into(), json!(actualizada));
                let cambio_estado = resolucion.unwrap_or_else(|| dia(d.creada) + Dias::days(1));
                c.insert("statuscategorychangedate".into(), json!(fecha_hora(cambio_estado, 11)));
                if let (Estado::Finalizada, Some(r)) = (d.estado, resolucion) {
                    c.insert("resolutiondate".into(), json!(fecha_hora(r, 16)));
                }
                if let Some(v) = d.vence {
                    c.insert("duedate".into(), json!(dia(v).format("%Y-%m-%d").to_string()));
                }
                if let Some(i) = d.inicio {
                    c.insert("customfield_10015".into(), json!(dia(i).format("%Y-%m-%d").to_string()));
                }
                if let Some(f) = d.fin_dev {
                    c.insert("customfield_10057".into(), json!(dia(f).format("%Y-%m-%d").to_string()));
                }
                c.insert("timeoriginalestimate".into(), if estimado > 0 { json!(estimado) } else { Value::Null });
                c.insert("timespent".into(), if gastado > 0 { json!(gastado) } else { Value::Null });
                c.insert("components".into(), json!(d.componente.map(|n| vec![json!({ "name": n })]).unwrap_or_default()));
                c.insert("labels".into(), json!(d.etiquetas));
                if let Some(p) = d.puntos {
                    c.insert("customfield_10031".into(), json!(p));
                }
                if !d.sprints.is_empty() {
                    let sprints: Vec<Value> = d
                        .sprints
                        .iter()
                        .map(|s| {
                            let fin = dia(s.fin);
                            json!({
                                "id": s.id,
                                "name": s.nombre,
                                "state": s.estado,
                                "boardId": s.tablero,
                                "startDate": format!("{}T13:00:00.000Z", (fin - Dias::days(13)).format("%Y-%m-%d")),
                                "endDate": format!("{}T22:00:00.000Z", fin.format("%Y-%m-%d")),
                            })
                        })
                        .collect();
                    c.insert("customfield_10020".into(), Value::Array(sprints));
                }
                if let Some(padre) = d.subtarea_de {
                    c.insert("parent".into(), json!({ "key": padre }));
                }
                TareaSim { clave: d.clave, resumen: d.resumen, campos: c, registros: regs, pr: d.pr, doc: d.doc }
            })
            .collect();
        JiraSimulado { correo: correo.to_string(), token_valido, tareas }
    }

    /// Espera un poco (para que se vean los estados de carga) y revisa el token.
    async fn atender(&self) -> Resultado<()> {
        tokio::time::sleep(ESPERA).await;
        if self.token_valido {
            Ok(())
        } else {
            Err(ErrorApp::NoAutorizado)
        }
    }

    pub async fn yo(&self) -> Resultado<Usuario> {
        self.atender().await?;
        Ok(Usuario {
            account_id: YO.to_string(),
            nombre: "Persona de Prueba".to_string(),
            correo: Some(self.correo.clone()),
        })
    }

    /// Interpreta el JQL por su contenido: con `development[pullrequests]` solo las tareas con PR;
    /// con `worklogDate`, las que tienen horas propias; en otro caso, todas. Pagina de 10 en 10.
    pub async fn buscar(
        &self,
        jql: &str,
        campos: &[&str],
        siguiente: Option<&str>,
        _tam: u32,
    ) -> Resultado<PaginaBusqueda> {
        self.atender().await?;
        let desde = if jql.contains("worklogDate") {
            Some(
                jql.split_once("worklogDate >= \"")
                    .and_then(|(_, resto)| resto.get(..10))
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
            )
        } else {
            None
        };
        let elegidas: Vec<&TareaSim> = self
            .tareas
            .iter()
            .filter(|t| {
                if jql.contains("development[pullrequests]") {
                    t.pr
                } else if let Some(d) = desde {
                    t.tiene_horas_propias_desde(d)
                } else {
                    true
                }
            })
            .collect();
        let inicio = match siguiente {
            None => 0,
            Some(t) => t
                .strip_prefix("sim-pag-")
                .and_then(|n| n.parse::<usize>().ok())
                .ok_or_else(|| ErrorApp::Formato("token de página no válido".into()))?,
        };
        let fin = (inicio + TAM_PAGINA).min(elegidas.len());
        let issues = elegidas.get(inicio..fin).unwrap_or(&[]).iter().map(|t| t.issue(campos)).collect();
        let siguiente = (fin < elegidas.len()).then(|| format!("sim-pag-{fin}"));
        Ok(PaginaBusqueda { issues, siguiente })
    }

    /// Todos los registros de la tarea (de cualquier autor) iniciados desde `desde_ms`.
    pub async fn worklogs(&self, clave: &str, desde_ms: i64) -> Resultado<Vec<Value>> {
        let clave = validar_clave(clave)?;
        self.atender().await?;
        Ok(self
            .tareas
            .iter()
            .find(|t| t.clave == clave)
            .map(|t| t.registros.iter().filter(|r| r.inicio_ms() >= desde_ms).map(Reg::a_json).collect())
            .unwrap_or_default())
    }

    /// Devuelve la página «Documento de Despliegue» de las tareas con documento en Confluence
    /// cuya clave aparece en el CQL.
    pub async fn confluence(&self, cql: &str) -> Resultado<Vec<PaginaWiki>> {
        self.atender().await?;
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"[A-Z][A-Z0-9_]*-[0-9]+").expect("regex válida"));
        let pedidas: HashSet<&str> = re.find_iter(cql).map(|m| m.as_str()).collect();
        Ok(self
            .tareas
            .iter()
            .enumerate()
            .filter(|(_, t)| t.doc == Documento::Confluence && pedidas.contains(t.clave))
            .map(|(i, t)| t.pagina_wiki(i))
            .collect())
    }

    pub async fn enlaces(&self, clave: &str) -> Resultado<Vec<EnlaceRemoto>> {
        let clave = validar_clave(clave)?;
        self.atender().await?;
        let Some((i, t)) = self.tareas.iter().enumerate().find(|(_, t)| t.clave == clave) else {
            return Ok(Vec::new());
        };
        let mut enlaces = Vec::new();
        if t.pr {
            enlaces.push(EnlaceRemoto {
                titulo: format!("Pull request de {}", t.clave),
                url: format!("https://repositorio.ejemplo.test/pulls/{}", 100 + i),
            });
        }
        if t.doc == Documento::SoloEnlace {
            let mut p = t.pagina_wiki(i);
            p.url = format!("{SITIO}/wiki/spaces/DEV/pages/{}/Documento+de+Despliegue+{}", 9_000 + i, t.clave);
            enlaces.push(EnlaceRemoto { titulo: p.titulo, url: p.url });
        }
        Ok(enlaces)
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::jira::{CAMPOS, JQL_TAREAS};

    fn hoy() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
    }

    async fn todas(s: &JiraSimulado) -> Vec<Value> {
        let mut out = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let p = s.buscar(JQL_TAREAS, CAMPOS, token.as_deref(), 100).await.unwrap();
            out.extend(p.issues);
            token = p.siguiente;
            if token.is_none() {
                return out;
            }
        }
    }

    #[tokio::test]
    async fn es_determinista_para_un_hoy_fijo() {
        let a = todas(&JiraSimulado::nuevo("a@b.com", true, hoy())).await;
        let b = todas(&JiraSimulado::nuevo("otro@b.com", true, hoy())).await;
        assert_eq!(a, b);
        assert!(a.len() >= 17);
        let otro_dia = todas(&JiraSimulado::nuevo("a@b.com", true, hoy() + Dias::days(7))).await;
        assert_ne!(a, otro_dia);
    }

    #[tokio::test]
    async fn no_genera_horas_en_feriados_ni_fines_de_semana_salvo_el_sabado_a_proposito() {
        let s = JiraSimulado::nuevo("a@b.com", true, hoy());
        let mut sabados = 0;
        let mut dias_con_horas: HashSet<NaiveDate> = HashSet::new();
        for t in &s.tareas {
            for r in t.registros.iter().filter(|r| r.autor == YO) {
                assert!(!es_feriado(r.dia), "{} registra horas en feriado {}", t.clave, r.dia);
                if matches!(r.dia.weekday(), Weekday::Sat | Weekday::Sun) {
                    sabados += 1;
                    assert_eq!(t.clave, "AFO-101");
                } else {
                    dias_con_horas.insert(r.dia);
                }
            }
        }
        assert_eq!(sabados, 1);
        // El 2 y 3 de noviembre son feriados y no hay horas; un día laborable normal sí tiene.
        assert!(!dias_con_horas.contains(&NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()));
        assert!(dias_con_horas.contains(&NaiveDate::from_ymd_opt(2026, 11, 4).unwrap()));
    }

    #[tokio::test]
    async fn dias_normales_suman_ocho_horas_y_la_semana_actual_tiene_huecos() {
        let s = JiraSimulado::nuevo("a@b.com", true, hoy() + Dias::days(3)); // jueves 19-nov
        let suma = |d: &str| -> i64 {
            let d = NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap();
            s.tareas.iter().flat_map(|t| &t.registros).filter(|r| r.autor == YO && r.dia == d).map(|r| r.segundos).sum()
        };
        assert_eq!(suma("2026-11-10"), 8 * 3600);
        assert_eq!(suma("2026-11-16"), 0); // lunes actual: 0 h
        assert_eq!(suma("2026-11-17"), 4 * 3600); // martes actual: menos de 8 h
        assert_eq!(suma("2026-11-18"), 8 * 3600);
    }

    #[tokio::test]
    async fn worklog_embebido_tiene_tope_de_20_y_total_real() {
        let s = JiraSimulado::nuevo("a@b.com", true, hoy());
        let todas = todas(&s).await;
        let base = todas.iter().find(|v| v["key"] == "OPE-702").unwrap();
        let wl = &base["fields"]["worklog"];
        assert_eq!(wl["worklogs"].as_array().unwrap().len(), 20);
        assert!(wl["total"].as_u64().unwrap() > 20);
        let completo = s.worklogs("OPE-702", 0).await.unwrap();
        assert_eq!(completo.len() as u64, wl["total"].as_u64().unwrap());
        // `desde_ms` recorta por fecha de inicio.
        let reciente = s.worklogs("OPE-702", 1_794_000_000_000).await.unwrap(); // 2026-11-07 aprox.
        assert!(!reciente.is_empty() && reciente.len() < completo.len());
        assert!(s.worklogs("ZZZ-1", 0).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn la_forma_json_es_la_de_jira_y_cubre_los_casos_del_tablero() {
        use crate::jira::modelo::Tarea;
        let s = JiraSimulado::nuevo("a@b.com", true, hoy());
        let tareas: Vec<Tarea> =
            todas(&s).await.iter().map(|v| Tarea::desde_jira(v, Some(YO)).unwrap()).collect();
        let por = |k: &str| tareas.iter().find(|t| t.key == k).unwrap();
        assert_eq!(por("AFO-101").sprints[0].board_id, Some(455));
        assert_eq!(por("DAI-201").sprints[0].board_id, Some(92));
        assert!(por("AFO-103").subtask && por("AFO-103").parent.as_deref() == Some("AFO-101"));
        assert_eq!(por("OPE-602").comp, "DAI");
        assert_eq!(por("OPE-701").comp, "Sin componente");
        assert_eq!(por("ADN-301").cat.as_deref(), Some("new"));
        assert_eq!(por("AFO-102").status.as_deref(), Some("En pruebas"));
        assert_eq!(por("DAI-202").cat.as_deref(), Some("done"));
        assert!(por("DAI-202").fin_dev.is_none() && por("DAI-202").resolved.is_some());
        assert!(por("DAI-201").fin_dev.is_some());
        let a = por("AFO-101");
        assert!(a.spent as f64 > a.est as f64 * 1.2, "debe superar la estimación");
        // Concluida después del vencimiento.
        assert!(por("DAI-201").fin_dev.as_deref().unwrap() > por("DAI-201").due.as_deref().unwrap());
        // Abierta y vencida.
        assert!(por("ADN-302").due.as_deref().unwrap() < "2026-11-16" && por("ADN-302").resolved.is_none());
        // Fechas en formato RFC 3339 tras la normalización.
        assert!(a.created.as_deref().unwrap().ends_with("-05:00"));
    }

    #[test]
    fn se_genera_sin_errores_para_cualquier_dia_de_la_semana() {
        for n in 0..21 {
            let s = JiraSimulado::nuevo("a@b.com", true, hoy() + Dias::days(n));
            assert_eq!(s.tareas.len(), definiciones().len());
        }
    }

    #[test]
    fn toda_tarea_con_ventana_tiene_horas_propias() {
        let s = JiraSimulado::nuevo("a@b.com", true, hoy());
        for (d, t) in definiciones().iter().zip(&s.tareas) {
            assert_eq!(
                d.ventana.is_some(),
                t.tiene_horas_propias_desde(None),
                "{}: la ventana de horas y los registros no coinciden",
                t.clave
            );
        }
    }

    #[tokio::test]
    async fn token_invalido_rechaza_todas_las_llamadas() {
        let s = JiraSimulado::nuevo("a@b.com", false, hoy());
        assert!(matches!(s.yo().await, Err(ErrorApp::NoAutorizado)));
        assert!(matches!(s.buscar("x", &[], None, 10).await, Err(ErrorApp::NoAutorizado)));
        assert!(matches!(s.worklogs("APP-1", 0).await, Err(ErrorApp::NoAutorizado)));
    }

    #[tokio::test]
    async fn token_de_pagina_invalido_es_error_de_formato() {
        let s = JiraSimulado::nuevo("a@b.com", true, hoy());
        assert!(matches!(s.buscar(JQL_TAREAS, CAMPOS, Some("xx"), 10).await, Err(ErrorApp::Formato(_))));
    }

    #[test]
    fn hoy_ecuador_es_una_fecha_cercana_a_la_de_utc() {
        let dif = (Utc::now().date_naive() - hoy_ecuador()).num_days();
        assert!((0..=1).contains(&dif));
    }
}
