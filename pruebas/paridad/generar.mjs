// Genera los casos de paridad: ejecuta el JS legado (`legado.js`) sobre los volcados del simulado y
// escribe `casos/<nombre>.entrada.json` (Entrada) y `casos/<nombre>.esperado.json` (Vista, contrato D).
// Uso: node pruebas/paridad/generar.mjs   (no usa red ni reloj: `hoy` sale de cada caso)
import { readFileSync, writeFileSync, mkdirSync, readdirSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { crearLegado } from "./legado.js";

const aqui = dirname(fileURLToPath(import.meta.url));
const dirCasos = join(aqui, "casos");

const VOLCADOS = {
  viernes: { hoy: "2026-11-13", archivo: "volcados/viernes.json" },
  lunes: { hoy: "2026-11-16", archivo: "volcados/lunes.json" },
};
const cargar = k => JSON.parse(readFileSync(join(aqui, VOLCADOS[k].archivo), "utf8"));
const clonar = x => JSON.parse(JSON.stringify(x));

/* Casos: { nombre, volcado, semana (lunes), tablero, feriadosExtra } */
const TODA_FERIADO = d => Object.fromEntries([0, 1, 2, 3, 4].map(k => [sumar(d, k), true]));
function sumar(ymd, n) { return new Date(new Date(ymd + "T00:00:00Z").getTime() + n * 864e5).toISOString().slice(0, 10); }
const caso = (nombre, volcado, semana, tablero = "Todos", feriadosExtra = {}) => ({ nombre, volcado, semana, tablero, feriadosExtra });

const casos = [
  // Hoy viernes 2026-11-13
  caso("v-semana-actual", "viernes", "2026-11-09"),
  caso("v-semana-anterior", "viernes", "2026-11-02"),
  caso("v-hace-dos-semanas", "viernes", "2026-10-26"),
  caso("v-semana-futura", "viernes", "2026-11-23"),
  caso("v-feriados-base", "viernes", "2026-11-02"),
  caso("v-feriado-quitado", "viernes", "2026-11-02", "Todos", { "2026-11-02": false }),
  caso("v-feriado-agregado", "viernes", "2026-11-09", "Todos", { "2026-11-11": true }),
  caso("v-feriado-agregado-hoy", "viernes", "2026-11-09", "Todos", { "2026-11-13": true }),
  caso("v-semana-actual-toda-feriado", "viernes", "2026-11-09", "Todos", TODA_FERIADO("2026-11-09")),
  caso("v-semana-pasada-toda-feriado", "viernes", "2026-10-26", "Todos", TODA_FERIADO("2026-10-26")),
  caso("v-tablero-inexistente", "viernes", "2026-11-09", "Tablero Fantasma"),
  caso("v-tablero-todos-semana-anterior", "viernes", "2026-11-02", "Todos"),
  caso("v-cambio-de-anio-1", "viernes", "2026-12-28"),
  caso("v-cambio-de-anio-2", "viernes", "2027-01-04"),
  caso("v-semana-sin-datos", "viernes", "2026-05-04"),
  // Hoy lunes 2026-11-16 (primer día de la semana)
  caso("l-semana-actual", "lunes", "2026-11-16"),
  caso("l-semana-anterior", "lunes", "2026-11-09"),
  caso("l-hace-dos-semanas", "lunes", "2026-11-02"),
  caso("l-semana-futura", "lunes", "2026-11-23"),
  caso("l-hoy-feriado", "lunes", "2026-11-16", "Todos", { "2026-11-16": true }),
  caso("l-feriado-quitado", "lunes", "2026-11-02", "Todos", { "2026-11-03": false }),
  caso("l-semana-actual-toda-feriado", "lunes", "2026-11-16", "Todos", TODA_FERIADO("2026-11-16")),
  caso("l-tablero-inexistente", "lunes", "2026-11-09", "No existe"),
];

/* Un caso por cada tablero que resulta de los datos (el nombre sale de `resolveBoard`, no de una lista fija) */
function nombresTablero(volcado) {
  const L = crearLegado();
  L.estado({ issues: clonar(cargar(volcado)), board: "Todos", weekKey: "2026-11-09", feriadosExtra: {}, hoy: VOLCADOS[volcado].hoy });
  L.prep();
  return [...new Set(L.issues.map(i => i.board))].sort();
}
const slug = s => s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
for (const b of nombresTablero("viernes")) casos.push(caso("v-tablero-" + slug(b), "viernes", "2026-11-09", b));
for (const b of nombresTablero("lunes").slice(0, 2)) casos.push(caso("l-tablero-" + slug(b), "lunes", "2026-11-09", b));

/* ---------- adaptador: arma la `Vista` (contrato D) llamando solo a las funciones legadas ---------- */
function ordenar(x) {
  if (Array.isArray(x)) return x.map(ordenar);
  if (x && typeof x === "object") return Object.fromEntries(Object.keys(x).sort().map(k => [k, ordenar(x[k])]));
  return x;
}
const ordenado = o => Object.fromEntries(Object.keys(o).sort().map(k => [k, o[k]]));

function construirVista(entrada) {
  const { tareas, semana, tablero, feriadosExtra, hoy } = entrada;
  const L = crearLegado();
  L.estado({ issues: clonar(tareas), board: tablero, weekKey: semana, feriadosExtra, hoy });
  L.prep();
  const issues = L.issues;
  const tableros = L.boards();
  // Igual que `renderChips`: si el tablero no existe en los datos, se vuelve a "Todos".
  const efectivo = tablero === "Todos" || tableros.includes(tablero) ? tablero : "Todos";
  L.setBoard(efectivo);

  const s = L.weekStats(semana);
  const enOrden = set => issues.filter(i => set.has(i.key)).map(i => i.key); // claves en el orden de `tareas`
  const stats = {
    secs: s.secs, worked: enOrden(s.worked), perBoard: ordenado(s.perBoard),
    perDay: Object.fromEntries(Object.keys(s.perDay).sort().map(d => [d, ordenado(s.perDay[d])])),
    resolved: s.resolved.map(i => i.key), est: s.est, spent: s.spent, withDue: s.withDue, onTime: s.onTime,
  };

  const hs = L.hoursState(semana);
  const horas = {
    ...hs,
    prevSecs: L.weekStats(L.addDays(semana, -7), () => true).secs,
    laborables: L.laborDays(semana).length,
    dias: [0, 1, 2, 3, 4].map(k => { const day = L.addDays(semana, k); return { day, secs: L.daySecs(day), feriado: L.isFeriado(day) }; }),
    feriados: L.weekHolidays(semana),
  };

  const semanas = [];
  for (let k = 11; k >= 0; k--) {
    const wk = L.addDays(semana, -7 * k), w = L.weekStats(wk);
    semanas.push({ wk, semanaIso: L.isoWeek(wk).w, secs: w.secs, perBoard: ordenado(w.perBoard), target: L.weekTarget(wk) });
  }

  const porTablero = tableros.map(b => {
    const w = L.weekStats(semana, i => i.board === b);
    const abiertas = issues.filter(i => i.board === b && i.cat !== "done");
    const estados = {}; abiertas.forEach(i => { estados[i.status] = (estados[i.status] || 0) + 1; });
    return { board: b, secs: w.secs, worked: w.worked.size, resolved: w.resolved.length, abiertas: abiertas.length, estados: ordenado(estados), ultima: L.lastActivity(b) };
  });

  return { hoy, semana, tablero: efectivo, tareas: issues, tableros, stats, horas, semanas, porTablero };
}

/* ---------- escritura ---------- */
const escribir = (ruta, obj) => writeFileSync(ruta, JSON.stringify(ordenar(obj), null, 2) + "\n");
mkdirSync(dirCasos, { recursive: true });
for (const f of readdirSync(dirCasos)) if (/\.(entrada|esperado)\.json$/.test(f)) unlinkSync(join(dirCasos, f));
const nombres = new Set();
for (const c of casos) {
  if (nombres.has(c.nombre)) throw new Error("caso repetido: " + c.nombre);
  nombres.add(c.nombre);
  const entrada = { tareas: clonar(cargar(c.volcado)), semana: c.semana, tablero: c.tablero, feriadosExtra: c.feriadosExtra, hoy: VOLCADOS[c.volcado].hoy };
  const antes = JSON.stringify(entrada); // la Entrada se guarda tal cual, antes de `prep()`
  const vista = construirVista(entrada);
  if (JSON.stringify(entrada) !== antes) throw new Error("la entrada fue mutada: " + c.nombre);
  escribir(join(dirCasos, c.nombre + ".entrada.json"), entrada);
  escribir(join(dirCasos, c.nombre + ".esperado.json"), vista);
}
console.log(`${casos.length} casos escritos en ${dirCasos}`);
