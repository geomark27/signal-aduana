//! Un ciclo del monitor: lee las horas, fija el tooltip de la bandeja y decide los avisos.
//! Lógica pura (sin Tauri ni reloj propio): la salida real se conecta con `Salida`.

use chrono::{Duration, NaiveDate, NaiveDateTime, Timelike, Utc};

use crate::avisos::{self, EstadoMonitor};
use crate::error::{AppError, AppResult};
use crate::jira;
use crate::preferencias::{self, Preferencias};
use crate::resumen::{self, Resumen, MAX_TEXTO_BANDEJA};
use crate::session::AppState;

/// Destino de lo que produce el ciclo (la bandeja y las notificaciones de Windows).
pub trait Salida: Send + Sync {
    fn tooltip(&self, texto: &str);
    fn notificar(&self, titulo: &str, cuerpo: &str);
}

/// Último dato bueno: el resumen y la hora de Ecuador en que se leyó.
#[derive(Debug, Default)]
pub struct Memoria {
    pub ultimo: Option<(Resumen, NaiveDateTime)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultadoCiclo {
    Datos,
    SinSesion,
    NoAutorizado,
    Fallo,
}

/// Hora de Ecuador (UTC-5, sin horario de verano).
pub fn ahora_ecuador() -> NaiveDateTime {
    (Utc::now() - Duration::hours(5)).naive_utc()
}

fn recortar(texto: String) -> String {
    texto.chars().take(MAX_TEXTO_BANDEJA).collect()
}

fn con_marca_simulado(mut texto: String, simulado: bool) -> String {
    if simulado {
        texto.push_str(" (simulado)");
    }
    recortar(texto)
}

/// Texto del tooltip cuando no hay sesión.
pub fn texto_sin_sesion(simulado: bool) -> String {
    con_marca_simulado("Signal Aduana · Sin sesión".into(), simulado)
}

/// Texto del tooltip cuando un ciclo entró en pánico.
pub fn texto_error_interno(simulado: bool) -> String {
    con_marca_simulado("Signal Aduana · Error interno, se reintenta".into(), simulado)
}

async fn leer_resumen(
    fuente: &jira::Source,
    cuenta: &str,
    hoy: NaiveDate,
    prefs: &Preferencias,
) -> AppResult<Resumen> {
    let tareas = jira::load_for_summary(fuente, Some(cuenta), hoy).await?;
    resumen::resumir(tareas, hoy, &prefs.feriados_extra)
}

/// Una pasada del monitor. La hora `ahora` (Ecuador) entra por parámetro.
pub async fn ciclo(
    estado: &AppState,
    salida: &dyn Salida,
    ahora: NaiveDateTime,
    memoria: &mut Memoria,
) -> ResultadoCiclo {
    let hoy = ahora.date();
    let prefs = preferencias::cargar(&estado.config_dir, estado.simulated);
    let mut registro = avisos::cargar_registro(&estado.data_dir, estado.simulated);

    let mut leido: Option<Resumen> = None;
    let (tooltip, resultado) = match estado.current().await {
        Err(_) => (texto_sin_sesion(estado.simulated), ResultadoCiclo::SinSesion),
        Ok(s) => match leer_resumen(&s.source, &s.user.account_id, hoy, &prefs).await {
            Ok(r) => {
                memoria.ultimo = Some((r.clone(), ahora));
                let texto = resumen::texto_bandeja(Some(&r), estado.simulated);
                leido = Some(r);
                (texto, ResultadoCiclo::Datos)
            }
            Err(AppError::Unauthorized) => {
                estado.clear().await;
                (
                    con_marca_simulado("Signal Aduana · Sin conexión con Jira".into(), estado.simulated),
                    ResultadoCiclo::NoAutorizado,
                )
            }
            Err(e) => {
                eprintln!("monitor: ciclo fallido: {e}");
                let texto = match &memoria.ultimo {
                    Some((r, t)) => recortar(format!(
                        "{} · último dato {:02}:{:02}",
                        resumen::texto_bandeja(Some(r), estado.simulated),
                        t.hour(),
                        t.minute()
                    )),
                    None => resumen::texto_bandeja(None, estado.simulated),
                };
                (texto, ResultadoCiclo::Fallo)
            }
        },
    };

    salida.tooltip(&tooltip);

    let estado_monitor = match (&leido, resultado) {
        (Some(r), _) => EstadoMonitor::Resumen(r),
        (None, ResultadoCiclo::SinSesion) => EstadoMonitor::SinSesion,
        (None, ResultadoCiclo::NoAutorizado) => EstadoMonitor::NoAutorizado,
        (None, _) => EstadoMonitor::Error,
    };
    let lista = avisos::decidir(estado_monitor, ahora, &prefs, &registro, estado.simulated);
    for a in &lista {
        salida.notificar(&a.titulo, &a.cuerpo);
    }
    if !lista.is_empty() {
        avisos::marcar(&mut registro, &lista, hoy);
        if let Err(e) = avisos::guardar_registro(&estado.data_dir, estado.simulated, &registro) {
            eprintln!("monitor: no se pudo guardar el registro de avisos: {e}");
        }
    }
    resultado
}

#[cfg(test)]
mod pruebas {
    use std::sync::{Arc, Mutex};

