// Copia LITERAL de las funciones y constantes de cálculo de `ui/app.js` (estado de la fase 2), sin DOM.
// Es la «verdad» contra la que se prueba el módulo Rust `dominio`: NO se corrige ni se «mejora» nada aquí.
//
// Únicos cambios respecto del original (todos marcados con «CAMBIO»):
//   1. Las globales del navegador (`issues`, `board`, `weekKey`, `feriadosExtra`) viven dentro de una
//      fábrica y se inyectan con `estado()`.
//   2. `todayLocal()` devuelve el `hoy` inyectado en vez de leer el reloj.
//   3. Se omite todo lo que toca DOM, `localStorage` o presentación (colores, HTML, `toggleFeriado`).
export function crearLegado() {
// CAMBIO: estado inyectable (en el original son globales del navegador)
let issues = [], board = "Todos", weekKey = null, HOY = null;
const WEEK_TARGET = 40, DAY_TARGET = 8, TZ_OFFSET_H = -5; // Ecuador, sin horario de verano
const FERIADOS_BASE = ["2026-01-01","2026-02-16","2026-02-17","2026-04-03","2026-05-01","2026-05-25","2026-08-10","2026-10-09","2026-11-02","2026-11-03","2026-12-25","2027-01-01"];
let feriadosExtra = {};
function isFeriado(ymd){ return ymd in feriadosExtra ? feriadosExtra[ymd] : FERIADOS_BASE.includes(ymd); }
function laborDays(wk){ const out = []; for (let d=0; d<5; d++){ const day = addDays(wk,d); if (!isFeriado(day)) out.push(day); } return out; }
function weekTarget(wk){ return laborDays(wk).length * DAY_TARGET; }
function weekHolidays(wk){ const out = []; for (let d=0; d<5; d++){ const day = addDays(wk,d); if (isFeriado(day)) out.push(day); } return out; }
const BOARD_NAMES = {10:"Tablero Kanban",455:"Tablero AFO",92:"Tablero DAI",14:"Tablero AT",4:"Tablero BA",3:"Tablero PP",12:"Tablero IMP",59:"Tablero RE",23:"Tablero ADN",16:"Kanban-GPS",5:"Tablero GPS",29:"TLP-TRACK",15:"Kanban-TLP",9:"TLP-FENIX-OP",224:"Tickets Tolepu",48:"TLP-FENIX COM",39:"Tablero CTT Kanban",25:"Tablero Integración RPA - Echologistics"};
const KANBAN_PROJECTS = ["GPS","APP","TLP","CTT","RPA"], KANBAN_TYPES = /^(tarea|task|levantamiento)$/i;
const COMP_BOARDS = {"Aforo":"Tablero AFO","DAI":"Tablero DAI","DAI - Release 2.0":"Tablero DAI","Regímenes Especiales":"Tablero RE"};
const PROJECT_BOARDS = {"ADN":"Tablero ADN"};
function resolveBoard(i, byKey, depth=0){
  if (i.sprints && i.sprints.length){ const s = [...i.sprints].sort((a,b)=> (a.end||"").localeCompare(b.end||"") || a.id-b.id).pop();
    return {board: BOARD_NAMES[s.boardId] || (s.name ? s.name.replace(/[_ ]*Sprint.*$/i,"") : `Tablero ${s.boardId}`), how:`sprint «${s.name}»`}; }
  if (PROJECT_BOARDS[i.project]) return {board:PROJECT_BOARDS[i.project], how:`proyecto ${i.project}`};
  if (i.subtask && i.parent && byKey[i.parent] && depth < 2){ const r = resolveBoard(byKey[i.parent], byKey, depth+1); return {board:r.board, how:`tarea padre ${i.parent}`}; }
  if (KANBAN_PROJECTS.includes(i.project) && KANBAN_TYPES.test(i.type||"")) return {board:"Tablero Kanban", how:"filtro del Tablero Kanban"};
  if (COMP_BOARDS[i.comp]) return {board:COMP_BOARDS[i.comp], how:`backlog por componente ${i.comp}`};
  return {board:"Sin tablero", how:"no coincide con ningún filtro conocido"};
}
const NO_DOC_RE = /reuni[oó]n|capacitaci[oó]n|revisi[oó]n de tarea|soporte|an[aá]lisis/i;
const DAY = 864e5;
function localDate(iso){ const t = new Date(iso).getTime() + TZ_OFFSET_H*3600e3; return new Date(t).toISOString().slice(0,10); }
function dateOnly(s){ return s ? (s.length > 10 ? localDate(s) : s) : null; }
function mondayOf(ymd){ const d = new Date(ymd+"T00:00:00Z"); const wd = (d.getUTCDay()+6)%7; return new Date(d - wd*DAY).toISOString().slice(0,10); }
function addDays(ymd,n){ return new Date(new Date(ymd+"T00:00:00Z").getTime()+n*DAY).toISOString().slice(0,10); }
function isoWeek(ymd){ const d = new Date(ymd+"T00:00:00Z"); const th = new Date(d.getTime() + (3-((d.getUTCDay()+6)%7))*DAY); const y = th.getUTCFullYear(); const j1 = new Date(Date.UTC(y,0,4)); return {y, w: 1+Math.round(((th-j1)/DAY - 3 + ((j1.getUTCDay()+6)%7))/7)}; }
const MES = ["ene","feb","mar","abr","may","jun","jul","ago","sep","oct","nov","dic"];
const DIA = ["Lun","Mar","Mié","Jue","Vie","Sáb","Dom"];
function fmtD(ymd){ if(!ymd) return "—"; const [y,m,d] = ymd.split("-"); return `${+d} ${MES[+m-1]}`; }
function todayLocal(){ return HOY; } // CAMBIO: el original es `localDate(new Date().toISOString())`; aquí `hoy` viene del caso
const h = s => Math.round(s/360)/10; // segundos -> horas con 1 decimal
const fh = s => { const v = h(s); return (v % 1 ? v.toFixed(1) : v) + " h"; };
function prep(){
  const byKey = Object.fromEntries(issues.map(i=>[i.key,i]));
  for (const i of issues){ const r = resolveBoard(i, byKey); i.board = r.board; i.boardHow = r.how; for (const w of i.worklogs){ w.day = localDate(w.d); const wd = (new Date(w.day+"T00:00:00Z").getUTCDay()+6)%7; w.weekend = wd >= 5; w.monday = mondayOf(w.day); w.wk = w.weekend ? null : w.monday; } i.finDay = dateOnly(i.finDev); i.resDay = i.finDay || (i.cat==="done" ? dateOnly(i.resolved || i.scd) : null); i.noFin = !i.finDay && i.cat==="done"; i.late = (i.resDay && i.due) ? Math.round((new Date(i.resDay) - new Date(i.due))/DAY) : null; i.hasWl = i.worklogs.length > 0; i.needDoc = i.hasPR !== undefined ? i.hasPR : (!NO_DOC_RE.test(i.summary||"") && i.type !== "Subtarea"); i.creDay = dateOnly(i.created); }
}
function lastActivity(b){ let last = ""; for (const i of issues){ if (i.board!==b) continue;
  for (const w of i.worklogs) if (w.day > last) last = w.day; if (i.resDay && i.resDay > last) last = i.resDay; const u = dateOnly(i.updated); if (u && u > last) last = u; } return last; }
/* Orden: primero el que tiene horas en la semana elegida, luego concluidas en la semana, luego abiertas, luego el trabajado más recientemente */
function boards(){
  const names = [...new Set(issues.map(i=>i.board))];
  const score = {}; for (const b of names){ const s = weekStats(weekKey, i => i.board===b);
    score[b] = [s.secs, s.resolved.length, issues.filter(i=>i.board===b && i.cat!=="done").length, lastActivity(b)]; }
  return names.sort((a,b) => { const x = score[a], y = score[b]; for (let k=0; k<3; k++) if (x[k]!==y[k]) return y[k]-x[k]; return y[3].localeCompare(x[3]) || a.localeCompare(b); });
}
const inBoard = i => board === "Todos" || i.board === board;
function weekStats(wk, filter = inBoard){
  const end = addDays(wk,4); let secs = 0; const worked = new Set(); const perBoard = {}; const perDay = {};
  for (const i of issues){ if(!filter(i)) continue;
    for (const w of i.worklogs){ if (w.wk !== wk) continue; secs += w.s; worked.add(i.key);
      perBoard[i.board] = (perBoard[i.board]||0) + w.s;
      (perDay[w.day] ||= {}); perDay[w.day][i.board] = (perDay[w.day][i.board]||0) + w.s; } }
  const resolved = issues.filter(i => filter(i) && i.resDay && i.resDay >= wk && i.resDay <= end);
  const est = resolved.reduce((a,i)=>a+i.est,0), spent = resolved.reduce((a,i)=>a+i.spent,0);
  const withDue = resolved.filter(i=>i.due); const onTime = withDue.filter(i=>i.resDay <= i.due).length;
  return {secs, worked, perBoard, perDay, resolved, est, spent, withDue:withDue.length, onTime};
}
function daySecs(day, filter = () => true){ let secs = 0; for (const i of issues){ if (!filter(i)) continue; for (const w of i.worklogs) if (w.day === day) secs += w.s; } return secs; }
function hoursState(wk){
  const total = weekStats(wk, () => true).secs, targetH = weekTarget(wk), target = targetH*3600, today = todayLocal(), end = addDays(wk,4);
  const current = today >= wk && today <= end, future = today < wk;
  const workedDays = laborDays(wk).filter(d => d < today).length;
  const todaySecs = current ? daySecs(today) : 0, todayLabor = current && !isFeriado(today) && today <= end;
  const expected = current ? workedDays*DAY_TARGET*3600 : target;
  const missing = Math.max(0, target - total), behind = Math.max(0, expected - total);
 let sev, title, detail;
  if (!targetH){ sev = ""; title = "Semana de feriados"; detail = `No hay días laborables en esta semana.${total ? ` Registraste ${fh(total)}.` : ""}`; }
  else if (future){ sev = ""; title = "Semana futura"; detail = "Todavía no hay horas para esta semana."; }
  else if (total >= target){ sev = "ok"; title = total > target ? `Semana completa: ${fh(total)} registradas (+${fh(total-target)})` : "Semana completa"; detail = (current ? "Ya cumpliste las horas de la semana." : "Cumpliste las horas de esa semana."); }
  else if (!current){ sev = "crit"; title = `Faltaron ${fh(missing)} para completar las ${targetH} h`; detail = "Esa semana ya cerró. Si trabajaste esas horas, regístralas en Jira con su fecha real."; }
  else if (behind >= DAY_TARGET*3600){ sev = "crit"; title = `Te faltan ${fh(missing)} para las ${targetH} h`; detail = `Vas ${fh(behind)} por debajo de lo esperado hasta ayer (${fh(expected)} en ${workedDays} ${workedDays===1?"día":"días"} laborables). Revisa si te falta registrar horas.`; }
  else if (behind > 0){ sev = "warn"; title = `Te faltan ${fh(missing)} para las ${targetH} h`; detail = `Vas ${fh(behind)} por debajo de lo esperado hasta ayer (${fh(expected)}).`; }
  else { sev = "warn"; title = `Te faltan ${fh(missing)} para las ${targetH} h`; detail = (workedDays ? `Vas al día: lo esperado hasta ayer era ${fh(expected)}.` : "Empieza la semana: registra tus horas cada día."); if (!workedDays) sev = ""; }
  return {total, target, targetH, expected, missing, behind, sev, title, detail, current, todaySecs, todayLabor, today};
}
// CAMBIO: acceso al estado y a las funciones desde fuera
return {
  estado(e){ issues = e.issues; board = e.board; weekKey = e.weekKey; feriadosExtra = e.feriadosExtra; HOY = e.hoy; },
  setBoard(b){ board = b; }, // el original lo hace en `renderChips`
  get issues(){ return issues; },
  isFeriado, laborDays, weekTarget, weekHolidays, resolveBoard, localDate, dateOnly, mondayOf, addDays, isoWeek,
  fmtD, h, fh, prep, lastActivity, boards, inBoard, weekStats, daySecs, hoursState,
  WEEK_TARGET, DAY_TARGET,
};
}
