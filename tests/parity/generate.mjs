// Generates the parity cases: runs the legacy JS (`legado.js`) over the simulated dumps and
// writes `cases/<name>.input.json` (ViewInput) and `cases/<name>.expected.json` (View, contract D).
// Usage: node tests/parity/generate.mjs   (no network or clock: `today` comes from each case)
import { readFileSync, writeFileSync, mkdirSync, readdirSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { crearLegado } from "./legado.js";

const here = dirname(fileURLToPath(import.meta.url));
const casesDir = join(here, "cases");

const DUMPS = {
  friday: { today: "2026-11-13", file: "dumps/friday.json" },
  monday: { today: "2026-11-16", file: "dumps/monday.json" },
};
const load = k => JSON.parse(readFileSync(join(here, DUMPS[k].file), "utf8"));
const deepClone = x => JSON.parse(JSON.stringify(x));

/* Cases: { name, dump, week (Monday), board, extraHolidays } */
const ALL_HOLIDAYS = d => Object.fromEntries([0, 1, 2, 3, 4].map(k => [addDays(d, k), true]));
function addDays(ymd, n) { return new Date(new Date(ymd + "T00:00:00Z").getTime() + n * 864e5).toISOString().slice(0, 10); }
const makeCase = (name, dump, week, board = "Todos", extraHolidays = {}) => ({ name, dump, week, board, extraHolidays });

const cases = [
  // Today is Friday 2026-11-13
  makeCase("fri-current-week", "friday", "2026-11-09"),
  makeCase("fri-previous-week", "friday", "2026-11-02"),
  makeCase("fri-two-weeks-ago", "friday", "2026-10-26"),
  makeCase("fri-future-week", "friday", "2026-11-23"),
  makeCase("fri-base-holidays", "friday", "2026-11-02"),
  makeCase("fri-holiday-removed", "friday", "2026-11-02", "Todos", { "2026-11-02": false }),
  makeCase("fri-holiday-added", "friday", "2026-11-09", "Todos", { "2026-11-11": true }),
  makeCase("fri-holiday-added-today", "friday", "2026-11-09", "Todos", { "2026-11-13": true }),
  makeCase("fri-current-week-all-holidays", "friday", "2026-11-09", "Todos", ALL_HOLIDAYS("2026-11-09")),
  makeCase("fri-last-week-all-holidays", "friday", "2026-10-26", "Todos", ALL_HOLIDAYS("2026-10-26")),
  makeCase("fri-board-missing", "friday", "2026-11-09", "Tablero Fantasma"),
  makeCase("fri-board-all-previous-week", "friday", "2026-11-02", "Todos"),
  makeCase("fri-year-change-1", "friday", "2026-12-28"),
  makeCase("fri-year-change-2", "friday", "2027-01-04"),
  makeCase("fri-week-without-data", "friday", "2026-05-04"),
  // Today is Monday 2026-11-16 (first day of the week)
  makeCase("mon-current-week", "monday", "2026-11-16"),
  makeCase("mon-previous-week", "monday", "2026-11-09"),
  makeCase("mon-two-weeks-ago", "monday", "2026-11-02"),
  makeCase("mon-future-week", "monday", "2026-11-23"),
  makeCase("mon-today-holiday", "monday", "2026-11-16", "Todos", { "2026-11-16": true }),
  makeCase("mon-holiday-removed", "monday", "2026-11-02", "Todos", { "2026-11-03": false }),
  makeCase("mon-current-week-all-holidays", "monday", "2026-11-16", "Todos", ALL_HOLIDAYS("2026-11-16")),
  makeCase("mon-board-missing", "monday", "2026-11-09", "No existe"),
];

/* One case per board that results from the data (the name comes from `resolveBoard`, not from a fixed list) */
function boardNames(dump) {
  const L = crearLegado();
  L.estado({ issues: deepClone(load(dump)), board: "Todos", weekKey: "2026-11-09", feriadosExtra: {}, hoy: DUMPS[dump].today });
  L.prep();
  return [...new Set(L.issues.map(i => i.board))].sort();
}
const slug = s => s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
for (const b of boardNames("friday")) cases.push(makeCase("fri-board-" + slug(b), "friday", "2026-11-09", b));
for (const b of boardNames("monday").slice(0, 2)) cases.push(makeCase("mon-board-" + slug(b), "monday", "2026-11-09", b));

/* ---------- adapter: builds the `View` (contract D) calling only the legacy functions ---------- */
function sortKeys(x) {
  if (Array.isArray(x)) return x.map(sortKeys);
  if (x && typeof x === "object") return Object.fromEntries(Object.keys(x).sort().map(k => [k, sortKeys(x[k])]));
  return x;
}
const sortedObject = o => Object.fromEntries(Object.keys(o).sort().map(k => [k, o[k]]));

function buildView(input) {
  const { issues, week, board, extraHolidays, today } = input;
  const L = crearLegado();
  L.estado({ issues: deepClone(issues), board, weekKey: week, feriadosExtra: extraHolidays, hoy: today });
  L.prep();
  const derived = L.issues;
  const boards = L.boards();
  // Same as `renderChips`: if the board is not in the data, it goes back to "Todos".
  const effective = board === "Todos" || boards.includes(board) ? board : "Todos";
  L.setBoard(effective);

  const s = L.weekStats(week);
  const inOrder = set => derived.filter(i => set.has(i.key)).map(i => i.key); // keys in the order of `issues`
  const stats = {
    secs: s.secs, worked: inOrder(s.worked), perBoard: sortedObject(s.perBoard),
    perDay: Object.fromEntries(Object.keys(s.perDay).sort().map(d => [d, sortedObject(s.perDay[d])])),
    resolved: s.resolved.map(i => i.key), est: s.est, spent: s.spent, withDue: s.withDue, onTime: s.onTime,
  };

  const hs = L.hoursState(week);
  const hours = {
    ...hs,
    prevSecs: L.weekStats(L.addDays(week, -7), () => true).secs,
    workdays: L.laborDays(week).length,
    days: [0, 1, 2, 3, 4].map(k => { const day = L.addDays(week, k); return { day, secs: L.daySecs(day), holiday: L.isFeriado(day) }; }),
    holidays: L.weekHolidays(week),
  };

  const weeks = [];
  for (let k = 11; k >= 0; k--) {
    const wk = L.addDays(week, -7 * k), w = L.weekStats(wk);
    weeks.push({ wk, isoWeek: L.isoWeek(wk).w, secs: w.secs, perBoard: sortedObject(w.perBoard), target: L.weekTarget(wk) });
  }

  const byBoard = boards.map(b => {
    const w = L.weekStats(week, i => i.board === b);
    const open = derived.filter(i => i.board === b && i.cat !== "done");
    const statuses = {}; open.forEach(i => { statuses[i.status] = (statuses[i.status] || 0) + 1; });
    return { board: b, secs: w.secs, worked: w.worked.size, resolved: w.resolved.length, open: open.length, statuses: sortedObject(statuses), last: L.lastActivity(b) };
  });

  return { today, week, board: effective, issues: derived, boards, stats, hours, weeks, byBoard };
}

/* ---------- writing ---------- */
const write = (path, obj) => writeFileSync(path, JSON.stringify(sortKeys(obj), null, 2) + "\n");
mkdirSync(casesDir, { recursive: true });
for (const f of readdirSync(casesDir)) if (/\.(input|expected)\.json$/.test(f)) unlinkSync(join(casesDir, f));
const names = new Set();
for (const c of cases) {
  if (names.has(c.name)) throw new Error("repeated case: " + c.name);
  names.add(c.name);
  const input = { issues: deepClone(load(c.dump)), week: c.week, board: c.board, extraHolidays: c.extraHolidays, today: DUMPS[c.dump].today };
  const before = JSON.stringify(input); // the ViewInput is saved as is, before `prep()`
  const view = buildView(input);
  if (JSON.stringify(input) !== before) throw new Error("the input was mutated: " + c.name);
  write(join(casesDir, c.name + ".input.json"), input);
  write(join(casesDir, c.name + ".expected.json"), view);
}
console.log(`${cases.length} cases written to ${casesDir}`);
