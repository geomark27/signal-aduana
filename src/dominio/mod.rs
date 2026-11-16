//! Cálculos del tablero: funciones puras (sin Tauri ni reloj) portadas de `ui/app.js`.
//! `hoy` siempre llega como parámetro.

pub mod calendario;
pub mod horas;
pub mod semana;
pub mod tablero;
pub mod tarea;
pub mod vista;

pub use vista::{calcular_vista, Entrada, Vista};
