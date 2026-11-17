let SITE = "";
const JQL =
    "assignee = currentUser() OR worklogAuthor = currentUser() ORDER BY updated DESC"; // solo informativo (pie de página)
const WEEK_TARGET = 40,
    DAY_TARGET = 8,
    TZ_OFFSET_H = -5; // Ecuador, sin horario de verano
/* Feriados nacionales de Ecuador (días hábiles de descanso obligatorio). Cada persona puede marcar o desmarcar días con clic en «Horas por día». */
const FERIADOS_BASE = [
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
/* Preferencias del equipo (contrato E): viven en Rust (preferencias.toml), no en el navegador. `feriadosExtra` es lo que dibuja la vista. */
const PREFS_DEFECTO = {
    avisos: { activos: true, horaCierre: "16:30", resumenLunes: true },
    iniciarConWindows: false,
    minimizarABandeja: true,
    feriadosExtra: {},
};
const AVISO_INICIO =
    "No se pudo activar el inicio con Windows; la opción quedó desactivada.";
let prefs = PREFS_DEFECTO,
    feriadosExtra = {},
    prefsBusy = false,
    prefsNotice = "";
function isFeriado(ymd) {
    return ymd in feriadosExtra
        ? feriadosExtra[ymd]
        : FERIADOS_BASE.includes(ymd);
}
function aplicarPrefs(p) {
    prefs = p;
    feriadosExtra = p.feriadosExtra || {};
}
/* Guarda las preferencias completas (Rust las normaliza y devuelve como quedaron) y las aplica. */
async function guardarPrefs(p) {
    aplicarPrefs(await invoke("guardar_preferencias", { preferencias: p }));
}
/* Marca o quita un feriado. No cambia nada en pantalla hasta que Rust confirma; si guardar falla, queda como estaba y se avisa en #notice. */
async function toggleFeriado(ymd) {
    if (prefsBusy) return false;
    const nuevo = { ...feriadosExtra };
    nuevo[ymd] = !isFeriado(ymd);
    if (nuevo[ymd] === FERIADOS_BASE.includes(ymd)) delete nuevo[ymd];
    prefsBusy = true;
    try {
        await guardarPrefs({ ...prefs, feriadosExtra: nuevo });
    } catch (e) {
        prefsNotice = `No se pudo guardar el feriado: ${errText(e)}`;
        showNotice(prefsNotice);
        return false;
    } finally {
        prefsBusy = false;
    }
    if (prefsNotice) {
        if ($("notice").textContent === prefsNotice) showNotice("");
        prefsNotice = "";
    }
    return true;
}
/* Migración única: los feriados que antes se guardaban en localStorage pasan a las preferencias (solo si estas vienen sin feriados). */
async function migrarFeriados() {
    const CLAVE = "spe.feriados",
        quitar = () => {
            try {
                localStorage.removeItem(CLAVE);
            } catch (e) {}
        };
    let raw = null;
    try {
        raw = localStorage.getItem(CLAVE);
    } catch (e) {}
    if (raw === null) return;
    if (Object.keys(feriadosExtra).length) {
        quitar();
        return;
    }
    const viejos = {};
    try {
        const o = JSON.parse(raw);
        if (o && typeof o === "object" && !Array.isArray(o))
            for (const [k, v] of Object.entries(o))
                if (/^\d{4}-\d{2}-\d{2}$/.test(k) && typeof v === "boolean")
                    viejos[k] = v;
    } catch (e) {}
    if (!Object.keys(viejos).length) {
        quitar();
        return;
    }
    try {
        await guardarPrefs({ ...prefs, feriadosExtra: viejos });
        quitar();
    } catch (e) {
        feriadosExtra = viejos;
        console.warn(
            "No se pudieron migrar los feriados guardados en el navegador; se reintentará al abrir:",
            errText(e),
        );
    } /* la clave se conserva para el próximo arranque */
}
async function cargarPrefs() {
    try {
        aplicarPrefs(await invoke("leer_preferencias"));
    } catch (e) {
        console.warn("No se pudieron leer las preferencias:", errText(e));
        return;
    }
    await migrarFeriados();
}
let ME = null;
const BOARD_COLORS = {
    "Tablero Kanban": "var(--b1)",
    "Tablero AFO": "var(--b3)",
    "Tablero DAI": "var(--b2)",
    "Tablero ADN": "var(--b6)",
};
const PALETTE = [
    "var(--b1)",
    "var(--b2)",
    "var(--b3)",
    "var(--b4)",
    "var(--b5)",
    "var(--b6)",
];
function bColor(b) {
    if (b === "Sin tablero") return "var(--b7)";
    if (!BOARD_COLORS[b]) {
        const used = new Set(Object.values(BOARD_COLORS));
        BOARD_COLORS[b] = PALETTE.find((c) => !used.has(c)) || "var(--b7)";
    }
    return BOARD_COLORS[b];
}

let issues = [],
    syncedAt = null,
    source = "none";
const DOC_WINDOW_DAYS = 84; // 12 semanas hacia atrás
let docs = {},
    docState = "idle",
    docError = "",
    prMode = "keywords"; // prMode: "pr" cuando se leyó el panel de desarrollo // docs[key] = url
const SIG_PAGE = 4;
let sigPage = 0,
    sigCount = 0;
const T_PAGE = 20;
let tPage = 0,
    tSig = "";
let collapsed = {};
try {
    collapsed = JSON.parse(localStorage.getItem("spe.collapsed") || "{}") || {};
} catch (e) {}
function applyCollapse() {
    for (const [k, pid] of [
        ["boards", "pBoards"],
        ["signals", "pSignals"],
    ]) {
        const c = !!collapsed[k];
        $(k + "Body").hidden = c;
        $(pid).classList.toggle("collapsed", c);
        document
            .querySelector(`[data-panel="${k}"]`)
            .setAttribute("aria-expanded", String(!c));
    }
}
let view = "week",
    board = "Todos",
    weekKey = null,
    sortKey = "act",
    sortDir = -1;
try {
    const s = localStorage.getItem("spe.board");
    if (s) board = s;
} catch (e) {}

/* ---------- fechas (hora de Ecuador) ---------- */
const DAY = 864e5;
function localDate(iso) {
    const t = new Date(iso).getTime() + TZ_OFFSET_H * 3600e3;
    return new Date(t).toISOString().slice(0, 10);
}
function dateOnly(s) {
    return s ? (s.length > 10 ? localDate(s) : s) : null;
}
function mondayOf(ymd) {
    const d = new Date(ymd + "T00:00:00Z");
    const wd = (d.getUTCDay() + 6) % 7;
    return new Date(d - wd * DAY).toISOString().slice(0, 10);
}
function addDays(ymd, n) {
    return new Date(new Date(ymd + "T00:00:00Z").getTime() + n * DAY)
        .toISOString()
        .slice(0, 10);
}
function isoWeek(ymd) {
    const d = new Date(ymd + "T00:00:00Z");
    const th = new Date(d.getTime() + (3 - ((d.getUTCDay() + 6) % 7)) * DAY);
    const y = th.getUTCFullYear();
    const j1 = new Date(Date.UTC(y, 0, 4));
    return {
        y,
        w:
            1 +
            Math.round(((th - j1) / DAY - 3 + ((j1.getUTCDay() + 6) % 7)) / 7),
    };
}
const MES = [
    "ene",
    "feb",
    "mar",
    "abr",
    "may",
    "jun",
    "jul",
    "ago",
    "sep",
    "oct",
    "nov",
    "dic",
];
const DIA = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];
function fmtD(ymd) {
    if (!ymd) return "—";
    const [y, m, d] = ymd.split("-");
    return `${+d} ${MES[+m - 1]}`;
}
function todayLocal() {
    return localDate(new Date().toISOString());
}
const h = (s) => Math.round(s / 360) / 10; // segundos -> horas con 1 decimal
const fh = (s) => {
    const v = h(s);
    return (v % 1 ? v.toFixed(1) : v) + " h";
};

