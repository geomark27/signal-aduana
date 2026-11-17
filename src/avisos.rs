//! Decisión de avisos de Windows: lógica pura, sin reloj ni Tauri.
//! La hora «ahora» llega como parámetro (hora de Ecuador, UTC-5).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::domain::calendar::{fh, DAY_TARGET_H};
use crate::error::{AppError, AppResult};
use crate::preferencias::Preferencias;
use crate::resumen::Resumen;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoAviso {
    CierreJornada,
    ResumenLunes,
    Reconectar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aviso {
    pub tipo: TipoAviso,
    pub titulo: String,
    pub cuerpo: String,
}

/// Última fecha (`AAAA-MM-DD`) en que se envió cada aviso.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Registro {
    pub cierre: Option<String>,
    pub lunes: Option<String>,
    pub reconectar: Option<String>,
}

pub enum EstadoMonitor<'a> {
    Resumen(&'a Resumen),
    NoAutorizado,
    SinSesion,
    Error,
}

fn texto_fecha(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn ya_enviado(fecha: &Option<String>, hoy: &str) -> bool {
    fecha.as_deref() == Some(hoy)
}

/// Qué avisos corresponden ahora. Con `prefs.avisos.activos == false` no se envía nada.
pub fn decidir(
    estado: EstadoMonitor,
    ahora: NaiveDateTime,
    prefs: &Preferencias,
    registro: &Registro,
    simulado: bool,
) -> Vec<Aviso> {
    let mut avisos = Vec::new();
    if !prefs.avisos.activos {
        return avisos;
    }
    let hoy = texto_fecha(ahora.date());
    let prefijo = if simulado { "[Simulado] " } else { "" };

    match estado {
        EstadoMonitor::NoAutorizado => {
            if !ya_enviado(&registro.reconectar, &hoy) {
                avisos.push(Aviso {
                    tipo: TipoAviso::Reconectar,
                    titulo: format!("{prefijo}Vuelve a conectar Signal Aduana"),
                    cuerpo: "Tu token de Jira venció o fue revocado. Abre la app y conéctate de nuevo.".into(),
                });
            }
        }
        // Un resumen de otro día (dato viejo) no sirve para decidir.
        EstadoMonitor::Resumen(r) if r.hoy == hoy => {
            let meta_dia = DAY_TARGET_H * 3600;
            let cierre = NaiveTime::parse_from_str(&prefs.avisos.hora_cierre, "%H:%M").ok();
            if r.hoy_laborable
                && cierre.is_some_and(|c| ahora.time() >= c)
                && r.hoy_secs < meta_dia
                && !ya_enviado(&registro.cierre, &hoy)
            {
                avisos.push(Aviso {
                    tipo: TipoAviso::CierreJornada,
                    titulo: format!("{prefijo}Te faltan {} hoy", fh(meta_dia - r.hoy_secs)),
                    cuerpo: format!(
                        "Llevas {} de {} h. Registra tus horas en Jira antes de cerrar el día.",
                        fh(r.hoy_secs).trim_end_matches(" h"),
                        DAY_TARGET_H
                    ),
                });
            }
            let meta_ant = r.anterior_meta_h * 3600;
            if r.primer_laborable.as_deref() == Some(hoy.as_str())
                && ahora.time() >= NaiveTime::from_hms_opt(9, 0, 0).expect("hora válida")
                && prefs.avisos.resumen_lunes
                && r.anterior_meta_h > 0
                && r.anterior_secs < meta_ant
                && !ya_enviado(&registro.lunes, &hoy)
            {
                avisos.push(Aviso {
                    tipo: TipoAviso::ResumenLunes,
                    titulo: format!("{prefijo}La semana pasada faltaron {}", fh(meta_ant - r.anterior_secs)),
                    cuerpo: format!(
                        "Registraste {} de {}. Si trabajaste esas horas, regístralas en Jira con su fecha real.",
                        fh(r.anterior_secs),
                        fh(meta_ant)
                    ),
                });
            }
        }
        EstadoMonitor::Resumen(_) | EstadoMonitor::SinSesion | EstadoMonitor::Error => {}
    }
    avisos
}

/// Anota en el registro los avisos enviados (la fecha de hoy; el del lunes es de esa semana).
pub fn marcar(registro: &mut Registro, avisos: &[Aviso], hoy: NaiveDate) {
    let f = Some(texto_fecha(hoy));
    for a in avisos {
        match a.tipo {
            TipoAviso::CierreJornada => registro.cierre = f.clone(),
            TipoAviso::ResumenLunes => registro.lunes = f.clone(),
            TipoAviso::Reconectar => registro.reconectar = f.clone(),
        }
    }
}

fn ruta(dir: &Path, simulado: bool) -> PathBuf {
    dir.join(if simulado { "avisos.simulado.json" } else { "avisos.json" })
}

/// Sin archivo o dañado → registro vacío.
pub fn cargar_registro(dir: &Path, simulado: bool) -> Registro {
    fs::read_to_string(ruta(dir, simulado)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn guardar_registro(dir: &Path, simulado: bool, r: &Registro) -> AppResult<()> {
    fs::create_dir_all(dir).map_err(|e| AppError::Config(e.to_string()))?;
    let texto = serde_json::to_string_pretty(r).map_err(|e| AppError::Config(e.to_string()))?;
    let destino = ruta(dir, simulado);
    let tmp = destino.with_extension("json.tmp");
    fs::write(&tmp, texto).map_err(|e| AppError::Config(e.to_string()))?;
    fs::rename(&tmp, &destino).map_err(|e| AppError::Config(e.to_string()))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn ahora(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn lunes_de(hoy: &str) -> String {
        let d = NaiveDate::parse_from_str(hoy, "%Y-%m-%d").unwrap();
        let lunes = d - chrono::Duration::days(i64::from(chrono::Datelike::weekday(&d).num_days_from_monday()));
        texto_fecha(lunes)
    }

    fn res(hoy: &str) -> Resumen {
        Resumen {
            hoy: hoy.into(),
            semana_secs: 0,
            semana_meta_h: 40,
            hoy_secs: 3 * 3600,
            hoy_laborable: true,
            anterior_secs: 32 * 3600,
            anterior_meta_h: 40,
            primer_laborable: Some(lunes_de(hoy)),
        }
    }

    fn tipos(v: &[Aviso]) -> Vec<TipoAviso> {
        v.iter().map(|a| a.tipo).collect()
    }

    fn con(r: &Resumen, t: &str) -> Vec<Aviso> {
        decidir(EstadoMonitor::Resumen(r), ahora(t), &Preferencias::default(), &Registro::default(), false)
    }

    #[test]
    fn cierre_antes_y_despues_de_la_hora() {
        let r = res("2026-11-13"); // viernes
        assert!(con(&r, "2026-11-13 16:29").is_empty());
        let v = con(&r, "2026-11-13 16:30");
        assert_eq!(tipos(&v), [TipoAviso::CierreJornada]);
        assert_eq!(v[0].titulo, "Te faltan 5 h hoy");
        assert_eq!(v[0].cuerpo, "Llevas 3 de 8 h. Registra tus horas en Jira antes de cerrar el día.");
        let mut r2 = res("2026-11-13");
        r2.hoy_secs = 7 * 3600 + 1800;
        let v = con(&r2, "2026-11-13 18:00");
        assert_eq!(v[0].titulo, "Te faltan 0.5 h hoy");
        assert!(v[0].cuerpo.starts_with("Llevas 7.5 de 8 h."));
    }

    #[test]
    fn cierre_respeta_hora_configurada() {
        let r = res("2026-11-13");
        let mut p = Preferencias::default();
        p.avisos.hora_cierre = "18:00".into();
        let d = |t| decidir(EstadoMonitor::Resumen(&r), ahora(t), &p, &Registro::default(), false);
        assert!(d("2026-11-13 17:59").is_empty());
        assert_eq!(tipos(&d("2026-11-13 18:00")), [TipoAviso::CierreJornada]);
    }

    #[test]
    fn cierre_no_en_feriado_fin_de_semana_ni_con_jornada_completa() {
        let mut r = res("2026-11-13");
        r.hoy_laborable = false; // feriado
        assert!(con(&r, "2026-11-13 17:00").is_empty());
        let mut s = res("2026-11-14"); // sábado
        s.hoy_laborable = false;
        assert!(con(&s, "2026-11-14 17:00").is_empty());
        let mut c = res("2026-11-13");
        c.hoy_secs = 8 * 3600;
        assert!(con(&c, "2026-11-13 17:00").is_empty());
        c.hoy_secs = 10 * 3600;
        assert!(con(&c, "2026-11-13 17:00").is_empty());
    }

    #[test]
    fn cierre_ya_enviado_hoy() {
        let r = res("2026-11-13");
        let mut reg = Registro { cierre: Some("2026-11-13".into()), ..Registro::default() };
        let p = Preferencias::default();
        assert!(decidir(EstadoMonitor::Resumen(&r), ahora("2026-11-13 17:00"), &p, &reg, false).is_empty());
        // Enviado ayer: hoy sí.
        reg.cierre = Some("2026-11-12".into());
        assert_eq!(decidir(EstadoMonitor::Resumen(&r), ahora("2026-11-13 17:00"), &p, &reg, false).len(), 1);
    }

    #[test]
    fn lunes_antes_y_despues_de_las_nueve() {
        let mut r = res("2026-11-16");
        r.hoy_secs = 0;
        assert!(con(&r, "2026-11-16 08:59").is_empty());
        let v = con(&r, "2026-11-16 09:00");
        assert_eq!(tipos(&v), [TipoAviso::ResumenLunes]);
        assert_eq!(v[0].titulo, "La semana pasada faltaron 8 h");
        assert_eq!(
            v[0].cuerpo,
            "Registraste 32 h de 40 h. Si trabajaste esas horas, regístralas en Jira con su fecha real."
        );
        // Después de la hora de cierre, el lunes salen los dos.
        assert_eq!(tipos(&con(&r, "2026-11-16 17:00")), [TipoAviso::CierreJornada, TipoAviso::ResumenLunes]);
    }

    #[test]
    fn lunes_no_aplica_si_esta_completa_o_sin_meta_o_otro_dia() {
        let mut r = res("2026-11-16");
        r.anterior_secs = 40 * 3600;
        assert!(con(&r, "2026-11-16 09:30").is_empty());
        r.anterior_secs = 45 * 3600;
        assert!(con(&r, "2026-11-16 09:30").is_empty());
        // Semana anterior toda feriado: meta 0.
        r.anterior_meta_h = 0;
        r.anterior_secs = 0;
        assert!(con(&r, "2026-11-16 09:30").is_empty());
        // Martes con la semana pasada incompleta: no.
        let t = res("2026-11-17");
        assert!(con(&t, "2026-11-17 10:00").is_empty());
    }

    #[test]
    fn lunes_ya_enviado_y_desactivado() {
        let r = res("2026-11-16");
        let reg = Registro { lunes: Some("2026-11-16".into()), ..Registro::default() };
        let p = Preferencias::default();
        assert!(decidir(EstadoMonitor::Resumen(&r), ahora("2026-11-16 10:00"), &p, &reg, false).is_empty());
        let mut p = Preferencias::default();
        p.avisos.resumen_lunes = false;
        assert!(decidir(EstadoMonitor::Resumen(&r), ahora("2026-11-16 10:00"), &p, &Registro::default(), false).is_empty());
    }

    #[test]
    fn avisos_desactivados_no_envian_nada() {
        let mut p = Preferencias::default();
        p.avisos.activos = false;
        let r = res("2026-11-16");
        let reg = Registro::default();
        assert!(decidir(EstadoMonitor::Resumen(&r), ahora("2026-11-16 17:00"), &p, &reg, false).is_empty());
        assert!(decidir(EstadoMonitor::NoAutorizado, ahora("2026-11-16 17:00"), &p, &reg, false).is_empty());
    }

    #[test]
    fn reconectar_una_vez_al_dia() {
        let p = Preferencias::default();
        let mut reg = Registro::default();
        let v = decidir(EstadoMonitor::NoAutorizado, ahora("2026-11-13 10:00"), &p, &reg, false);
        assert_eq!(tipos(&v), [TipoAviso::Reconectar]);
        assert_eq!(v[0].titulo, "Vuelve a conectar Signal Aduana");
        assert_eq!(v[0].cuerpo, "Tu token de Jira venció o fue revocado. Abre la app y conéctate de nuevo.");
        marcar(&mut reg, &v, ahora("2026-11-13 10:00").date());
        assert!(decidir(EstadoMonitor::NoAutorizado, ahora("2026-11-13 15:00"), &p, &reg, false).is_empty());
        assert_eq!(decidir(EstadoMonitor::NoAutorizado, ahora("2026-11-14 10:00"), &p, &reg, false).len(), 1);
    }

    #[test]
    fn sin_sesion_y_error_no_avisan() {
        let p = Preferencias::default();
        let reg = Registro::default();
        assert!(decidir(EstadoMonitor::SinSesion, ahora("2026-11-16 17:00"), &p, &reg, false).is_empty());
        assert!(decidir(EstadoMonitor::Error, ahora("2026-11-16 17:00"), &p, &reg, false).is_empty());
    }

    #[test]
    fn resumen_de_otro_dia_no_decide() {
        let r = res("2026-11-12");
        assert!(con(&r, "2026-11-13 17:00").is_empty());
    }

    fn con_primero(hoy: &str, primero: Option<&str>) -> Resumen {
        let mut r = res(hoy);
        r.primer_laborable = primero.map(String::from);
        r
    }

    #[test]
    fn cierre_con_hora_invalida_no_avisa_ni_entra_en_panico() {
        let mut p = Preferencias::default();
        p.avisos.hora_cierre = "99:99".into();
        let reg = Registro::default();
        let viernes = res("2026-11-13");
        assert!(decidir(EstadoMonitor::Resumen(&viernes), ahora("2026-11-13 17:00"), &p, &reg, false).is_empty());
        let lunes = con_primero("2026-11-16", Some("2026-11-16"));
        let v = decidir(EstadoMonitor::Resumen(&lunes), ahora("2026-11-16 10:00"), &p, &reg, false);
        assert_eq!(tipos(&v), [TipoAviso::ResumenLunes]);
    }

    #[test]
    fn resumen_enviado_la_semana_pasada_vuelve_a_avisar() {
        let r = con_primero("2026-11-16", Some("2026-11-16"));
        let mut reg = Registro { lunes: Some("2026-11-09".into()), ..Registro::default() };
        let p = Preferencias::default();
        let t = ahora("2026-11-16 10:00");
        let v = decidir(EstadoMonitor::Resumen(&r), t, &p, &reg, false);
        assert_eq!(tipos(&v), [TipoAviso::ResumenLunes]);
        marcar(&mut reg, &v, t.date());
        assert_eq!(reg.lunes.as_deref(), Some("2026-11-16"));
    }

    #[test]
    fn lunes_feriado_no_envia_resumen() {
        let mut r = con_primero("2026-11-16", Some("2026-11-17"));
        r.hoy_laborable = false;
        assert!(con(&r, "2026-11-16 10:00").is_empty());
    }

    #[test]
    fn resumen_sale_el_martes_si_el_lunes_fue_feriado() {
        let r = con_primero("2026-11-17", Some("2026-11-17"));
        assert_eq!(tipos(&con(&r, "2026-11-17 10:00")), [TipoAviso::ResumenLunes]);
        assert!(con(&r, "2026-11-17 08:59").is_empty());
    }

    #[test]
    fn martes_normal_no_envia_resumen() {
        let r = con_primero("2026-11-17", Some("2026-11-16"));
        assert!(!tipos(&con(&r, "2026-11-17 10:00")).contains(&TipoAviso::ResumenLunes));
    }

    #[test]
    fn semana_toda_feriado_no_envia_resumen() {
        let r = con_primero("2026-11-16", None);
        assert!(!tipos(&con(&r, "2026-11-16 10:00")).contains(&TipoAviso::ResumenLunes));
    }

    #[test]
    fn simulado_lleva_prefijo() {
        let r = res("2026-11-16");
        let p = Preferencias::default();
        let reg = Registro::default();
        let v = decidir(EstadoMonitor::Resumen(&r), ahora("2026-11-16 17:00"), &p, &reg, true);
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|a| a.titulo.starts_with("[Simulado] ")));
        let v = decidir(EstadoMonitor::NoAutorizado, ahora("2026-11-16 17:00"), &p, &reg, true);
        assert_eq!(v[0].titulo, "[Simulado] Vuelve a conectar Signal Aduana");
    }

    #[test]
    fn marcar_y_registro_en_disco() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(cargar_registro(dir.path(), false), Registro::default());
        let r = res("2026-11-16");
        let hoy = ahora("2026-11-16 17:00");
        let mut reg = Registro::default();
        let v = decidir(EstadoMonitor::Resumen(&r), hoy, &Preferencias::default(), &reg, false);
        assert_eq!(v.len(), 2);
        marcar(&mut reg, &v, hoy.date());
        assert_eq!(reg.cierre.as_deref(), Some("2026-11-16"));
        assert_eq!(reg.lunes.as_deref(), Some("2026-11-16"));
        assert_eq!(reg.reconectar, None);
        guardar_registro(dir.path(), false, &reg).unwrap();
        let leido = cargar_registro(dir.path(), false);
        assert_eq!(leido, reg);
        assert!(decidir(EstadoMonitor::Resumen(&r), hoy, &Preferencias::default(), &leido, false).is_empty());
        // El simulado usa otro archivo; un archivo dañado da registro vacío.
        assert_eq!(cargar_registro(dir.path(), true), Registro::default());
        fs::write(ruta(dir.path(), false), "{ roto").unwrap();
        assert_eq!(cargar_registro(dir.path(), false), Registro::default());
    }
}
