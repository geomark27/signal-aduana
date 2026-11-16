//! Sesión de Jira: iniciar, restaurar y cerrar. Lógica sin Tauri, para poder probarla.
//!
//! El token solo vive en el almacén de secretos y dentro de la fuente de datos; nunca se
//! registra ni se devuelve a la interfaz.

use std::path::PathBuf;
use std::sync::Arc;

use secrecy::SecretString;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::almacen::{servicio_para, AlmacenSecretos};
use crate::cache;
use crate::auth::{validar_correo, Credenciales, Sitio};
use crate::config::{self, Config};
use crate::error::{ErrorApp, Resultado};
use crate::jira::http::JiraHttp;
use crate::jira::modelo::Usuario;
use crate::jira::simulado::{hoy_ecuador, JiraSimulado};
use crate::jira::Fuente;

/// Token que, en modo simulado, hace que Jira «rechace» las credenciales.
const TOKEN_RECHAZADO: &str = "rechazar";

pub struct Sesion {
    pub fuente: Fuente,
    pub usuario: Usuario,
    pub sitio: Sitio,
    pub correo: String,
}

pub struct Estado {
    pub simulado: bool,
    pub dir_config: PathBuf,
    pub dir_datos: PathBuf,
    pub almacen: Box<dyn AlmacenSecretos>,
    pub sesion: RwLock<Option<Arc<Sesion>>>,
}

/// Lo que la interfaz sabe de la sesión (Contrato B).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct InfoSesion {
    pub conectado: bool,
    pub simulado: bool,
    pub usuario: Option<Usuario>,
    pub sitio: Option<String>,
    pub correo: Option<String>,
    pub aviso: Option<String>,
}

impl Estado {
    pub fn nuevo(
        simulado: bool,
        dir_config: PathBuf,
        dir_datos: PathBuf,
        almacen: Box<dyn AlmacenSecretos>,
    ) -> Self {
        Estado { simulado, dir_config, dir_datos, almacen, sesion: RwLock::new(None) }
    }

    fn ruta_config(&self) -> PathBuf {
        config::ruta(&self.dir_config, self.simulado)
    }

    /// Sesión activa o `SinSesion`.
    pub async fn actual(&self) -> Resultado<Arc<Sesion>> {
        self.sesion.read().await.clone().ok_or(ErrorApp::SinSesion)
    }

    /// Descarta la sesión en memoria (no toca el token guardado).
    pub async fn limpiar(&self) {
        *self.sesion.write().await = None;
    }

    fn crear_fuente(&self, cred: Credenciales, correo: &str, token: &str) -> Resultado<Fuente> {
        if self.simulado {
            Ok(Fuente::Simulado(JiraSimulado::nuevo(correo, token != TOKEN_RECHAZADO, hoy_ecuador())))
        } else {
            Ok(Fuente::Http(JiraHttp::nuevo(cred)?))
        }
    }

    fn info(&self, s: &Sesion, aviso: Option<String>) -> InfoSesion {
        InfoSesion {
            conectado: true,
            simulado: self.simulado,
            usuario: Some(s.usuario.clone()),
            sitio: Some(s.sitio.como_texto()),
            correo: Some(s.correo.clone()),
            aviso,
        }
    }

    fn info_guardada(&self, aviso: Option<String>) -> InfoSesion {
        let c = Config::cargar(&self.ruta_config());
        InfoSesion { conectado: false, simulado: self.simulado, usuario: None, sitio: c.sitio, correo: c.correo, aviso }
    }
}

pub async fn iniciar(estado: &Estado, sitio: &str, correo: &str, token: &str) -> Resultado<InfoSesion> {
    let sitio = Sitio::parse(sitio)?;
    let correo = validar_correo(correo)?;
    let secreto = SecretString::from(token.to_string());
    let cred = Credenciales::token_clasico(sitio.clone(), &correo, secreto.clone())?;
    let fuente = estado.crear_fuente(cred, &correo, token)?;
    let usuario = fuente.yo().await?;

    // La conexión ya funcionó: si guardar falla, la sesión sigue y se avisa.
    let mut aviso = None;
    if let Err(e) = estado.almacen.guardar(&servicio_para(sitio.host()), &correo, &secreto) {
        aviso = Some(format!("Conectado, pero no se pudo guardar el token para la próxima vez. {e}"));
    }
    let cfg = Config { sitio: Some(sitio.como_texto()), correo: Some(correo.clone()) };
    if let Err(e) = cfg.guardar(&estado.ruta_config()) {
        aviso.get_or_insert_with(|| format!("Conectado, pero no se pudo guardar la configuración. {e}"));
    }

    let s = Arc::new(Sesion { fuente, usuario, sitio, correo });
    let info = estado.info(&s, aviso);
    *estado.sesion.write().await = Some(s);
    Ok(info)
}

pub async fn restaurar(estado: &Estado) -> InfoSesion {
    if let Some(s) = estado.sesion.read().await.clone() {
        return estado.info(&s, None);
    }
    let cfg = Config::cargar(&estado.ruta_config());
    let (Some(sitio_txt), Some(correo)) = (cfg.sitio.clone(), cfg.correo.clone()) else {
        return estado.info_guardada(None);
    };
    let Ok(sitio) = Sitio::parse(&sitio_txt) else {
        return estado.info_guardada(None);
    };
    let token = match estado.almacen.leer(&servicio_para(sitio.host()), &correo) {
        Ok(Some(t)) => t,
        Ok(None) => return estado.info_guardada(None),
        Err(e) => return estado.info_guardada(Some(format!("No se pudo leer el token guardado. {e}"))),
    };
    use secrecy::ExposeSecret;
    let texto = token.expose_secret().to_string();
    let intento = async {
        let cred = Credenciales::token_clasico(sitio.clone(), &correo, token)?;
        let fuente = estado.crear_fuente(cred, &correo, &texto)?;
        let usuario = fuente.yo().await?;
        Ok::<_, ErrorApp>(Sesion { fuente, usuario, sitio: sitio.clone(), correo: correo.clone() })
    };
    match intento.await {
        Ok(s) => {
            let s = Arc::new(s);
            let info = estado.info(&s, None);
            *estado.sesion.write().await = Some(s);
            info
        }
        Err(ErrorApp::NoAutorizado) => estado.info_guardada(Some(
            "Tu token guardado ya no es válido (venció o fue revocado). Genera uno nuevo y vuelve a conectarte.".into(),
        )),
        Err(e) => estado.info_guardada(Some(format!("No se pudo restaurar la sesión. {e}"))),
    }
}