/* ---------- render ---------- */
const $ = (id) => document.getElementById(id);
const esc = (s) =>
    String(s ?? "").replace(
        /[&<>"]/g,
        (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c],
    );

/* ---------- vista (calculada en Rust) ----------
   `vista` es la última respuesta de `calcular_vista`; `VC` guarda sus derivados de uso rápido (mapas por clave).
   `issues` sigue siendo la lista cruda; todo lo que se dibuja sale de `vista`. */
let vista = null,
    VC = null,
    renderSeq = 0,
    vistaError = "";
const hoyAct = () => (vista ? vista.today : todayLocal());
function setVista(v) {
    const tareas = v.issues,
        byKey = new Map(tareas.map((i) => [i.key, i]));
    const resolved = v.stats.resolved.map((k) => byKey.get(k)).filter(Boolean);
    VC = {
        tareas,
        byKey,
        worked: new Set(v.stats.worked),
        resolved,
        resolvedSet: new Set(v.stats.resolved),
    };
    vista = v;
}
const inBoard = (i) => vista.board === "Todos" || i.board === vista.board;
/* Pide la vista con los datos actuales (sin tocar la pantalla). */
const pedirVista = () =>
    invoke("calcular_vista", {
        entrada: {
            issues,
            week: weekKey,
            board,
            extraHolidays: feriadosExtra,
        },
    });

function renderChips() {
    const list = ["Todos", ...vista.boards];
    $("boardChips").innerHTML = list
        .map(
            (b) =>
                `<button type="button" class="chip" aria-pressed="${b === vista.board}" data-b="${esc(b)}">${b === "Todos" ? "" : `<i style="background:${bColor(b)}"></i>`}${esc(b)}</button>`,
        )
        .join("");
}

function docCandidates(v) {
    const from = addDays(v.today, -DOC_WINDOW_DAYS);
    return v.issues.filter(
        (i) =>
            i.needDoc &&
            ((i.resDay && i.resDay >= from) ||
                (i.cat !== "done" &&
                    (i.finDay || /prueba|producci/i.test(i.status || "")))),
    );
}
function docStatus(i) {
    if (!i.needDoc) return "na";
    if (docs[i.key]) return "ok";
    return docState === "done" && i.docChecked ? "missing" : "unknown";
}
/* Tareas que requieren documento en la semana: concluidas en la semana con PR; en la semana actual, también las abiertas en pruebas o con Fin desarrollo */
function docWeek(wk) {
    const end = addDays(wk, 4),
        current = wk === mondayOf(vista.today);
    return VC.tareas.filter(
        (i) =>
            inBoard(i) &&
            i.needDoc &&
            ((i.resDay && i.resDay >= wk && i.resDay <= end) ||
                (current &&
                    i.cat !== "done" &&
                    (i.finDay || /prueba|producci/i.test(i.status || "")))),
    );
}
function docKpi() {
    const need = docWeek(vista.week);
    const ok = need.filter((i) => docs[i.key]).length;
    const miss = need.filter((i) => docStatus(i) === "missing").length;
    const v =
        docState === "done"
            ? need.length
                ? `${ok}<small> / ${need.length}</small>`
                : "—"
            : docState === "error"
              ? "—"
              : "…";
    const d =
        docState === "error"
            ? "No se pudo revisar Confluence"
            : docState !== "done"
              ? "Revisando Confluence…"
              : !need.length
                ? "sin tareas con PR"
                : miss
                  ? `<span class="over">${miss} sin documento</span>`
                  : "todas con documento";
    return `<button type="button" class="kpi" data-view="nodoc" aria-pressed="${view === "nodoc"}" title="Tareas con PR de la semana que tienen documento de despliegue"><span class="label">Doc. de despliegue</span><span class="v">${v}</span><span class="d">${d}</span><span class="go">Ver lista →</span></button>`;
}
function holidayNote() {
    const H = vista.hours,
        hs = H.holidays;
    return hs.length
        ? `${hs.length} ${hs.length === 1 ? "feriado" : "feriados"} (${hs.map((d) => DIA[(new Date(d + "T00:00:00Z").getUTCDay() + 6) % 7] + " " + fmtD(d)).join(", ")}): meta ${H.targetH} h.`
        : "";
}
function dayName(ymd) {
    return `${DIA[(new Date(ymd + "T00:00:00Z").getUTCDay() + 6) % 7]} ${fmtD(ymd)}`;
}
function renderHoursAlert() {
    const H = vista.hours,
        st = { ...H },
        today = vista.today,
        prevSecs = H.prevSecs;
    if (st.sev === "warn" && !st.behind) st.sev = "info";
    let chip = {
        ok: "Completa",
        crit: st.current ? "Atrasada" : "Incompleta",
        warn: "Por debajo",
        info: "Al día",
        "": st.current ? "Inicio de semana" : "—",
    }[st.sev];
    if (wlState === "loading") {
        st.sev = "";
        chip = "Cargando";
        st.title = "Leyendo tus horas de Jira…";
        st.detail = "Se consultan los registros de horas tarea por tarea.";
    } else if (wlState === "error") {
        st.sev = "crit";
        chip = "Error";
        st.title = "No se pudieron leer tus horas";
        st.detail = wlError;
    }
    const pct = st.target ? Math.min(100, (st.total / st.target) * 100) : 0,
        mark =
            st.current && st.sev !== "ok" && st.target
                ? Math.min(100, (st.expected / st.target) * 100)
                : null;
    let side;
    if (st.current) {
        const off = !st.todayLabor,
            tp = Math.min(100, (st.todaySecs / (DAY_TARGET * 3600)) * 100);
        side = `<div class="blk today"><span class="label">Hoy · ${dayName(today)}</span>
      <div class="fig">${h(st.todaySecs)}<small>${off ? "h · día no laborable" : `de ${DAY_TARGET} h`}</small></div>
      <div class="track2"><i style="width:${off ? 0 : tp}%"></i></div>
      <span class="det">${off ? "No suma a la meta de la semana." : st.todaySecs >= DAY_TARGET * 3600 ? "Jornada completa." : `Faltan ${fh(DAY_TARGET * 3600 - st.todaySecs)} para la jornada.`}</span></div>`;
    } else {
        const n = H.workdays,
            avg = n ? st.total / n : 0;
        side = `<div class="blk today"><span class="label">Promedio por día laborable</span>
      <div class="fig">${h(avg)}<small>de ${DAY_TARGET} h</small></div>
      <div class="track2"><i style="width:${Math.min(100, (avg / (DAY_TARGET * 3600)) * 100)}%"></i></div>
      <span class="det">${n} ${n === 1 ? "día laborable" : "días laborables"} en la semana.</span></div>`;
    }
    const days = H.days
        .map(({ day, secs, holiday: off }) => {
            const cls = [
                off ? "off" : "",
                day === today ? "now" : "",
                day > today ? "fut" : "",
                !off && day < today && !secs ? "zero" : "",
                !off && day < today && secs && secs < DAY_TARGET * 3600
                    ? "low"
                    : "",
            ]
                .filter(Boolean)
                .join(" ");
            return `<div class="day ${cls}"><span>${dayName(day)}${day === today ? " · hoy" : ""}</span><b>${off && !secs ? "Feriado" : fh(secs)}</b>
      <div class="mb"><i style="width:${off ? 0 : Math.min(100, (secs / (DAY_TARGET * 3600)) * 100)}%"></i></div>
      <button type="button" class="tg" data-fer="${day}">${off ? "Quitar feriado" : "Marcar feriado"}</button></div>`;
        })
        .join("");
    $("hoursAlert").className = "hero " + st.sev;
    $("hoursAlert").innerHTML = `<div class="blk">
      <div class="head"><span class="label">Semana · meta ${st.targetH} h</span><span class="chipst ${st.sev}">${chip}</span></div>
      <div class="fig">${h(st.total)}<small>de ${st.targetH} h</small></div>
      <div class="track2" role="img" aria-label="${h(st.total)} de ${st.targetH} horas"><i style="width:${pct}%"></i>${mark !== null && mark > 0 && mark < 100 ? `<u style="left:${mark}%" title="Esperado hasta ayer: ${fh(st.expected)}"></u>` : ""}</div>
      <span class="ttl">${st.title}</span>
      <span class="det">${st.detail} Semana anterior: ${fh(prevSecs)}.${vista.board !== "Todos" ? " Suma de todos los tableros." : ""}</span></div>
    ${side}
    <div class="days" aria-label="Horas por día">${days}</div>`;
}
function renderKpis() {
    const s = vista.stats,
        varPct = s.est ? ((s.spent - s.est) / s.est) * 100 : null;
    const open = VC.tareas.filter((i) => inBoard(i) && i.cat !== "done");
    const overdue = VC.tareas.filter(
        (i) => inBoard(i) && !i.resDay && i.due && i.due < vista.today,
    );
    $("kpis").innerHTML = `
  <button type="button" class="kpi" data-view="worked" aria-pressed="${view === "worked"}"><span class="label">Tareas trabajadas</span><span class="v">${VC.worked.size}</span><span class="d">con horas en la semana</span><span class="go">Ver lista →</span></button>
  <button type="button" class="kpi" data-view="done" aria-pressed="${view === "done"}"><span class="label">Concluidas</span><span class="v">${VC.resolved.length}</span><span class="d">${s.withDue ? `${s.onTime} de ${s.withDue} a tiempo` : "sin vencimiento para medir"}</span><span class="go">Ver lista →</span></button>
  ${docKpi()}
  <div class="kpi"><span class="label">Estimado vs real</span><span class="v">${s.est ? (varPct > 0 ? "+" : "") + varPct.toFixed(0) + " %" : "—"}</span><span class="d">${s.est ? `${fh(s.est)} est. · ${fh(s.spent)} reales` : "sin estimaciones"}</span></div>
  <button type="button" class="kpi" data-view="open" aria-pressed="${view === "open"}"><span class="label">Abiertas ahora</span><span class="v">${open.length}</span><span class="d">${overdue.length ? `<span class="over">${overdue.length} vencidas</span>` : "ninguna vencida"}</span><span class="go">Ver lista →</span></button>`;
}

function stackBars(el, cols, maxV, refV, opts) {
    // cols: [{label, sub, parts:{board:secs}, key, selected}]
    const W = opts.w || 640,
        H = opts.h || 230,
        L = 34,
        R = 8,
        T = 10,
        B = 34;
    const iw = W - L - R,
        ih = H - T - B,
        bw = iw / cols.length;
    const top = Math.max(maxV, refV || 0) || 1;
    const step = top > 40 ? 10 : top > 16 ? 8 : top > 8 ? 4 : 2;
    const niceTop = Math.ceil(top / step) * step;
    const y = (v) => T + ih - (v / niceTop) * ih;
    let g = "";
    for (let v = 0; v <= niceTop; v += step)
        g += `<line class="grid" x1="${L}" x2="${W - R}" y1="${y(v)}" y2="${y(v)}"/><text x="${L - 6}" y="${y(v) + 3.5}" text-anchor="end">${v}</text>`;
    cols.forEach((c, k) => {
        const x = L + k * bw;
        if (c.selected)
            g += `<rect class="sel" x="${x + 2}" y="${T}" width="${bw - 4}" height="${ih}" rx="4"/>`;
        if (c.off)
            g += `<text x="${x + bw / 2}" y="${T + ih / 2}" text-anchor="middle" style="font-style:italic">feriado</text>`;
        if (c.ref !== undefined && c.ref > 0)
            g += `<line class="ref" x1="${x + 2}" x2="${x + bw - 2}" y1="${y(c.ref)}" y2="${y(c.ref)}"/>`;
        let acc = 0;
        const pad = Math.min(10, bw * 0.22);
        for (const b of Object.keys(c.parts).sort()) {
            const v = c.parts[b] / 3600;
            if (!v) continue;
            g += `<rect x="${x + pad}" y="${y(acc + v)}" width="${bw - 2 * pad}" height="${Math.max(0, y(acc) - y(acc + v))}" fill="${bColor(b)}" rx="1.5"><title>${esc(b)}: ${v.toFixed(1)} h</title></rect>`;
            acc += v;
        }
        if (acc)
            g += `<text x="${x + bw / 2}" y="${y(acc) - 4}" text-anchor="middle" style="fill:var(--ink)">${Math.round(acc * 10) / 10}</text>`;
        g += `<text x="${x + bw / 2}" y="${H - B + 14}" text-anchor="middle" ${c.selected ? 'style="fill:var(--ink);font-weight:600"' : ""}>${c.label}</text>`;
        if (c.sub)
            g += `<text x="${x + bw / 2}" y="${H - B + 26}" text-anchor="middle">${c.sub}</text>`;
        if (opts.click)
            g += `<rect class="hit" data-${opts.click}="${c.key}" x="${x}" y="${T}" width="${bw}" height="${ih + B}"><title>${opts.click === "day" ? (c.off ? "Desmarcar feriado" : "Marcar como feriado") : "Ver semana " + c.label}</title></rect>`;
    });
    if (refV)
        g += `<line class="ref" x1="${L}" x2="${W - R}" y1="${y(refV)}" y2="${y(refV)}"/>`;
    el.innerHTML = `<svg viewBox="0 0 ${W} ${H}" width="100%" role="img" aria-label="${opts.aria}">${g}</svg>`;
}

function renderCharts() {
    const sem = vista.week,
        H = vista.hours,
        cols = [];
    let max = 0;
    for (const w of vista.weeks) {
        max = Math.max(max, w.secs / 3600);
        cols.push({
            label: "S" + w.isoWeek,
            sub: fmtD(w.wk),
            parts: w.perBoard,
            key: w.wk,
            selected: w.wk === sem,
            ref: w.target,
        });
    }
    stackBars($("weeklyChart"), cols, Math.max(max, WEEK_TARGET), 0, {
        click: "wk",
        aria: "Horas registradas por semana",
        h: 240,
    });
    const s = vista.stats;
    const dcols = [];
    let dmax = 0;
    for (let d = 0; d < 5; d++) {
        const day = H.days[d].day;
        const parts = s.perDay[day] || {};
        const tot = Object.values(parts).reduce((a, b) => a + b, 0) / 3600;
        dmax = Math.max(dmax, tot);
        const off = H.days[d].holiday;
        dcols.push({
            label: DIA[d],
            sub: fmtD(day).split(" ")[0],
            parts,
            key: day,
            selected: day === vista.today,
            off,
            ref: off ? 0 : DAY_TARGET,
        });
    }
    stackBars($("dailyChart"), dcols, Math.max(dmax, DAY_TARGET), 0, {
        aria: "Horas por día, lunes a viernes",
        h: 250,
        w: 380,
    });
    const bs = vista.board === "Todos" ? vista.boards : [vista.board];
    $("legend1").innerHTML =
        bs
            .map(
                (b) =>
                    `<span><i style="background:${bColor(b)}"></i>${esc(b)}</span>`,
            )
            .join("") +
        `<span>— — meta de la semana (${WEEK_TARGET} h menos ${DAY_TARGET} h por feriado)</span>`;
}

function statusPill(i) {
    const c =
        i.cat === "done" ? "done" : i.cat === "indeterminate" ? "prog" : "todo";
    return `<span class="pill ${c}">${esc(i.status)}</span>`;
}

function renderBoards() {
    const rows = vista.byBoard.filter(
        (r) => vista.board === "Todos" || r.board === vista.board,
    );
    const active = rows.filter((r) => r.secs),
        idle = rows.filter((r) => !r.secs);
    const max = Math.max(1, ...active.map((r) => r.secs));
    /* mayor cantidad primero; a igual cantidad, el estado que aparece antes en las tareas (como antes) */
    const stTxt = (r) => {
        const ord = [];
        for (const i of VC.tareas)
            if (
                i.board === r.board &&
                i.cat !== "done" &&
                !ord.includes(i.status)
            )
                ord.push(i.status);
        return Object.entries(r.statuses)
            .sort(
                (a, b) => b[1] - a[1] || ord.indexOf(a[0]) - ord.indexOf(b[0]),
            )
            .map(([k, v]) => `${k}: ${v}`)
            .join(" · ");
    };
    const openBtn = (r) =>
        r.open
            ? `<button type="button" class="linkbtn" data-open-board="${esc(r.board)}" title="${esc(stTxt(r))}">${r.open} abiertas</button>`
            : `<span class="mini">0 abiertas</span>`;
    const totB = active.reduce((a, r) => a + r.secs, 0);
    $("boardsSum").textContent = collapsed.boards
        ? active.length
            ? `${fh(totB)} en ${active.length} ${active.length === 1 ? "tablero" : "tableros"} esta semana`
            : "Sin horas esta semana"
        : "Semana seleccionada · clic en «abiertas» para ver la lista";
    $("boards").innerHTML =
        (active.length
            ? active
                  .map(
                      (r) =>
                          `<div class="brow"><span class="tag"><i style="background:${bColor(r.board)}"></i>${esc(r.board)}</span><div class="bbar"><div style="width:${(r.secs / max) * 100}%;background:${bColor(r.board)}"></div></div><span class="num">${fh(r.secs)}</span><span class="mini">${r.worked} ${r.worked === 1 ? "tarea" : "tareas"}</span>${openBtn(r)}</div>`,
                  )
                  .join("")
            : `<div class="empty">Sin horas registradas en esta semana</div>`) +
        (idle.length
            ? `<div class="mini bidle">Sin horas esta semana: ${idle.map((r) => `${esc(r.board)} · ${openBtn(r)}`).join("  ·  ")}</div>`
            : "");
}

function renderSignals() {
    const s = vista.stats,
        H = vista.hours,
        issues = VC.tareas,
        weekKey = vista.week;
    const out = [];
    const today = vista.today;
    const tgtW = H.targetH,
        pctW = tgtW ? s.secs / (tgtW * 3600) : 1;
    for (let d = 0; d < 5; d++) {
        const day = H.days[d].day;
        if (day >= today || H.days[d].holiday) continue;
        const tot = Object.values(s.perDay[day] || {}).reduce(
            (a, b) => a + b,
            0,
        );
        if (tot < DAY_TARGET * 3600)
            out.push({
                sev: tot === 0 ? "crit" : "warn",
                t: `${DIA[d]} ${fmtD(day)}: ${fh(tot)} registradas`,
                sub: `Faltan ${fh(DAY_TARGET * 3600 - tot)} para la jornada de ${DAY_TARGET} h`,
            });
    }
    issues
        .filter(
            (i) =>
                inBoard(i) &&
                i.est &&
                i.spent > i.est * 1.2 &&
                (VC.worked.has(i.key) || VC.resolvedSet.has(i.key)),
        )
        .forEach((i) =>
            out.push({
                sev: "warn",
                t: `${i.key} supera su estimación en ${Math.round((i.spent / i.est - 1) * 100)} %`,
                sub: `${fh(i.spent)} reales vs ${fh(i.est)} estimadas · ${esc(i.summary)}`,
            }),
        );
    issues
        .filter((i) => inBoard(i) && !i.resDay && i.due && i.due < today)
        .forEach((i) =>
            out.push({
                sev: "crit",
                t: `${i.key} vencida desde el ${fmtD(i.due)}`,
                sub: `${esc(i.status)} · ${esc(i.summary)}`,
            }),
        );
    issues
        .filter(
            (i) => inBoard(i) && i.cat === "new" && i.start && i.start <= today,
        )
        .forEach((i) =>
            out.push(
                i.start < today
                    ? {
                          sev: "warn",
                          t: `${i.key} debía iniciar el ${fmtD(i.start)} y sigue en «${esc(i.status)}»`,
                          sub: `Muévela a En curso si ya empezaste · ${esc(i.summary)}`,
                      }
                    : {
                          sev: "info",
                          t: `${i.key} inicia hoy y sigue en «${esc(i.status)}»`,
                          sub: `Muévela a En curso cuando empieces · ${esc(i.summary)}`,
                      },
            ),
        );
    issues
        .filter(
            (i) =>
                inBoard(i) &&
                !i.resDay &&
                i.cat === "indeterminate" &&
                i.start &&
                i.start < today &&
                !i.hasWl,
        )
        .forEach((i) =>
            out.push({
                sev: "warn",
                t: `${i.key} debía iniciar el ${fmtD(i.start)} y no tiene horas`,
                sub: `${esc(i.status)} · ${esc(i.summary)}`,
            }),
        );
    {
        const sat = addDays(weekKey, 5),
            sun = addDays(weekKey, 6);
        let wsecs = 0;
        const wk = new Set();
        for (const i of issues) {
            if (!inBoard(i)) continue;
            for (const w of i.worklogs)
                if (w.weekend && (w.day === sat || w.day === sun)) {
                    wsecs += w.s;
                    wk.add(i.key);
                }
        }
        const wres = issues.filter(
            (i) =>
                inBoard(i) &&
                i.resDay &&
                (i.resDay === sat || i.resDay === sun),
        );
        if (wsecs)
            out.push({
                sev: "info",
                t: `${fh(wsecs)} registradas el fin de semana (${fmtD(sat)}–${fmtD(sun)})`,
                sub: `No suman a la semana laboral. Tareas: ${[...wk].join(", ")}`,
            });
        if (wres.length)
            out.push({
                sev: "info",
                t: `${wres.length} ${wres.length === 1 ? "tarea concluida" : "tareas concluidas"} en fin de semana`,
                sub: `No cuentan en la semana laboral: ${wres.map((i) => i.key).join(", ")}`,
            });
    }
    if (docState === "done") {
        const end = addDays(weekKey, 4);
        const inWeek = (i) =>
            i.resDay && i.resDay >= weekKey && i.resDay <= end;
        const miss = issues.filter(
            (i) =>
                inBoard(i) &&
                docStatus(i) === "missing" &&
                (inWeek(i) || i.cat !== "done"),
        );
        if (miss.length)
            out.unshift({
                sev: miss.some(inWeek) ? "crit" : "warn",
                t: `${miss.length} ${miss.length === 1 ? "tarea" : "tareas"} con PR sin documento de despliegue`,
                sub:
                    miss
                        .map(
                            (i) =>
                                `<a href="${SITE}${i.key}" target="_blank" rel="noopener">${i.key}</a> ${esc(i.status)}`,
                        )
                        .join(" · ") +
                    ` · <button type="button" class="linkbtn" data-goview="nodoc">Ver lista</button>`,
            });
    } else if (docState === "error")
        out.push({
            sev: "info",
            t: "No se pudo revisar los documentos de despliegue",
            sub: docError || "Pulsa Actualizar para intentarlo de nuevo",
        });
    VC.resolved
        .filter((i) => i.late > 0)
        .forEach((i) =>
            out.push({
                sev: "warn",
                t: `${i.key} concluida ${i.late} ${i.late === 1 ? "día" : "días"} después del vencimiento`,
                sub: `Vencía el ${fmtD(i.due)}, Fin desarrollo ${fmtD(i.resDay)} · ${esc(i.summary)}`,
            }),
        );
    const noFin = VC.resolved.filter((i) => i.noFin);
    if (noFin.length)
        out.push({
            sev: "info",
            t: `${noFin.length} tareas finalizadas sin Fecha Fin Desarrollo`,
            sub: `Se usa la fecha del último cambio de estado: ${noFin.map((i) => i.key).join(", ")}`,
        });
    issues
        .filter((i) => inBoard(i) && i.wlTotal > i.worklogs.length)
        .forEach((i) =>
            out.push({
                sev: "info",
                t: `${i.key}: solo se leyeron ${i.worklogs.length} de ${i.wlTotal} registros de horas`,
                sub: "Jira limita los registros por tarea en la búsqueda; las horas de esta tarea pueden estar incompletas",
            }),
        );
    if (H.holidays.length)
        out.push({
            sev: "info",
            t: `Semana con ${holidayNote().replace(/\.$/, "")}`,
            sub: `Cada feriado descuenta ${DAY_TARGET} h de la meta de ${WEEK_TARGET} h. Usa «Marcar feriado» o «Quitar feriado» en la franja de días del resumen.`,
        });
    if (!out.length)
        out.push({
            sev: "ok",
            t:
                pctW >= 1
                    ? "Semana completa y sin desviaciones"
                    : "Sin alertas para esta semana",
            sub: `${fh(s.secs)} registradas de ${tgtW} h`,
        });
    const tag = { crit: "ALTA", warn: "MEDIA", info: "INFO", ok: "OK" },
        rank = { crit: 0, warn: 1, info: 2, ok: 3 };
    out.sort(
        (a, b) => rank[a.sev] - rank[b.sev],
    ); /* orden estable: primero las alertas altas */
    const pages = Math.max(1, Math.ceil(out.length / SIG_PAGE));
    if (out.length !== sigCount) {
        sigCount = out.length;
        sigPage = 0;
    }
    sigPage = Math.min(Math.max(0, sigPage), pages - 1);
    const cnt = { crit: 0, warn: 0, info: 0 };
    out.forEach((o) => {
        if (o.sev in cnt) cnt[o.sev]++;
    });
    const sumTxt = [
        cnt.crit && `${cnt.crit} alta${cnt.crit > 1 ? "s" : ""}`,
        cnt.warn && `${cnt.warn} media${cnt.warn > 1 ? "s" : ""}`,
        cnt.info && `${cnt.info} info`,
    ]
        .filter(Boolean)
        .join(" · ");
    $("signalsSum").textContent = sumTxt
        ? `${out.length} ${out.length === 1 ? "señal" : "señales"}: ${sumTxt}`
        : "Sin alertas";
    const slice = out.slice(sigPage * SIG_PAGE, sigPage * SIG_PAGE + SIG_PAGE);
    $("signals").innerHTML = slice
        .map(
            (o) =>
                `<li><span class="sev ${o.sev}">${tag[o.sev]}</span><div><p>${o.t}</p><small>${o.sub}</small></div></li>`,
        )
        .join("");
    const pg = $("sigPager");
    pg.hidden = pages < 2;
    if (pages > 1)
        pg.innerHTML = `<div class="dots" role="group" aria-label="Páginas de señales">${Array.from({ length: pages }, (_, k) => `<button type="button" data-sigpage="${k}" aria-label="Página ${k + 1}" aria-current="${k === sigPage}"></button>`).join("")}</div>
    <div class="pnav"><span>${sigPage * SIG_PAGE + 1}–${Math.min(out.length, (sigPage + 1) * SIG_PAGE)} de ${out.length}</span>
    <button type="button" class="btn" data-sigstep="-1" aria-label="Señales anteriores" ${sigPage === 0 ? "disabled" : ""}>‹</button>
    <button type="button" class="btn" data-sigstep="1" aria-label="Señales siguientes" ${sigPage >= pages - 1 ? "disabled" : ""}>›</button></div>`;
}

function docCell(i) {
    const st = docStatus(i);
    if (st === "ok")
        return `<a href="${esc(docs[i.key])}" target="_blank" rel="noopener" class="pill done" style="text-decoration:none">Ver doc</a>`;
    if (st === "missing") return `<span class="pill miss">Falta</span>`;
    if (st === "na")
        return `<span class="mini" title="${prMode === "pr" ? "La tarea no tiene pull requests vinculados" : "Reunión, capacitación, soporte, análisis o revisión"}">${prMode === "pr" ? "Sin PR · no aplica" : "No aplica"}</span>`;
    return `<span class="mini">${docState === "loading" ? "…" : "—"}</span>`;
}
function renderTable() {
    const issues = VC.tareas,
        weekKey = vista.week,
        board = vista.board,
        hoy = vista.today,
        end = addDays(weekKey, 4);
    const isCurrentWeek = weekKey === mondayOf(hoy);
    const VIEWS = {
        week: {
            title: "Tareas de la semana",
            hint: (n) =>
                `${n} tareas con horas tuyas, concluidas o creadas en la semana · las abiertas sin actividad están en «Abiertas ahora»`,
            f: (i) =>
                VC.worked.has(i.key) ||
                (i.resDay && i.resDay >= weekKey && i.resDay <= end) ||
                (i.creDay && i.creDay >= weekKey && i.creDay <= end),
        },
        worked: {
            title: "Tareas trabajadas en la semana",
            hint: (n) => `${n} tareas con horas registradas`,
            f: (i) => VC.worked.has(i.key),
        },
        done: {
            title: "Concluidas en la semana",
            hint: (n) => `${n} tareas con Fin desarrollo en la semana`,
            f: (i) => i.resDay && i.resDay >= weekKey && i.resDay <= end,
        },
        nodoc: {
            title: "Documentos de despliegue de la semana",
            hint: (n) =>
                `${n} tareas con PR concluidas en la semana${isCurrentWeek ? " o en pruebas" : ""} · primero las que faltan`,
            f: (() => {
                const set = new Set(docWeek(weekKey).map((i) => i.key));
                return (i) => set.has(i.key);
            })(),
        },
        open: {
            title: "Abiertas ahora",
            hint: (n) => `${n} tareas sin finalizar, de cualquier semana`,
            f: (i) => i.cat !== "done",
        },
    };
    const V = VIEWS[view] || VIEWS.week;
    $("tTitle").textContent = V.title;
    $("tReset").hidden = view === "week";
    const rows = issues
        .filter((i) => inBoard(i) && V.f(i))
        .map((i) => {
            const wls = i.worklogs.filter((w) => w.wk === weekKey);
            const lastWl = wls.reduce((m, w) => (w.day > m ? w.day : m), "");
            const inW = (d) => d && d >= weekKey && d <= end;
            return {
                ...i,
                wk: wls.reduce((a, w) => a + w.s, 0),
                act:
                    lastWl ||
                    (inW(i.resDay) ? i.resDay : "") ||
                    (inW(i.creDay) ? i.creDay : "") ||
                    i.resDay ||
                    dateOnly(i.updated) ||
                    "",
            };
        });
    rows.sort((a, b) => {
        const x = a[sortKey] ?? "",
            y = b[sortKey] ?? "";
        return (
            (x > y ? 1 : x < y ? -1 : 0) * sortDir ||
            b.wk - a.wk ||
            (b.act || "").localeCompare(a.act || "") ||
            a.key.localeCompare(b.key)
        );
    });
    if (view === "nodoc") {
        const rank = (i) =>
            ({ missing: 0, unknown: 1, ok: 2, na: 3 })[docStatus(i)];
        rows.sort((a, b) => rank(a) - rank(b));
    }
    document.querySelectorAll("th[data-sort]").forEach((th) => {
        const on = th.dataset.sort === sortKey;
        th.classList.toggle("sorted", on);
        th.classList.toggle("asc", on && sortDir > 0);
        th.setAttribute(
            "aria-sort",
            on ? (sortDir > 0 ? "ascending" : "descending") : "none",
        );
    });
    $("tHint").textContent =
        V.hint(rows.length) + (board !== "Todos" ? ` · tablero ${board}` : "");
    /* Máximo 20 filas por página; se vuelve a la primera al cambiar vista, tablero, semana u orden */
    const sigT = [view, board, weekKey, sortKey, sortDir].join("|");
    if (sigT !== tSig) {
        tSig = sigT;
        tPage = 0;
    }
    const tPages = Math.max(1, Math.ceil(rows.length / T_PAGE));
    tPage = Math.min(Math.max(0, tPage), tPages - 1);
    const allRows = rows;
    const pageRows = rows.slice(tPage * T_PAGE, tPage * T_PAGE + T_PAGE);
    const tp = $("tPager");
    tp.hidden = tPages < 2;
    if (tPages > 1)
        tp.innerHTML = `<div class="dots" role="group" aria-label="Páginas de la tabla">${Array.from({ length: tPages }, (_, k) => `<button type="button" data-tpage="${k}" aria-label="Página ${k + 1}" aria-current="${k === tPage}"></button>`).join("")}</div>
    <div class="pnav"><span>${tPage * T_PAGE + 1}–${Math.min(allRows.length, (tPage + 1) * T_PAGE)} de ${allRows.length}</span>
    <button type="button" class="btn" data-tstep="-1" aria-label="Tareas anteriores" ${tPage === 0 ? "disabled" : ""}>‹</button>
    <button type="button" class="btn" data-tstep="1" aria-label="Tareas siguientes" ${tPage >= tPages - 1 ? "disabled" : ""}>›</button></div>`;
    $("tbody").innerHTML =
        pageRows
            .map(
                (i) => `<tr>
    <td class="num">${i.act ? `${DIA[(new Date(i.act + "T00:00:00Z").getUTCDay() + 6) % 7]} ${fmtD(i.act)}${i.act.slice(0, 4) !== weekKey.slice(0, 4) ? ` <small class="mini">${i.act.slice(0, 4)}</small>` : ""}` : "—"}</td>
    <td><a class="key" href="${SITE}${i.key}" target="_blank" rel="noopener">${i.key}</a></td>
    <td><div class="sum">${esc(i.summary)}</div>${i.sprint ? `<small class="label" style="letter-spacing:.04em">${esc(i.sprint)}</small>` : ""}</td>
    <td><span class="tag" title="Asignado por ${esc(i.boardHow)}"><i style="background:${bColor(i.board)}"></i>${esc(i.board)}</span><br><small class="mini">${esc(i.comp)}</small></td>
    <td>${statusPill(i)}</td>
    <td class="r num">${i.wk ? fh(i.wk) : "—"}</td>
    <td class="r num ${i.est && i.spent > i.est * 1.2 ? "over" : ""}">${fh(i.spent)} / ${i.est ? fh(i.est) : "—"}</td>
    <td class="num">${fmtD(i.start)}</td>
    <td class="num ${!i.resDay && i.due && i.due < hoy ? "over" : ""}">${fmtD(i.due)}</td>
    <td class="num">${i.resDay ? fmtD(i.resDay) + (i.late > 0 ? `<span class="lateb">+${i.late} d</span>` : i.late !== null ? `<span class="okb">a tiempo</span>` : "") + (i.noFin ? `<br><small class="mini">sin Fin desarrollo</small>` : "") : "—"}</td>
    <td>${docCell(i)}</td></tr>`,
            )
            .join("") ||
        `<tr><td colspan="11" class="empty">No hay tareas en esta vista para este tablero.</td></tr>`;
}

function renderTrace() {
    const issues = VC.tareas,
        wl = issues.reduce((a, i) => a + i.worklogs.length, 0);
    const when = new Date(syncedAt);
    $("trace").innerHTML =
        `<div><b>Fuente:</b> ${issues.length} tareas y ${wl} registros de horas de Jira (proyectos ${[...new Set(issues.map((i) => i.projectName))].map(esc).join(", ")}). Tablero = tablero de Jira de donde viene la tarea: el del sprint si tiene sprint; Tablero ADN para el proyecto ADN; Tablero Kanban si cumple su filtro (proyectos GPS, APP, TLP, CTT, RPA y tipo Tarea o Levantamiento); si no, el backlog del tablero de su componente. El componente se muestra aparte. Semana laboral = lunes a viernes, hora de Ecuador; la meta es ${WEEK_TARGET} h menos ${DAY_TARGET} h por cada feriado nacional (puedes marcar o quitar feriados en la franja de días del resumen; se guarda en este equipo). Solo se cuentan tus propios registros de horas. Las horas y cierres de sábado o domingo no suman a la semana y se avisan aparte. Horas = tus propios registros de las últimas 14 semanas, leídos tarea por tarea desde Jira y asignados al día en que se inició el registro. Fechas: Inicio = cuándo debe iniciar, Vence = fecha de entrega, Fin desarrollo = cuándo se concluyó (si falta en una tarea finalizada, se usa su último cambio de estado). Documento de despliegue = página de Confluence titulada «Documento de Despliegue - CLAVE - …», buscada en Confluence y, si aún no aparece en el buscador, en los enlaces de la tarea en Jira; se revisan las tareas concluidas en las últimas 12 semanas y las que están en pruebas. Solo se exige a tareas con pull requests vinculados en Jira (panel de desarrollo); las tareas sin PR, como soporte o reuniones, no aplican. Si no se puede leer el panel de desarrollo, se excluyen por palabra: reuniones, capacitaciones, soporte, análisis y revisiones de tarea.</div>
  <div><b>Consulta:</b> <code>${esc(JQL)}</code></div>
  <div>${source === "cache" ? "Datos guardados en este equipo, pendientes de actualizar" : simulado ? "Datos ficticios del modo simulado" : "Datos en vivo de Jira"} del ${when.toLocaleString("es-EC", { dateStyle: "medium", timeStyle: "short" })}. Los últimos datos cargados se guardan en este equipo para abrir más rápido y se borran al cerrar sesión; la app los actualiza sola cada 15 minutos mientras está abierta. Si la dejas en la bandeja de Windows, te avisa al cierre de la jornada si faltan horas y los lunes si la semana pasada quedó incompleta; puedes ajustarlo en «Preferencias».</div>`;
}

function paint() {
    const iw = isoWeek(vista.week);
    $("wTitle").textContent = `Semana ${iw.w} · ${iw.y}`;
    $("wRange").textContent =
        `${fmtD(vista.week)} – ${fmtD(addDays(vista.week, 4))}`;
    $("nextW").disabled = vista.week >= mondayOf(vista.today);
    $("gate").hidden = true;
    $("main").hidden = false;
    renderChips();
    renderHoursAlert();
    renderKpis();
    renderCharts();
    renderBoards();
    renderSignals();
    renderTable();
    renderTrace();
}
/* Pide la vista a Rust y dibuja. Asíncrono: cada llamada lleva un número y solo la última pinta; mientras llega la respuesta se queda lo que ya
   está en pantalla. Devuelve la vista pedida (aunque otra más nueva la haya reemplazado) o null si falló; en ese caso conserva lo anterior. */
async function render() {
    if (!issues.length) return null;
    const seq = ++renderSeq,
        g = gen;
    let v;
    try {
        v = await pedirVista();
    } catch (e) {
        if (g !== gen || handleAuth(e)) return null;
        vistaError = `No se pudo calcular la vista del tablero: ${errText(e)}`;
        if (seq === renderSeq) showNotice(vistaError);
        return null;
    }
    if (g !== gen || !issues.length) return null;
    if (seq !== renderSeq) return v;
    board =
        v.board; /* si el tablero guardado ya no existe, Rust devuelve «Todos» (como hacía renderChips) */
    if (vistaError) {
        if ($("notice").textContent === vistaError) showNotice("");
        vistaError = "";
    }
    setVista(v);
    paint();
    return v;
}
/* Repinta con la última vista, sin pedir otra (paginadores, orden, vistas de la tabla, paneles). */
function repaint() {
    if (vista) paint();
}

/* ---------- eventos ---------- */
$("prevW").onclick = () => {
    weekKey = addDays(weekKey, -7);
    sigPage = 0;
    render();
};
$("nextW").onclick = () => {
    if (weekKey >= mondayOf(hoyAct())) return;
    weekKey = addDays(weekKey, 7);
    sigPage = 0;
    render();
}; /* con clics rápidos el botón aún no se deshabilitó */
$("todayW").onclick = () => {
    weekKey = mondayOf(hoyAct());
    render();
};
$("boardChips").onclick = (e) => {
    const b = e.target.closest("[data-b]");
    if (!b) return;
    board = b.dataset.b;
    try {
        localStorage.setItem("spe.board", board);
    } catch (err) {}
    render();
};
$("weeklyChart").onclick = (e) => {
    const r = e.target.closest("[data-wk]");
    if (r) {
        weekKey = r.dataset.wk;
        render();
    }
};
$("hoursAlert").onclick = async (e) => {
    const r = e.target.closest("[data-fer]");
    if (r && (await toggleFeriado(r.dataset.fer))) render();
};
document.querySelector("thead").onclick = (e) => {
    const th = e.target.closest("[data-sort]");
    if (!th) return;
    const k = th.dataset.sort;
    if (sortKey === k) sortDir *= -1;
    else {
        sortKey = k;
        sortDir = ["act", "wk", "spent", "resDay", "due", "start"].includes(k)
            ? -1
            : 1;
    }
    repaint();
};

/* ---------- datos: comandos de Tauri (Rust) ---------- */
const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
function setSync(state, text) {
    $("syncDot").className = "dot " + state;
    $("syncText").textContent = text;
}
function showNotice(msg) {
    $("notice").hidden = !msg;
    $("notice").textContent = msg || "";
}
function errText(e) {
    const msg =
        (e && typeof e === "object" ? e.mensaje || e.message : e) ||
        "error desconocido";
    return String(msg).slice(0, 300) + (e?.codigo ? ` [${e.codigo}]` : "");
}
const isAuthErr = (e) =>
    e?.codigo === "no_autorizado" || e?.codigo === "sin_sesion";
const WL_WINDOW_DAYS = 98;
let wlState = "idle",
    wlError = "";
let busy = false,
    busyGen = -1;
let gen = 0; // cambia al cerrar sesión o perder la conexión: descarta respuestas pendientes
let simulado = false,
    lastInfo = null;
/* Caché local y actualización automática */
const CACHE_FORMAT = 2; // si cambia la forma de lo guardado, se sube y la caché vieja se ignora
const REFRESH_MS = 15 * 60e3; // actualización automática cada 15 minutos
let sessionActive = false,
    autoTimer = null,
    lastOkAt = 0,
    lastTryAt = 0;
function validIso(s) {
    return typeof s === "string" && !Number.isNaN(Date.parse(s)) ? s : null;
}
function fmtWhen(iso) {
    return new Date(iso).toLocaleString("es-EC", {
        dateStyle: "medium",
        timeStyle: "short",
    });
}
function stopAuto() {
    if (autoTimer) {
        clearInterval(autoTimer);
        autoTimer = null;
    }
}
function autoRefresh() {
    if (
        !sessionActive ||
        document.visibilityState !== "visible" ||
        (busy && busyGen === gen)
    )
        return;
    refresh(false);
}
function startAuto() {
    stopAuto();
    autoTimer = setInterval(autoRefresh, REFRESH_MS);
}
document.addEventListener("visibilitychange", () => {
    if (
        document.visibilityState === "visible" &&
        sessionActive &&
        Date.now() - lastOkAt > REFRESH_MS &&
        Date.now() - lastTryAt > 60e3
    )
        autoRefresh();
});

/* ---------- pantallas: conexión / carga / tablero ---------- */
function setGate(title, text) {
    $("gateTitle").textContent = title;
    $("gateText").textContent = text;
    $("gate").hidden = false;
}
function showConnect(info, err) {
    gen++;
    renderSeq++;
    vista = null;
    VC = null;
    vistaError = "";
    issues = [];
    docs = {};
    docState = "idle";
    wlState = "idle";
    source = "none";
    ME = null;
    syncedAt = null;
    prMode = "keywords";
    wlError = "";
    docError = "";
    lastOkAt = 0;
    sessionActive = false;
    stopAuto();
    if (info) {
        lastInfo = info;
        simulado = !!info.simulado;
    }
    $("main").hidden = true;
    $("gate").hidden = true;
    $("connect").hidden = false;
    $("refreshBtn").hidden = true;
    $("logoutBtn").hidden = true;
    showNotice("");
    $("who").textContent =
        "Tus horas, tareas y estados de Jira por semana y por tablero";
    $("simTag").hidden = !simulado;
    $("simNote").hidden = !simulado;
    const aviso = info?.aviso || "";
    $("connAviso").hidden = !aviso;
    $("connAviso").textContent = aviso;
    $("connError").hidden = !err;
    $("connError").textContent = err || "";
    $("siteInput").value =
        info?.sitio ||
        $("siteInput").value ||
        "https://torresytorres.atlassian.net";
    if (info?.correo) $("emailInput").value = info.correo;
    $("tokenInput").value = "";
    $("connBtn").disabled = false;
    setSync("", "Sin conexión con Jira");
}
/* Repinta el tablero con lo guardado. Devuelve false (sin tocar nada) si la caché no sirve. */
async function restoreCache(snap) {
    const d = snap && snap.datos;
    if (
        !d ||
        d.formato !== CACHE_FORMAT ||
        !Array.isArray(d.issues) ||
        !d.issues.length ||
        !d.issues.every(
            (i) => i && typeof i.key === "string" && Array.isArray(i.worklogs),
        )
    )
        return false;
    const g = gen;
    try {
        issues = d.issues;
        docs = d.docs && typeof d.docs === "object" ? d.docs : {};
        prMode = d.prMode === "pr" ? "pr" : "keywords";
        syncedAt =
            validIso(d.syncedAt) ||
            validIso(snap.guardado) ||
            new Date().toISOString();
        source = "cache";
        wlState = "done";
        docState = "done";
        wlError = "";
        docError = "";
        const v =
            await render(); /* la vista se calcula en Rust con las tareas guardadas; render() muestra el tablero */
        if (g !== gen) return false;
        if (!v) throw new Error(vistaError || "no se pudo calcular la vista");
        setSync("", `Datos guardados · ${fmtWhen(syncedAt)}`);
        return true;
    } catch (err) {
        if (g !== gen) return false;
        console.warn("La caché local no se pudo usar:", err);
        vista = null;
        VC = null;
        renderSeq++;
        issues = [];
        docs = {};
        prMode = "keywords";
        source = "none";
        wlState = "idle";
        docState = "idle";
        syncedAt = null;
        $("main").hidden = true;
        setGate("Cargando tus tareas…", "Leyendo tus tareas de Jira.");
        return false;
    }
}
async function saveCache(g) {
    try {
        await invoke("guardar_cache", {
            datos: { formato: CACHE_FORMAT, issues, docs, prMode, syncedAt },
        });
    } catch (e) {
        if (g === gen)
            console.warn("No se pudo guardar la caché local:", errText(e));
    }
}
async function enterApp(info) {
    lastInfo = info;
    simulado = !!info.simulado;
    ME = info.usuario?.accountId || null;
    SITE = String(info.sitio || "").replace(/\/+$/, "") + "/browse/";
    $("connect").hidden = true;
    $("connError").hidden = true;
    $("tokenInput").value = "";
    const name = info.usuario?.name;
    if (name)
        $("who").textContent =
            `${name} · horas, tareas y estados de Jira por semana y por tablero`;
    $("refreshBtn").hidden = false;
    $("logoutBtn").hidden = false;
    sessionActive = true;
    startAuto();
    setGate("Cargando tus tareas…", "Leyendo tus tareas de Jira.");
    const g = gen;
    try {
        const snap = await invoke("leer_cache");
        if (g !== gen) return;
        if (snap) await restoreCache(snap);
        if (g !== gen) return;
    } catch (e) {
        if (g !== gen || handleAuth(e)) return;
        console.warn("No se pudo leer la caché local:", errText(e));
    }
    refresh(false);
}
/* Si una llamada de datos devuelve no_autorizado / sin_sesion, se vuelve a la pantalla de conexión. */
function handleAuth(e) {
    if (!isAuthErr(e)) return false;
    showConnect({
        ...(lastInfo || {}),
        conectado: false,
        aviso: e.mensaje || "Tu sesión ya no es válida. Vuelve a conectarte.",
    });
    return true;
}

/* ---------- sincronización con Jira ---------- */
async function fetchWorklogs(quiet) {
    const g = gen;
    if (!quiet) {
        wlState = "loading";
        wlError = "";
        setSync("busy", "Leyendo tus horas…");
    } else setSync("busy", "Actualizando horas…");
    const from = mondayOf(addDays(todayLocal(), -WL_WINDOW_DAYS)),
        after = Date.parse(from + "T00:00:00-05:00");
    let ok = false,
        kept = false;
    try {
        const keys = await invoke("claves_con_horas", { desde: from });
        let r = { byKey: {}, failed: [], error: null };
        if (keys.length)
            r = await invoke("leer_horas", { claves: keys, desde: after });
        if (g !== gen) return false;
        const byKey = r.byKey || {},
            failed = r.failed || [];
        for (const i of issues) {
            const wl = byKey[i.key] || [];
            i.worklogs = wl;
            i.wlTotal = wl.length;
            if (!i.spent) i.spent = wl.reduce((a, w) => a + w.s, 0);
        }
        wlState = "done";
        ok = !failed.length;
        showNotice(
            failed.length
                ? `No se pudieron leer las horas de ${failed.length} ${failed.length === 1 ? "tarea" : "tareas"} (${failed.slice(0, 8).join(", ")}${failed.length > 8 ? "…" : ""})${r.error ? ": " + r.error : ""}. Pulsa Actualizar para reintentar.`
                : "",
        );
    } catch (e) {
        if (g !== gen || handleAuth(e)) return false;
        if (quiet && wlState === "done") {
            kept = true;
            showNotice(
                `No se pudieron actualizar tus horas de Jira: ${errText(e)}. Se conservan las anteriores.`,
            );
        } else {
            wlState = "error";
            wlError = `No se pudieron leer tus horas de Jira: ${errText(e)}`;
            showNotice(wlError);
        }
    }
    const bad = wlState === "error" || kept;
    setSync(
        bad ? "err" : "live",
        (bad ? "Horas sin leer · " : "En vivo · ") +
            new Date().toLocaleTimeString("es-EC", {
                hour: "2-digit",
                minute: "2-digit",
            }),
    );
    await render();
    if (g !== gen) return false;
    return ok;
}
/* quiet: ya hay datos en pantalla; no se pasa a «cargando» y, si falla, se conservan los anteriores. Devuelve true si la consulta terminó bien. */
async function fetchDocs(quiet) {
    const g = gen;
    if (!quiet) {
        docState = "loading";
        render();
    }
    try {
        const prs = new Set(await invoke("claves_con_pr"));
        if (g !== gen) return false;
        for (const i of issues) i.hasPR = prs.has(i.key);
        prMode = "pr";
    } catch (e) {
        if (g !== gen || handleAuth(e)) return false;
        if (quiet && prMode === "pr" && docState === "done")
            return false; /* se conserva lo que ya se sabía del panel de desarrollo */
        prMode =
            "keywords"; /* sin panel de desarrollo: se usa la regla por palabras */
    }
    /* `needDoc` lo calcula Rust a partir de `hasPR`: se pide la vista con las tareas y horas actuales antes de elegir candidatas */
    const v = await render();
    if (g !== gen) return false;
    if (!v) {
        if (quiet && docState === "done") return false;
        docState = "error";
        docError = vistaError || "No se pudo calcular la vista de tareas.";
        return false;
    }
    const cands = docCandidates(v);
    if (!cands.length) {
        docState = "done";
        render();
        return true;
    }
    if (!quiet) {
        docState = "loading";
        docError = "";
        render();
    }
    try {
        const found = await invoke("buscar_documentos", {
            claves: cands.map((i) => i.key),
        });
        if (g !== gen) return false;
        const checked = new Set(cands.map((i) => i.key));
        issues.forEach((i) => {
            if (checked.has(i.key)) i.docChecked = true;
        });
        docs = found || {};
        docState = "done";
    } catch (e) {
        if (g !== gen || handleAuth(e)) return false;
        if (quiet && docState === "done") return false;
        docState = "error";
        docError = errText(e);
        render();
        return false;
    }
    render();
    return true;
}
/* Horas y documentos tras cargar las tareas. Si todo termina bien y completo, guarda la caché. */
async function loadDetails(g, quiet) {
    try {
        await new Promise((r) =>
            setTimeout(r, 0),
        ); /* deja pintar las tareas primero */
        if (g !== gen) return;
        const okW = await fetchWorklogs(quiet);
        if (g !== gen) return;
        const okD = await fetchDocs(quiet);
        if (g !== gen) return;
        await fetchDocs(true);
        if (g !== gen) return;
        if (okW && okD && wlState === "done" && docState === "done") {
            lastOkAt = Date.now();
            await saveCache(g);
        }
    } catch (err) {
        console.error(err);
    }
}
async function refresh(force) {
    if (busy && busyGen === gen) return;
    const g = gen,
        quiet =
            issues.length > 0 &&
            source !==
                "none"; /* con datos en pantalla no se vacía ni se parpadea */
    busy = true;
    busyGen = g;
    lastTryAt = Date.now();
    $("refreshBtn").disabled = true;
    setSync(
        "busy",
        quiet
            ? source === "cache"
                ? `Datos guardados · ${fmtWhen(syncedAt)} · actualizando…`
                : "Actualizando…"
            : "Consultando Jira…",
    );
    try {
        const list = await invoke("cargar_tareas");
        if (g !== gen) return;
        if (quiet) {
            /* se conservan horas y documentos ya leídos hasta que lleguen los nuevos */
            const old = Object.fromEntries(issues.map((i) => [i.key, i]));
            for (const i of list) {
                const o = old[i.key];
                if (!o) continue;
                if (Array.isArray(o.worklogs)) {
                    i.worklogs = o.worklogs;
                    i.wlTotal = o.wlTotal;
                }
                if (o.hasPR !== undefined) i.hasPR = o.hasPR;
                if (o.docChecked) i.docChecked = true;
            }
        } else {
            showNotice("");
            docs = {};
            docState = "idle";
        }
        issues = list;
        syncedAt = new Date().toISOString();
        source = "live";
        if (!issues.length) {
            vista = null;
            VC = null;
            renderSeq++;
            docs = {};
            docState = "idle";
            wlState = "idle";
            $("main").hidden = true;
            setGate(
                "No encontramos tareas tuyas",
                "Jira no devolvió tareas asignadas a ti ni con horas registradas por ti. Si esperabas verlas, revisa que el usuario conectado sea el correcto.",
            );
            setSync(
                "live",
                "En vivo · " +
                    new Date(syncedAt).toLocaleTimeString("es-EC", {
                        hour: "2-digit",
                        minute: "2-digit",
                    }),
            );
            return;
        }
        setSync(
            "live",
            "En vivo · " +
                new Date(syncedAt).toLocaleTimeString("es-EC", {
                    hour: "2-digit",
                    minute: "2-digit",
                }),
        );
        const v = await render();
        if (g !== gen)
            return; /* render() muestra el tablero al llegar la vista */
        if (!v && !vista) {
            setGate(
                "No pudimos calcular el tablero",
                (vistaError || "Error al calcular la vista") +
                    ". Pulsa Actualizar para reintentar.",
            );
            $("main").hidden = true;
            return;
        }
        await loadDetails(g, quiet);
    } catch (e) {
        if (g !== gen || handleAuth(e)) return;
        if (issues.length > 0 && source !== "none") {
            /* hay datos en pantalla: se conservan */
            $("gate").hidden = true;
            $("main").hidden = false;
            if (e?.codigo === "red" || e?.codigo === "limite") {
                showNotice(
                    `Sin conexión con Jira. Mostrando los datos guardados del ${fmtWhen(syncedAt)}.`,
                );
                setSync("err", "Sin conexión con Jira");
            } else {
                showNotice(
                    `No se pudo leer Jira: ${errText(e)}. Pulsa Actualizar para reintentar.`,
                );
                setSync("err", "Última sincronización con error");
            }
        } else {
            $("main").hidden = true;
            setGate(
                "No pudimos leer tu Jira",
                errText(e) + ". Pulsa Actualizar para reintentar.",
            );
            showNotice("");
            setSync("err", "Sin conexión con Jira");
        }
    } finally {
        if (busyGen === g) {
            busy = false;
            $("refreshBtn").disabled = false;
        }
    }
}
$("refreshBtn").onclick = () => refresh(true);
$("kpis").onclick = (e) => {
    const k = e.target.closest("[data-view]");
    if (!k) return;
    view = view === k.dataset.view ? "week" : k.dataset.view;
    if (view === "open") {
        sortKey = "due";
        sortDir = 1;
    } else if (sortKey === "due") {
        sortKey = "act";
        sortDir = -1;
    }
    repaint();
    if (view !== "week")
        $("tableHead").scrollIntoView({
            behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
                ? "auto"
                : "smooth",
            block: "start",
        });
};
$("tReset").onclick = () => {
    view = "week";
    repaint();
};
$("tPager").onclick = (e) => {
    if (!vista) return;
    const d = e.target.closest("[data-tpage]"),
        s = e.target.closest("[data-tstep]");
    if (d) tPage = +d.dataset.tpage;
    else if (s) tPage += +s.dataset.tstep;
    else return;
    renderTable();
    $("tableHead").scrollIntoView({
        behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
            ? "auto"
            : "smooth",
        block: "start",
    });
};
const goTable = () =>
    $("tableHead").scrollIntoView({
        behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
            ? "auto"
            : "smooth",
        block: "start",
    });
$("boards").onclick = (e) => {
    const b = e.target.closest("[data-open-board]");
    if (!b) return;
    board = b.dataset.openBoard;
    view = "open";
    sortKey = "due";
    sortDir = 1;
    render();
    goTable();
};
$("sigPager").onclick = (e) => {
    if (!vista) return;
    const d = e.target.closest("[data-sigpage]"),
        s = e.target.closest("[data-sigstep]");
    if (d) sigPage = +d.dataset.sigpage;
    else if (s) sigPage += +s.dataset.sigstep;
    else return;
    renderSignals();
};
document.querySelectorAll("[data-panel]").forEach(
    (b) =>
        (b.onclick = () => {
            const k = b.dataset.panel;
            collapsed[k] = !collapsed[k];
            try {
                localStorage.setItem(
                    "spe.collapsed",
                    JSON.stringify(collapsed),
                );
            } catch (err) {}
            applyCollapse();
            repaint();
        }),
);
applyCollapse();
$("signals").onclick = (e) => {
    const b = e.target.closest("[data-goview]");
    if (!b) return;
    view = b.dataset.goview;
    repaint();
    goTable();
};

/* ---------- conexión, cierre de sesión y enlaces externos ---------- */
$("connForm").onsubmit = async (ev) => {
    ev.preventDefault();
    const sitio = $("siteInput").value.trim(),
        correo = $("emailInput").value.trim(),
        token = $("tokenInput").value;
    $("tokenInput").value = "";
    $("connError").hidden = true;
    $("connBtn").disabled = true;
    try {
        const info = await invoke("iniciar_sesion", { sitio, correo, token });
        enterApp(info);
    } catch (e) {
        $("connError").textContent = errText(e);
        $("connError").hidden = false;
        $("connBtn").disabled = false;
    }
};
$("logoutBtn").onclick = async () => {
    let info = { ...(lastInfo || {}), conectado: false, aviso: null },
        err = null;
    try {
        info = await invoke("cerrar_sesion");
    } catch (e) {
        err = errText(e);
    }
    showConnect(info, err);
};
/* En la app, target="_blank" no abre el navegador: los enlaces https se abren con el sistema. */
document.addEventListener("click", (e) => {
    const a = e.target.closest && e.target.closest('a[href^="https://"]');
    if (!a) return;
    e.preventDefault();
    try {
        Promise.resolve(window.__TAURI__.opener.openUrl(a.href)).catch(
            () => {},
        );
    } catch (err) {}
});

/* ---------- preferencias: panel ---------- */
const prefsAvisosDependientes = () => {
    $("prefHora").disabled = $("prefLunes").disabled = !$("prefAvisos").checked;
};
function abrirPrefs() {
    $("prefAvisos").checked = prefs.avisos.activos;
    $("prefHora").value = prefs.avisos.horaCierre;
    $("prefLunes").checked = prefs.avisos.resumenLunes;
    $("prefInicio").checked = prefs.iniciarConWindows;
    $("prefBandeja").checked = prefs.minimizarABandeja;
    prefsAvisosDependientes();
    $("prefsSimNote").hidden = !simulado;
    $("prefsError").hidden = true;
    $("prefsSave").disabled = false;
    if (!$("prefsDlg").open) $("prefsDlg").showModal();
}
function errorPrefs(msg) {
    $("prefsError").textContent = msg;
    $("prefsError").hidden = false;
}
$("prefsBtn").onclick = abrirPrefs;
$("prefsCancel").onclick = () => $("prefsDlg").close();
$("prefAvisos").onchange = prefsAvisosDependientes;
$("prefsForm").onsubmit = async (ev) => {
    ev.preventDefault();
    if (prefsBusy) return;
    const activos = $("prefAvisos").checked,
        hora = $("prefHora").value;
    if (activos && !hora) {
        errorPrefs("Indica la hora de cierre de la jornada (HH:MM).");
        return;
    }
    const nuevas = {
        ...prefs,
        avisos: {
            activos,
            horaCierre: hora || prefs.avisos.horaCierre,
            resumenLunes: $("prefLunes").checked,
        },
        iniciarConWindows: $("prefInicio").checked,
        minimizarABandeja: $("prefBandeja").checked,
        feriadosExtra,
    };
    prefsBusy = true;
    $("prefsSave").disabled = true;
    $("prefsError").hidden = true;
    try {
        await guardarPrefs(nuevas);
        if (
            nuevas.iniciarConWindows === true &&
            prefs.iniciarConWindows === false
        ) {
            $("prefInicio").checked = false;
            errorPrefs(AVISO_INICIO + " Los demás cambios se guardaron.");
        } else $("prefsDlg").close();
    } catch (e) {
        errorPrefs(errText(e));
    } finally {
        prefsBusy = false;
        $("prefsSave").disabled = false;
    }
};

/* ---------- eventos de Rust (bandeja y monitor) ---------- */
function escucharEventos() {
    const listen = window.__TAURI__?.event?.listen;
    if (!listen) return;
    const en = (nombre, fn) => {
        try {
            Promise.resolve(listen(nombre, fn)).catch((e) =>
                console.warn(`No se pudo escuchar ${nombre}:`, errText(e)),
            );
        } catch (e) {
            console.warn(`No se pudo escuchar ${nombre}:`, errText(e));
        }
    };
    en("signal://actualizar", () => {
        if (sessionActive && !(busy && busyGen === gen)) refresh(true);
    });
    en("signal://preferencias", async () => {
        const antes = prefs.iniciarConWindows;
        try {
            aplicarPrefs(await invoke("leer_preferencias"));
        } catch (e) {
            console.warn("No se pudieron releer las preferencias:", errText(e));
            return;
        }
        if (antes === true && prefs.iniciarConWindows === false) {
            prefsNotice = AVISO_INICIO;
            showNotice(AVISO_INICIO);
        }
        if ($("prefsDlg").open)
            $("prefInicio").checked =
                prefs.iniciarConWindows; /* lo que Rust cambia por su cuenta; no pisa lo que la persona está editando */
        render(); /* los feriados pueden haber cambiado; sin tareas cargadas no hace nada */
    });
}

/* ---------- arranque ---------- */
weekKey = mondayOf(todayLocal());
setSync("", "Sin datos todavía");
escucharEventos();
(async () => {
    await cargarPrefs(); /* no requiere sesión y no lanza: las preferencias (feriados) deben estar antes de pedir la primera vista */
    try {
        const info = await invoke("restaurar_sesion");
        if (info.conectado) enterApp(info);
        else showConnect(info);
    } catch (e) {
        showConnect(null, `No se pudo iniciar la aplicación: ${errText(e)}`);
    }
})();