    use chrono::NaiveDate;
    use secrecy::SecretString;
    use url::Url;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::secret_store::MemorySecretStore;
    use crate::auth::{Credentials, Site};
    use crate::jira::http::JiraHttp;
    use crate::jira::simulated::SimulatedJira;
    use crate::jira::Source;
    use crate::session::Session;

    #[derive(Default)]
    struct Registrador {
        tooltips: Mutex<Vec<String>>,
        notificaciones: Mutex<Vec<(String, String)>>,
    }

    impl Salida for Registrador {
        fn tooltip(&self, texto: &str) {
            self.tooltips.lock().unwrap().push(texto.to_string());
        }
        fn notificar(&self, titulo: &str, cuerpo: &str) {
            self.notificaciones.lock().unwrap().push((titulo.to_string(), cuerpo.to_string()));
        }
    }

    impl Registrador {
        fn tooltips(&self) -> Vec<String> {
            self.tooltips.lock().unwrap().clone()
        }
        fn notificaciones(&self) -> Vec<(String, String)> {
            self.notificaciones.lock().unwrap().clone()
        }
    }

    fn fecha(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn ahora(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn estado(dir: &tempfile::TempDir) -> AppState {
        AppState::new(
            true,
            dir.path().join("config"),
            dir.path().join("datos"),
            Box::new(MemorySecretStore::default()),
        )
    }

    fn sitio() -> Site {
        Site::parse("prueba.atlassian.net").unwrap()
    }

    async fn poner_sesion(estado: &AppState, fuente: Source, usuario: crate::jira::model::User) {
        let s = Session { source: fuente, user: usuario, site: sitio(), email: "persona@ejemplo.test".into() };
        *estado.session.write().await = Some(Arc::new(s));
    }

    async fn sesion_simulada(estado: &AppState, hoy: &str, token_valido: bool) {
        let valida = Source::Simulated(SimulatedJira::new("persona@ejemplo.test", true, fecha(hoy)));
        let usuario = valida.myself().await.unwrap();
        let fuente = if token_valido {
            valida
        } else {
            Source::Simulated(SimulatedJira::new("persona@ejemplo.test", false, fecha(hoy)))
        };
        poner_sesion(estado, fuente, usuario).await;
    }

    fn todos_dentro_del_limite(r: &Registrador) {
        for t in r.tooltips() {
            assert!(t.chars().count() <= MAX_TEXTO_BANDEJA, "tooltip largo: {t}");
        }
    }

    #[test]
    fn texto_sin_sesion_con_y_sin_marca_simulado() {
        let real = texto_sin_sesion(false);
        assert!(real.ends_with("Sin sesión"));
        let sim = texto_sin_sesion(true);
        assert!(sim.ends_with(" (simulado)"));
        assert!(real.chars().count() <= MAX_TEXTO_BANDEJA);
        assert!(sim.chars().count() <= MAX_TEXTO_BANDEJA);
    }

    #[test]
    fn texto_error_interno_con_y_sin_marca_simulado() {
        let real = texto_error_interno(false);
        assert!(!real.ends_with(" (simulado)"));
        assert!(real.contains("Error interno"));
        let sim = texto_error_interno(true);
        assert!(sim.ends_with(" (simulado)"));
        assert!(real.chars().count() <= MAX_TEXTO_BANDEJA);
        assert!(sim.chars().count() <= MAX_TEXTO_BANDEJA);
    }

    #[tokio::test]
    async fn sin_sesion_muestra_tooltip_y_no_avisa() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        let salida = Registrador::default();
        let mut mem = Memoria::default();
        let r = ciclo(&e, &salida, ahora("2026-11-13 17:00"), &mut mem).await;
        assert_eq!(r, ResultadoCiclo::SinSesion);
        let t = salida.tooltips();
        assert_eq!(t.len(), 1);
        assert!(t[0].contains("Sin sesión"));
        assert!(t[0].ends_with(" (simulado)"));
        assert!(salida.notificaciones().is_empty());
        assert!(mem.ultimo.is_none());
        todos_dentro_del_limite(&salida);
    }

    #[tokio::test]
    async fn con_sesion_simulada_da_datos_y_tooltip() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        sesion_simulada(&e, "2026-11-13", true).await;
        let salida = Registrador::default();
        let mut mem = Memoria::default();
        let r = ciclo(&e, &salida, ahora("2026-11-13 10:00"), &mut mem).await;
        assert_eq!(r, ResultadoCiclo::Datos);
        let t = salida.tooltips();
        assert_eq!(t.len(), 1);
        assert!(t[0].starts_with("Signal Aduana · Semana:"));
        assert!(t[0].contains("(simulado)"));
        let (res, hora) = mem.ultimo.as_ref().unwrap();
        assert_eq!(res.hoy, "2026-11-13");
        assert_eq!(*hora, ahora("2026-11-13 10:00"));
        todos_dentro_del_limite(&salida);
    }