pub async fn cerrar(estado: &Estado) -> InfoSesion {
    let cfg = Config::cargar(&estado.ruta_config());
    let mut aviso = None;
    if let (Some(sitio), Some(correo)) = (&cfg.sitio, &cfg.correo) {
        if let Ok(sitio) = Sitio::parse(sitio) {
            if let Err(e) = estado.almacen.borrar(&servicio_para(sitio.host()), correo) {
                aviso = Some(format!("Se cerró la sesión, pero no se pudo borrar el token guardado. {e}"));
            }
        }
    }
    cache::borrar(&estado.dir_datos, estado.simulado);
    estado.limpiar().await;
    estado.info_guardada(aviso)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::almacen::AlmacenMemoria;
    use secrecy::ExposeSecret;

    const SITIO: &str = "https://prueba.atlassian.net";
    const CORREO: &str = "persona@ejemplo.com";

    fn estado(dir: &tempfile::TempDir) -> Estado {
        Estado::nuevo(true, dir.path().to_path_buf(), dir.path().join("datos"), Box::new(AlmacenMemoria::default()))
    }

    #[tokio::test]
    async fn iniciar_guarda_token_y_config() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        let info = iniciar(&e, "prueba.atlassian.net", CORREO, "tok").await.unwrap();
        assert!(info.conectado && info.simulado);
        assert_eq!(info.sitio.as_deref(), Some(SITIO));
        assert_eq!(info.usuario.unwrap().account_id, "sim-0001");
        let t = e.almacen.leer(&servicio_para("prueba.atlassian.net"), CORREO).unwrap().unwrap();
        assert_eq!(t.expose_secret(), "tok");
        assert_eq!(Config::cargar(&e.ruta_config()).correo.as_deref(), Some(CORREO));
        assert!(e.actual().await.is_ok());
    }

    #[tokio::test]
    async fn token_rechazado_no_guarda_nada() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        let r = iniciar(&e, SITIO, CORREO, "rechazar").await;
        assert!(matches!(r, Err(ErrorApp::NoAutorizado)));
        assert!(e.almacen.leer(&servicio_para("prueba.atlassian.net"), CORREO).unwrap().is_none());
        assert!(matches!(e.actual().await, Err(ErrorApp::SinSesion)));
    }

    #[tokio::test]
    async fn valida_entradas() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        assert!(matches!(iniciar(&e, "https://google.com", CORREO, "t").await, Err(ErrorApp::SitioInvalido(_))));
        assert!(matches!(iniciar(&e, SITIO, "sin-arroba", "t").await, Err(ErrorApp::CorreoInvalido)));
        assert!(matches!(iniciar(&e, SITIO, CORREO, "  ").await, Err(ErrorApp::TokenVacio)));
    }

    #[tokio::test]
    async fn restaurar_y_cerrar() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        iniciar(&e, SITIO, CORREO, "tok").await.unwrap();
        e.limpiar().await;

        let info = restaurar(&e).await;
        assert!(info.conectado);
        assert_eq!(info.correo.as_deref(), Some(CORREO));

        let info = cerrar(&e).await;
        assert!(!info.conectado);
        // Conserva sitio y correo para prellenar, pero ya no hay token.
        assert_eq!(info.sitio.as_deref(), Some(SITIO));
        assert!(e.actual().await.is_err());
        let info = restaurar(&e).await;
        assert!(!info.conectado);
        assert_eq!(info.correo.as_deref(), Some(CORREO));
    }

    #[tokio::test]
    async fn restaurar_con_token_vencido_avisa() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        iniciar(&e, SITIO, CORREO, "tok").await.unwrap();
        e.limpiar().await;
        e.almacen
            .guardar(&servicio_para("prueba.atlassian.net"), CORREO, &SecretString::from("rechazar"))
            .unwrap();
        let info = restaurar(&e).await;
        assert!(!info.conectado);
        assert!(info.aviso.unwrap().contains("ya no es válido"));
        assert_eq!(info.sitio.as_deref(), Some(SITIO));
    }

    #[tokio::test]
    async fn cerrar_sesion_borra_la_cache() {
        let dir = tempfile::tempdir().unwrap();
        let e = estado(&dir);
        iniciar(&e, SITIO, CORREO, "tok").await.unwrap();
        cache::guardar(&e.dir_datos, true, "sim-0001", SITIO, serde_json::json!({ "a": 1 })).unwrap();
        assert!(cache::leer(&e.dir_datos, true, "sim-0001", SITIO).is_some());
        cerrar(&e).await;
        assert!(cache::leer(&e.dir_datos, true, "sim-0001", SITIO).is_none());
        assert!(!cache::ruta(&e.dir_datos, true).exists());
    }

    #[tokio::test]
    async fn restaurar_sin_configuracion() {
        let dir = tempfile::tempdir().unwrap();
        let info = restaurar(&estado(&dir)).await;
        assert_eq!(info, InfoSesion { simulado: true, ..Default::default() });
    }
}