    async fn resumen_esperado(hoy: &str) -> Resumen {
        let f = Source::Simulated(SimulatedJira::new("persona@ejemplo.test", true, fecha(hoy)));
        let yo = f.myself().await.unwrap().account_id;
        let tareas = jira::load_for_summary(&f, Some(&yo), fecha(hoy)).await.unwrap();
        resumen::resumir(tareas, fecha(hoy), &Preferencias::default().feriados_extra).unwrap()
    }

    #[tokio::test]
    async fn cierre_de_jornada_coincide_con_decidir_y_no_se_repite() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        sesion_simulada(&e, "2026-11-13", true).await;
        let salida = Registrador::default();
        let mut mem = Memoria::default();
        let t = ahora("2026-11-13 17:00");

        let r = ciclo(&e, &salida, t, &mut mem).await;
        assert_eq!(r, ResultadoCiclo::Datos);

        let res = resumen_esperado("2026-11-13").await;
        let esperados = avisos::decidir(
            EstadoMonitor::Resumen(&res),
            t,
            &Preferencias::default(),
            &avisos::Registro::default(),
            true,
        );
        let enviados = salida.notificaciones();
        assert_eq!(enviados.len(), esperados.len());
        for (env, esp) in enviados.iter().zip(&esperados) {
            assert_eq!(env.0, esp.titulo);
            assert_eq!(env.1, esp.cuerpo);
        }
        assert!(res.hoy_secs < 8 * 3600);
        assert_eq!(enviados.len(), 1);
        assert!(enviados[0].0.starts_with("[Simulado] "));

        // El registro quedó en disco: el segundo ciclo del mismo día no repite.
        let r = ciclo(&e, &salida, ahora("2026-11-13 17:15"), &mut mem).await;
        assert_eq!(r, ResultadoCiclo::Datos);
        assert_eq!(salida.notificaciones().len(), enviados.len());
        assert_eq!(salida.tooltips().len(), 2);
        todos_dentro_del_limite(&salida);
    }

    #[tokio::test]
    async fn avisos_desactivados_no_notifican() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        let mut p = Preferencias::default();
        p.avisos.activos = false;
        preferencias::guardar(&e.config_dir, true, &p).unwrap();
        sesion_simulada(&e, "2026-11-13", true).await;
        let salida = Registrador::default();
        let mut mem = Memoria::default();
        let r = ciclo(&e, &salida, ahora("2026-11-13 17:00"), &mut mem).await;
        assert_eq!(r, ResultadoCiclo::Datos);
        assert!(salida.notificaciones().is_empty());
        assert_eq!(salida.tooltips().len(), 1);
        todos_dentro_del_limite(&salida);
    }

    #[tokio::test]
    async fn token_rechazado_limpia_la_sesion_y_avisa_una_vez() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        sesion_simulada(&e, "2026-11-13", false).await;
        let salida = Registrador::default();
        let mut mem = Memoria::default();

        let r = ciclo(&e, &salida, ahora("2026-11-13 10:00"), &mut mem).await;
        assert_eq!(r, ResultadoCiclo::NoAutorizado);
        assert!(matches!(e.current().await, Err(AppError::NoSession)));
        let t = salida.tooltips();
        assert_eq!(t.len(), 1);
        assert!(t[0].contains("Sin conexión con Jira"));
        let n = salida.notificaciones();
        assert_eq!(n.len(), 1);
        assert!(n[0].0.starts_with("[Simulado] "));
        assert!(n[0].0.contains("Vuelve a conectar"));

        // Otra sesión rechazada el mismo día: el aviso no se repite.
        sesion_simulada(&e, "2026-11-13", false).await;
        let r = ciclo(&e, &salida, ahora("2026-11-13 10:15"), &mut mem).await;
        assert_eq!(r, ResultadoCiclo::NoAutorizado);
        assert_eq!(salida.notificaciones().len(), 1);
        assert_eq!(salida.tooltips().len(), 2);
        todos_dentro_del_limite(&salida);
    }

    #[tokio::test]
    async fn fallo_de_red_conserva_la_sesion_y_muestra_el_ultimo_dato() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        sesion_simulada(&e, "2026-11-13", true).await;
        let salida = Registrador::default();
        let mut mem = Memoria::default();
        assert_eq!(ciclo(&e, &salida, ahora("2026-11-13 10:00"), &mut mem).await, ResultadoCiclo::Datos);

        let servidor = MockServer::start().await;
        Mock::given(wiremock::matchers::any()).respond_with(ResponseTemplate::new(500)).mount(&servidor).await;
        let cred = Credentials::classic_token(sitio(), "persona@ejemplo.test", SecretString::from("tok")).unwrap();
        let base = Url::parse(&format!("{}/", servidor.uri())).unwrap();
        let usuario = Source::Simulated(SimulatedJira::new("persona@ejemplo.test", true, fecha("2026-11-13")))
            .myself()
            .await
            .unwrap();
        poner_sesion(&e, Source::Http(JiraHttp::with_base(cred, base)), usuario).await;

        let r = ciclo(&e, &salida, ahora("2026-11-13 10:15"), &mut mem).await;
        assert_eq!(r, ResultadoCiclo::Fallo);
        assert!(e.current().await.is_ok());
        let t = salida.tooltips();
        assert_eq!(t.len(), 2);
        assert!(t[1].contains("último dato 10:00"), "tooltip: {}", t[1]);

        // Sin dato previo, el tooltip es el de «sin datos».
        let mut vacia = Memoria::default();
        let r = ciclo(&e, &salida, ahora("2026-11-13 10:30"), &mut vacia).await;
        assert_eq!(r, ResultadoCiclo::Fallo);
        assert!(salida.tooltips()[2].contains("Sin datos"));
        assert!(salida.notificaciones().is_empty());
        todos_dentro_del_limite(&salida);
    }
}
