let SITE = "";
const JQL =
    "assignee = currentUser() OR worklogAuthor = currentUser() ORDER BY updated DESC"; // informational only (footer)
const WEEK_TARGET = 40,
    DAY_TARGET = 8,
    TZ_OFFSET_H = -5; // Ecuador, no daylight saving time
/* National holidays of Ecuador (mandatory rest on working days). Each person can mark or unmark days by clicking in «Horas por día». */
const BASE_HOLIDAYS = [
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
/* Team preferences (contract E): they live in Rust (preferences.toml), not in the browser. `extraHolidays` is what the view draws. */
const DEFAULT_PREFS = {
    notifications: {
        enabled: true,
        endOfDayTime: "16:30",
        weeklySummary: true,
    },
    startWithWindows: false,
    minimizeToTray: true,
    extraHolidays: {},
};
const AUTOSTART_NOTICE =
    "No se pudo activar el inicio con Windows; la opción quedó desactivada.";
let prefs = DEFAULT_PREFS,
    extraHolidays = {},
    prefsBusy = false,
    prefsNotice = "";
function isHoliday(ymd) {
    return ymd in extraHolidays
        ? extraHolidays[ymd]
        : BASE_HOLIDAYS.includes(ymd);
}
function applyPrefs(p) {
    prefs = p;
    extraHolidays = p.extraHolidays || {};
}
/* Saves the full preferences (Rust normalizes them and returns them as stored) and applies them. */
async function savePrefs(p) {
    applyPrefs(await invoke("save_preferences", { preferences: p }));
}
/* Marks or unmarks a holiday. Nothing changes on screen until Rust confirms; if saving fails, it stays as it was and #notice shows the error. */
async function toggleHoliday(ymd) {
    if (prefsBusy) return false;
    const next = { ...extraHolidays };
    next[ymd] = !isHoliday(ymd);
    if (next[ymd] === BASE_HOLIDAYS.includes(ymd)) delete next[ymd];
    prefsBusy = true;
    try {
        await savePrefs({ ...prefs, extraHolidays: next });
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
/* One-time migration: holidays that used to be kept in localStorage move to the preferences (only if those come without holidays). */
async function migrateHolidays() {
    const KEY = "spe.feriados",
        remove = () => {
            try {
                localStorage.removeItem(KEY);
            } catch (e) {}
        };
    let raw = null;
    try {
        raw = localStorage.getItem(KEY);
    } catch (e) {}
    if (raw === null) return;
    if (Object.keys(extraHolidays).length) {
        remove();
        return;
    }
    const legacy = {};
    try {
        const o = JSON.parse(raw);
        if (o && typeof o === "object" && !Array.isArray(o))
            for (const [k, v] of Object.entries(o))
                if (/^\d{4}-\d{2}-\d{2}$/.test(k) && typeof v === "boolean")
                    legacy[k] = v;
    } catch (e) {}
    if (!Object.keys(legacy).length) {
        remove();
        return;
    }
    try {
        await savePrefs({ ...prefs, extraHolidays: legacy });
        remove();
    } catch (e) {
        extraHolidays = legacy;
        console.warn(
            "Could not migrate the holidays saved in the browser; will retry on next start:",
            errText(e),
        );
    } /* the key is kept for the next startup */
}
async function loadPrefs() {
    try {
        applyPrefs(await invoke("read_preferences"));
    } catch (e) {
        console.warn("Could not read preferences:", errText(e));
        return;
    }
    await migrateHolidays();
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
const DOC_WINDOW_DAYS = 84; // 12 weeks back
let docs = {},
    docState = "idle",
    docError = "",
    prMode = "keywords"; // prMode: "pr" when the development panel was read // docs[key] = url
let view = "week",
    board = "Todos",
    weekKey = null,
    sortKey = "act",
    sortDir = -1;
try {
    const s = localStorage.getItem("spe.board");
    if (s) board = s;
} catch (e) {}
const TABS = ["resumen", "tareas", "senales", "despliegues"];
let tab = "resumen",
    signalCounts = { crit: 0, warn: 0 },
    weekTaskCount = 0;
try {
    const s = localStorage.getItem("spe.tab");
    if (TABS.includes(s)) tab = s;
} catch (e) {}

/* ---------- dates (Ecuador time) ---------- */
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
const MONTHS = [
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
const WEEKDAYS = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];
function fmtD(ymd) {
    if (!ymd) return "—";
    const [y, m, d] = ymd.split("-");
    return `${+d} ${MONTHS[+m - 1]}`;
}
function todayLocal() {
    return localDate(new Date().toISOString());
}
const h = (s) => Math.round(s / 360) / 10; // seconds -> hours with 1 decimal
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

/* ---------- list component ---------- */
const listStates = {};
const normalizeText = (s) =>
    String(s ?? "")
        .normalize("NFD")
        .replace(/[̀-ͯ]/g, "")
        .toLowerCase();
const stripTags = (s) =>
    String(s ?? "")
        .replace(/<[^>]*>/g, " ")
        .replace(/&quot;/g, '"')
        .replace(/&lt;/g, "<")
        .replace(/&gt;/g, ">")
        .replace(/&amp;/g, "&");
function readListSize(id, sizes, fallback) {
    try {
        const n = Number(localStorage.getItem(`spe.list.${id}.size`));
        if (sizes.includes(n)) return n;
    } catch (e) {}
    return fallback;
}
function buildListBar(id, st) {
    const cfg = st.cfg;
    const tools = $(id + "Tools"),
        pager = $(id + "Pager");
    tools.innerHTML = `<label class="lsearch"><svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true" focusable="false"><circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M10.5 10.5 14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg><input type="search" placeholder="${esc(cfg.searchLabel)}" aria-label="${esc(cfg.searchLabel)}" autocomplete="off" spellcheck="false"></label>
    <div class="lchips" role="group" aria-label="Filtrar"></div>
    <label class="lsize">Filas <select aria-label="Filas por página">${cfg.sizes.map((n) => `<option value="${n}">${n}</option>`).join("")}</select></label>`;
    st.chipsEl = tools.querySelector(".lchips");
    st.sizeEl = tools.querySelector("select");
    const redraw = () => renderList(id, st.cfg);
    tools.addEventListener("input", (e) => {
        if (!e.target.matches("input")) return;
        st.q = e.target.value;
        st.page = 0;
        redraw();
    });
    tools.addEventListener("click", (e) => {
        const c = e.target.closest("[data-lfilter]");
        if (!c || c.dataset.lfilter === st.filter) return;
        st.filter = c.dataset.lfilter;
        st.page = 0;
        redraw();
    });
    tools.addEventListener("change", (e) => {
        if (!e.target.matches("select")) return;
        st.size = Number(e.target.value);
        st.page = 0;
        try {
            localStorage.setItem(`spe.list.${id}.size`, String(st.size));
        } catch (err) {}
        redraw();
    });
    pager.addEventListener("click", (e) => {
        const d = e.target.closest("[data-lpage]"),
            s = e.target.closest("[data-lstep]");
        if (d) st.page = Number(d.dataset.lpage);
        else if (s) st.page += Number(s.dataset.lstep);
        else return;
        redraw();
        if (st.cfg.onPage) st.cfg.onPage();
    });
}
function renderList(id, cfg) {
    let st = listStates[id];
    if (!st) {
        st = listStates[id] = {
            q: "",
            filter: "all",
            page: 0,
            size: readListSize(id, cfg.sizes, cfg.defaultSize),
            resetKey: null,
            cfg,
        };
        buildListBar(id, st);
    }
    st.cfg = cfg;
    if (!cfg.sizes.includes(st.size)) st.size = cfg.defaultSize;
    const keyChanged = cfg.resetKey !== st.resetKey;
    if (keyChanged) {
        st.resetKey = cfg.resetKey;
        st.page = 0;
    }
    const nq = normalizeText(st.q.trim());
    const searched = nq
        ? cfg.items.filter((i) => normalizeText(cfg.text(i)).includes(nq))
        : cfg.items;
    let active = cfg.filters.find((f) => f.id === st.filter);
    if (
        st.filter !== "all" &&
        (!active ||
            !cfg.items.length ||
            (keyChanged && !searched.some(active.test)))
    ) {
        st.filter = "all";
        st.page = 0;
        active = undefined;
    }
    const filtered = active ? searched.filter(active.test) : searched;
    const chips = [
        { id: "all", label: "Todas", n: searched.length },
        ...cfg.filters.map((f) => ({
            id: f.id,
            label: f.label,
            n: searched.filter(f.test).length,
        })),
    ].filter((c) => c.id === "all" || c.id === st.filter || c.n > 0);
    st.chipsEl.innerHTML = chips
        .map(
            (c) =>
                `<button type="button" class="chip" aria-pressed="${c.id === st.filter}" data-lfilter="${c.id}">${esc(c.label)} <span class="n">${c.n}</span></button>`,
        )
        .join("");
    st.sizeEl.value = String(st.size);
    const pages = Math.max(1, Math.ceil(filtered.length / st.size));
    st.page = Math.min(Math.max(0, st.page), pages - 1);
    const from = st.page * st.size;
    cfg.renderBody(filtered.slice(from, from + st.size), {
        empty: cfg.items.length
            ? "Nada coincide con la búsqueda o el filtro."
            : cfg.emptyText,
    });
    const pg = $(id + "Pager"),
        L = cfg.pagerLabels;
    pg.hidden = pages < 2;
    pg.innerHTML =
        pages < 2
            ? ""
            : `<div class="dots" role="group" aria-label="${L.group}">${Array.from({ length: pages }, (_, k) => `<button type="button" data-lpage="${k}" aria-label="Página ${k + 1}" aria-current="${k === st.page}"></button>`).join("")}</div>
    <div class="pnav"><span>${from + 1}–${Math.min(filtered.length, from + st.size)} de ${filtered.length}</span>
    <button type="button" class="btn" data-lstep="-1" aria-label="${L.prev}" ${st.page === 0 ? "disabled" : ""}>‹</button>
    <button type="button" class="btn" data-lstep="1" aria-label="${L.next}" ${st.page >= pages - 1 ? "disabled" : ""}>›</button></div>`;
}

/* ---------- view (computed in Rust) ----------
   `viewData` is the latest `compute_view` response; `VC` keeps its quick-access derivatives (maps by key).
   `issues` is still the raw list; everything drawn comes from `viewData`. */
let viewData = null,
    VC = null,
    renderSeq = 0,
    viewError = "";
const currentToday = () => (viewData ? viewData.today : todayLocal());
function setView(v) {
    const list = v.issues,
        byKey = new Map(list.map((i) => [i.key, i]));
    const resolved = v.stats.resolved.map((k) => byKey.get(k)).filter(Boolean);
    VC = {
        issues: list,
        byKey,
        worked: new Set(v.stats.worked),
        resolved,
        resolvedSet: new Set(v.stats.resolved),
    };
    viewData = v;
}
const inBoard = (i) => viewData.board === "Todos" || i.board === viewData.board;
/* Requests the view with the current data (without touching the screen). */
const requestView = () =>
    invoke("compute_view", {
        input: {
            issues,
            week: weekKey,
            board,
            extraHolidays,
        },
    });

function renderChips() {
    const list = ["Todos", ...viewData.boards];
    $("boardChips").innerHTML = list
        .map(
            (b) =>
                `<button type="button" class="chip" aria-pressed="${b === viewData.board}" data-b="${esc(b)}">${b === "Todos" ? "" : `<i style="background:${bColor(b)}"></i>`}${esc(b)}</button>`,
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
/* Issues that need a document in the week: resolved in the week with a PR; in the current week, also open ones in testing or with Fin desarrollo */
function docWeek(wk) {
    const end = addDays(wk, 4),
        current = wk === mondayOf(viewData.today);
    return VC.issues.filter(
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
    const need = docWeek(viewData.week);
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
    return `<button type="button" class="kpi" data-view="nodoc" title="Tareas con PR de la semana que tienen documento de despliegue"><span class="label">Doc. de despliegue</span><span class="v">${v}</span><span class="d">${d}</span><span class="go">Ver lista →</span></button>`;
}
function holidayNote() {
    const H = viewData.hours,
        hs = H.holidays;
    return hs.length
        ? `${hs.length} ${hs.length === 1 ? "feriado" : "feriados"} (${hs.map((d) => WEEKDAYS[(new Date(d + "T00:00:00Z").getUTCDay() + 6) % 7] + " " + fmtD(d)).join(", ")}): meta ${H.targetH} h.`
        : "";
}
function dayName(ymd) {
    return `${WEEKDAYS[(new Date(ymd + "T00:00:00Z").getUTCDay() + 6) % 7]} ${fmtD(ymd)}`;
}
function renderHoursAlert() {
    const H = viewData.hours,
        st = { ...H },
        today = viewData.today,
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
      <button type="button" class="tg" data-holiday="${day}">${off ? "Quitar feriado" : "Marcar feriado"}</button></div>`;
        })
        .join("");
    $("hoursAlert").className = "hero " + st.sev;
    $("hoursAlert").innerHTML = `<div class="blk">
      <div class="head"><span class="label">Semana · meta ${st.targetH} h</span><span class="chipst ${st.sev}">${chip}</span></div>
      <div class="fig">${h(st.total)}<small>de ${st.targetH} h</small></div>
      <div class="track2" role="img" aria-label="${h(st.total)} de ${st.targetH} horas"><i style="width:${pct}%"></i>${mark !== null && mark > 0 && mark < 100 ? `<u style="left:${mark}%" title="Esperado hasta ayer: ${fh(st.expected)}"></u>` : ""}</div>
      <span class="ttl">${st.title}</span>
      <span class="det">${st.detail} Semana anterior: ${fh(prevSecs)}.${viewData.board !== "Todos" ? " Suma de todos los tableros." : ""}</span></div>
    ${side}
    <div class="days" aria-label="Horas por día">${days}</div>`;
}
function renderKpis() {
    const s = viewData.stats,
        varPct = s.est ? ((s.spent - s.est) / s.est) * 100 : null;
    const open = VC.issues.filter((i) => inBoard(i) && i.cat !== "done");
    const overdue = VC.issues.filter(
        (i) => inBoard(i) && !i.resDay && i.due && i.due < viewData.today,
    );
    $("kpis").innerHTML = `
  <button type="button" class="kpi" data-view="worked"><span class="label">Tareas trabajadas</span><span class="v">${VC.worked.size}</span><span class="d">con horas en la semana</span><span class="go">Ver lista →</span></button>
  <button type="button" class="kpi" data-view="done"><span class="label">Concluidas</span><span class="v">${VC.resolved.length}</span><span class="d">${s.withDue ? `${s.onTime} de ${s.withDue} a tiempo` : "sin vencimiento para medir"}</span><span class="go">Ver lista →</span></button>
  ${docKpi()}
  <div class="kpi"><span class="label">Estimado vs real</span><span class="v">${s.est ? (varPct > 0 ? "+" : "") + varPct.toFixed(0) + " %" : "—"}</span><span class="d">${s.est ? `${fh(s.est)} est. · ${fh(s.spent)} reales` : "sin estimaciones"}</span></div>
  <button type="button" class="kpi" data-view="open"><span class="label">Abiertas ahora</span><span class="v">${open.length}</span><span class="d">${overdue.length ? `<span class="over">${overdue.length} vencidas</span>` : "ninguna vencida"}</span><span class="go">Ver lista →</span></button>`;
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
    const week = viewData.week,
        H = viewData.hours,
        cols = [];
    let max = 0;
    for (const w of viewData.weeks) {
        max = Math.max(max, w.secs / 3600);
        cols.push({
            label: "S" + w.isoWeek,
            sub: fmtD(w.wk),
            parts: w.perBoard,
            key: w.wk,
            selected: w.wk === week,
            ref: w.target,
        });
    }
    stackBars($("weeklyChart"), cols, Math.max(max, WEEK_TARGET), 0, {
        click: "wk",
        aria: "Horas registradas por semana",
        h: 240,
    });
    const s = viewData.stats;
    const dcols = [];
    let dmax = 0;
    for (let d = 0; d < 5; d++) {
        const day = H.days[d].day;
        const parts = s.perDay[day] || {};
        const tot = Object.values(parts).reduce((a, b) => a + b, 0) / 3600;
        dmax = Math.max(dmax, tot);
        const off = H.days[d].holiday;
        dcols.push({
            label: WEEKDAYS[d],
            sub: fmtD(day).split(" ")[0],
            parts,
            key: day,
            selected: day === viewData.today,
            off,
            ref: off ? 0 : DAY_TARGET,
        });
    }
    stackBars($("dailyChart"), dcols, Math.max(dmax, DAY_TARGET), 0, {
        aria: "Horas por día, lunes a viernes",
        h: 250,
        w: 380,
    });
    const bs = viewData.board === "Todos" ? viewData.boards : [viewData.board];
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
    const rows = viewData.byBoard.filter(
        (r) => viewData.board === "Todos" || r.board === viewData.board,
    );
    const active = rows.filter((r) => r.secs),
        idle = rows.filter((r) => !r.secs);
    const max = Math.max(1, ...active.map((r) => r.secs));
    /* highest count first; on ties, the status that appears first in the issues (as before) */
    const stTxt = (r) => {
        const ord = [];
        for (const i of VC.issues)
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
    const s = viewData.stats,
        H = viewData.hours,
        issues = VC.issues,
        weekKey = viewData.week;
    const out = [];
    const today = viewData.today;
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
                t: `${WEEKDAYS[d]} ${fmtD(day)}: ${fh(tot)} registradas`,
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
    ); /* stable order: high alerts first */
    const cnt = { crit: 0, warn: 0, info: 0 };
    out.forEach((o) => {
        if (o.sev in cnt) cnt[o.sev]++;
    });
    signalCounts = { crit: cnt.crit, warn: cnt.warn };
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
    renderList("sig", {
        items: out,
        filters: [
            { id: "crit", label: "Altas", test: (o) => o.sev === "crit" },
            { id: "warn", label: "Medias", test: (o) => o.sev === "warn" },
            { id: "info", label: "Info", test: (o) => o.sev === "info" },
        ],
        text: (o) => `${o.t} ${stripTags(o.sub)}`,
        sizes: [4, 8, 20],
        defaultSize: 4,
        resetKey: [board, weekKey, out.length].join("|"),
        searchLabel: "Buscar en las señales",
        emptyText: "Sin señales",
        pagerLabels: {
            group: "Páginas de señales",
            prev: "Señales anteriores",
            next: "Señales siguientes",
        },
        renderBody: (pageItems, ctx) => {
            $("signals").innerHTML =
                pageItems
                    .map(
                        (o) =>
                            `<li><span class="sev ${o.sev}">${tag[o.sev]}</span><div><p>${o.t}</p><small>${o.sub}</small></div></li>`,
                    )
                    .join("") || `<li>${ctx.empty}</li>`;
        },
    });
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
const docHint = (n) =>
    `${n} tareas con PR concluidas en la semana${viewData.week === mondayOf(viewData.today) ? " o en pruebas" : ""} · primero las que faltan`;
const boardSuffix = () =>
    viewData.board !== "Todos" ? ` · tablero ${viewData.board}` : "";
const summaryCell = (i) =>
    `<td><div class="sum">${esc(i.summary)}</div>${i.sprint ? `<small class="label" style="letter-spacing:.04em">${esc(i.sprint)}</small>` : ""}</td>`;
const boardCell = (i) =>
    `<td><span class="tag" title="Asignado por ${esc(i.boardHow)}"><i style="background:${bColor(i.board)}"></i>${esc(i.board)}</span><br><small class="mini">${esc(i.comp)}</small></td>`;
const resDayCell = (i) =>
    `<td class="num">${i.resDay ? fmtD(i.resDay) + (i.late > 0 ? `<span class="lateb">+${i.late} d</span>` : i.late !== null ? `<span class="okb">a tiempo</span>` : "") + (i.noFin ? `<br><small class="mini">sin Fin desarrollo</small>` : "") : "—"}</td>`;
const SEG_VIEWS = [
    ["week", "Semana"],
    ["worked", "Trabajadas"],
    ["done", "Concluidas"],
    ["open", "Abiertas"],
];
function renderDocs() {
    const weekKey = viewData.week,
        rank = (i) => ({ missing: 0, unknown: 1, ok: 2, na: 3 })[docStatus(i)];
    const items = docWeek(weekKey).sort(
        (a, b) => rank(a) - rank(b) || a.key.localeCompare(b.key),
    );
    $("dHint").textContent = docHint(items.length) + boardSuffix();
    renderList("d", {
        items,
        filters: [
            ["missing", "Faltan"],
            ["ok", "Con documento"],
            ["na", "No aplica"],
            ["unknown", "Sin revisar"],
        ].map(([id, label]) => ({
            id,
            label,
            test: (i) => docStatus(i) === id,
        })),
        text: (i) => `${i.key} ${i.summary}`,
        sizes: [10, 20, 50],
        defaultSize: 20,
        resetKey: [viewData.board, weekKey, docState].join("|"),
        searchLabel: "Buscar por clave o resumen",
        emptyText:
            "No hay tareas con PR para revisar en esta semana y este tablero.",
        pagerLabels: {
            group: "Páginas de despliegues",
            prev: "Despliegues anteriores",
            next: "Despliegues siguientes",
        },
        onPage: goDocs,
        renderBody: (pageRows, ctx) => {
            $("dbody").innerHTML =
                pageRows
                    .map(
                        (i) => `<tr>
    <td><a class="key" href="${SITE}${i.key}" target="_blank" rel="noopener">${i.key}</a></td>
    ${summaryCell(i)}
    ${boardCell(i)}
    <td>${statusPill(i)}</td>
    ${resDayCell(i)}
    <td>${docCell(i)}</td></tr>`,
                    )
                    .join("") ||
                `<tr><td colspan="6" class="empty">${ctx.empty}</td></tr>`;
        },
    });
}
function renderTable() {
    const issues = VC.issues,
        weekKey = viewData.week,
        board = viewData.board,
        today = viewData.today,
        end = addDays(weekKey, 4);
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
        open: {
            title: "Abiertas ahora",
            hint: (n) => `${n} tareas sin finalizar, de cualquier semana`,
            f: (i) => i.cat !== "done",
        },
    };
    const V = VIEWS[view] || VIEWS.week;
    $("tTitle").textContent = V.title;
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
    document.querySelectorAll("th[data-sort]").forEach((th) => {
        const on = th.dataset.sort === sortKey;
        th.classList.toggle("sorted", on);
        th.classList.toggle("asc", on && sortDir > 0);
        th.setAttribute(
            "aria-sort",
            on ? (sortDir > 0 ? "ascending" : "descending") : "none",
        );
    });
    $("tHint").textContent = V.hint(rows.length) + boardSuffix();
    const viewName = VIEWS[view] ? view : "week";
    weekTaskCount = issues.filter((i) => inBoard(i) && VIEWS.week.f(i)).length;
    $("viewSeg").innerHTML = SEG_VIEWS.map(
        ([id, label]) =>
            `<button type="button" class="segb" data-seg="${id}" aria-pressed="${id === viewName}">${label} <span class="n">${issues.filter((i) => inBoard(i) && VIEWS[id].f(i)).length}</span></button>`,
    ).join("");
    const catFilters = [
        ["new", "Por hacer"],
        ["indeterminate", "En curso"],
        ["done", "Concluidas"],
    ].map(([id, label]) => ({ id, label, test: (i) => i.cat === id }));
    renderList("t", {
        items: rows,
        filters: catFilters,
        text: (i) => `${i.key} ${i.summary}`,
        sizes: [10, 20, 50],
        defaultSize: 20,
        resetKey: [view, board, weekKey, sortKey, sortDir].join("|"),
        searchLabel: "Buscar por clave o resumen",
        emptyText: "No hay tareas en esta vista para este tablero.",
        pagerLabels: {
            group: "Páginas de la tabla",
            prev: "Tareas anteriores",
            next: "Tareas siguientes",
        },
        onPage: goTable,
        renderBody: (pageRows, ctx) => {
            $("tbody").innerHTML =
                pageRows
                    .map(
                        (i) => `<tr>
    <td class="num">${i.act ? `${WEEKDAYS[(new Date(i.act + "T00:00:00Z").getUTCDay() + 6) % 7]} ${fmtD(i.act)}${i.act.slice(0, 4) !== weekKey.slice(0, 4) ? ` <small class="mini">${i.act.slice(0, 4)}</small>` : ""}` : "—"}</td>
    <td><a class="key" href="${SITE}${i.key}" target="_blank" rel="noopener">${i.key}</a></td>
    ${summaryCell(i)}
    ${boardCell(i)}
    <td>${statusPill(i)}</td>
    <td class="r num">${i.wk ? fh(i.wk) : "—"}</td>
    <td class="r num ${i.est && i.spent > i.est * 1.2 ? "over" : ""}">${fh(i.spent)} / ${i.est ? fh(i.est) : "—"}</td>
    <td class="num">${fmtD(i.start)}</td>
    <td class="num ${!i.resDay && i.due && i.due < today ? "over" : ""}">${fmtD(i.due)}</td>
    ${resDayCell(i)}
    <td>${docCell(i)}</td></tr>`,
                    )
                    .join("") ||
                `<tr><td colspan="11" class="empty">${ctx.empty}</td></tr>`;
        },
    });
}

function renderTrace() {
    const issues = VC.issues,
        wl = issues.reduce((a, i) => a + i.worklogs.length, 0);
    const when = new Date(syncedAt);
    $("trace").innerHTML =
        `<div><b>Fuente:</b> ${issues.length} tareas y ${wl} registros de horas de Jira (proyectos ${[...new Set(issues.map((i) => i.projectName))].map(esc).join(", ")}). Tablero = tablero de Jira de donde viene la tarea: el del sprint si tiene sprint; Tablero ADN para el proyecto ADN; Tablero Kanban si cumple su filtro (proyectos GPS, APP, TLP, CTT, RPA y tipo Tarea o Levantamiento); si no, el backlog del tablero de su componente. El componente se muestra aparte. Semana laboral = lunes a viernes, hora de Ecuador; la meta es ${WEEK_TARGET} h menos ${DAY_TARGET} h por cada feriado nacional (puedes marcar o quitar feriados en la franja de días del resumen; se guarda en este equipo). Solo se cuentan tus propios registros de horas. Las horas y cierres de sábado o domingo no suman a la semana y se avisan aparte. Horas = tus propios registros de las últimas 14 semanas, leídos tarea por tarea desde Jira y asignados al día en que se inició el registro. Fechas: Inicio = cuándo debe iniciar, Vence = fecha de entrega, Fin desarrollo = cuándo se concluyó (si falta en una tarea finalizada, se usa su último cambio de estado). Documento de despliegue = página de Confluence titulada «Documento de Despliegue - CLAVE - …», buscada en Confluence y, si aún no aparece en el buscador, en los enlaces de la tarea en Jira; se revisan las tareas concluidas en las últimas 12 semanas y las que están en pruebas. Solo se exige a tareas con pull requests vinculados en Jira (panel de desarrollo); las tareas sin PR, como soporte o reuniones, no aplican. Si no se puede leer el panel de desarrollo, se excluyen por palabra: reuniones, capacitaciones, soporte, análisis y revisiones de tarea.</div>
  <div><b>Consulta:</b> <code>${esc(JQL)}</code></div>
  <div>${source === "cache" ? "Datos guardados en este equipo, pendientes de actualizar" : simulated ? "Datos ficticios del modo simulado" : "Datos en vivo de Jira"} del ${when.toLocaleString("es-EC", { dateStyle: "medium", timeStyle: "short" })}. Los últimos datos cargados se guardan en este equipo para abrir más rápido y se borran al cerrar sesión; la app los actualiza sola cada 15 minutos mientras está abierta. Si la dejas en la bandeja de Windows, te avisa al cierre de la jornada si faltan horas y los lunes si la semana pasada quedó incompleta; puedes ajustarlo en «Preferencias».</div>`;
}

function setTcount(id, n, cls, title) {
    const el = $(id);
    el.hidden = !n;
    el.textContent = n || "";
    el.className = "tcount" + (n && cls ? " " + cls : "");
    if (n && title) el.title = title;
    else el.removeAttribute("title");
}
function renderTabCounts() {
    setTcount("tcTareas", weekTaskCount);
    const { crit, warn } = signalCounts;
    if (crit)
        setTcount(
            "tcSenales",
            crit,
            "crit",
            `${crit} ${crit === 1 ? "señal alta" : "señales altas"}`,
        );
    else if (warn)
        setTcount(
            "tcSenales",
            warn,
            "warn",
            `${warn} ${warn === 1 ? "señal media" : "señales medias"}`,
        );
    else setTcount("tcSenales", 0);
    const miss =
        docState === "done"
            ? docWeek(viewData.week).filter((i) => docStatus(i) === "missing")
                  .length
            : 0;
    setTcount("tcDocs", miss, "crit", `${miss} con PR sin documento`);
}
function paint() {
    const iw = isoWeek(viewData.week);
    $("wTitle").textContent = `Semana ${iw.w} · ${iw.y}`;
    $("wRange").textContent =
        `${fmtD(viewData.week)} – ${fmtD(addDays(viewData.week, 4))}`;
    $("nextW").disabled = viewData.week >= mondayOf(viewData.today);
    $("gate").hidden = true;
    $("main").hidden = false;
    renderChips();
    renderHoursAlert();
    renderKpis();
    renderCharts();
    renderBoards();
    renderSignals();
    renderTable();
    renderDocs();
    renderTabCounts();
    renderTrace();
}
/* Requests the view from Rust and draws. Asynchronous: each call carries a number and only the last one paints; while the response is in flight what is
   already on screen stays. Returns the requested view (even if a newer one replaced it) or null if it failed; in that case the previous one is kept. */
async function render() {
    if (!issues.length) return null;
    const seq = ++renderSeq,
        g = gen;
    let v;
    try {
        v = await requestView();
    } catch (e) {
        if (g !== gen || handleAuth(e)) return null;
        viewError = `No se pudo calcular la vista del tablero: ${errText(e)}`;
        if (seq === renderSeq) showNotice(viewError);
        return null;
    }
    if (g !== gen || !issues.length) return null;
    if (seq !== renderSeq) return v;
    board =
        v.board; /* if the saved board no longer exists, Rust returns «Todos» (as renderChips used to) */
    if (viewError) {
        if ($("notice").textContent === viewError) showNotice("");
        viewError = "";
    }
    setView(v);
    paint();
    return v;
}
/* Repaints with the latest view, without requesting another (pagers, sorting, table views, panels). */
function repaint() {
    if (viewData) paint();
}

/* ---------- events ---------- */
$("prevW").onclick = () => {
    weekKey = addDays(weekKey, -7);
    render();
};
$("nextW").onclick = () => {
    if (weekKey >= mondayOf(currentToday())) return;
    weekKey = addDays(weekKey, 7);
    render();
}; /* with fast clicks the button was not yet disabled */
$("todayW").onclick = () => {
    weekKey = mondayOf(currentToday());
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
    const r = e.target.closest("[data-holiday]");
    if (r && (await toggleHoliday(r.dataset.holiday))) render();
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

/* ---------- data: Tauri commands (Rust) ---------- */
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
        (e && typeof e === "object" ? e.message : e) || "error desconocido";
    return String(msg).slice(0, 300);
}
const isAuthErr = (e) => e?.code === "unauthorized" || e?.code === "no_session";
const WL_WINDOW_DAYS = 98;
let wlState = "idle",
    wlError = "";
let busy = false,
    busyGen = -1;
let gen = 0; // changes on sign-out or connection loss: discards pending responses
let simulated = false,
    lastInfo = null;
/* Local cache and automatic refresh */
const CACHE_FORMAT = 2; // if the shape of what is saved changes, bump it and the old cache is ignored
const REFRESH_MS = 15 * 60e3; // automatic refresh every 15 minutes
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

/* ---------- screens: connect / loading / board ---------- */
function setGate(title, text) {
    $("gateTitle").textContent = title;
    $("gateText").textContent = text;
    $("gate").hidden = false;
}
function showConnect(info, err) {
    gen++;
    renderSeq++;
    viewData = null;
    VC = null;
    viewError = "";
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
        simulated = !!info.simulated;
    }
    $("main").hidden = true;
    $("gate").hidden = true;
    $("connect").hidden = false;
    $("refreshBtn").hidden = true;
    $("logoutBtn").hidden = true;
    showNotice("");
    $("who").textContent =
        "Tus horas, tareas y estados de Jira por semana y por tablero";
    $("simTag").hidden = !simulated;
    $("simNote").hidden = !simulated;
    const notice = info?.notice || "";
    $("connNotice").hidden = !notice;
    $("connNotice").textContent = notice;
    $("connError").hidden = !err;
    $("connError").textContent = err || "";
    $("siteInput").value =
        info?.site ||
        $("siteInput").value ||
        "https://torresytorres.atlassian.net";
    if (info?.email) $("emailInput").value = info.email;
    $("tokenInput").value = "";
    $("connBtn").disabled = false;
    setSync("", "Sin conexión con Jira");
}
/* Repaints the board with what was saved. Returns false (touching nothing) if the cache is unusable. */
async function restoreCache(snap) {
    const d = snap && snap.data;
    if (
        !d ||
        d.format !== CACHE_FORMAT ||
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
            validIso(snap.savedAt) ||
            new Date().toISOString();
        source = "cache";
        wlState = "done";
        docState = "done";
        wlError = "";
        docError = "";
        const v =
            await render(); /* the view is computed in Rust with the saved issues; render() shows the board */
        if (g !== gen) return false;
        if (!v) throw new Error(viewError || "no se pudo calcular la vista");
        setSync("", `Datos guardados · ${fmtWhen(syncedAt)}`);
        return true;
    } catch (err) {
        if (g !== gen) return false;
        console.warn("Local cache could not be used:", err);
        viewData = null;
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
        await invoke("save_cache", {
            data: { format: CACHE_FORMAT, issues, docs, prMode, syncedAt },
        });
    } catch (e) {
        if (g === gen) console.warn("Could not save local cache:", errText(e));
    }
}
async function enterApp(info) {
    lastInfo = info;
    simulated = !!info.simulated;
    ME = info.user?.accountId || null;
    SITE = String(info.site || "").replace(/\/+$/, "") + "/browse/";
    $("connect").hidden = true;
    $("connError").hidden = true;
    $("tokenInput").value = "";
    const name = info.user?.name;
    if (name)
        $("who").textContent =
            `${name} · horas, tareas y estados de Jira por semana y por tablero`;
    $("refreshBtn").hidden = false;
    $("logoutBtn").hidden = false;
    sessionActive = true;
    startAuto();
    selectTab(tab);
    setGate("Cargando tus tareas…", "Leyendo tus tareas de Jira.");
    const g = gen;
    try {
        const snap = await invoke("read_cache");
        if (g !== gen) return;
        if (snap) await restoreCache(snap);
        if (g !== gen) return;
    } catch (e) {
        if (g !== gen || handleAuth(e)) return;
        console.warn("Could not read local cache:", errText(e));
    }
    refresh(false);
}
/* If a data call returns unauthorized / no_session, it goes back to the connect screen. */
function handleAuth(e) {
    if (!isAuthErr(e)) return false;
    showConnect({
        ...(lastInfo || {}),
        connected: false,
        notice: e.message || "Tu sesión ya no es válida. Vuelve a conectarte.",
    });
    return true;
}

/* ---------- synchronization with Jira ---------- */
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
        const keys = await invoke("keys_with_hours", { since: from });
        let r = { byKey: {}, failed: [], error: null };
        if (keys.length)
            r = await invoke("read_worklogs", { keys, since: after });
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
/* quiet: data is already on screen; it does not switch to «loading» and, if it fails, the previous data is kept. Returns true if the query finished well. */
async function fetchDocs(quiet) {
    const g = gen;
    if (!quiet) {
        docState = "loading";
        render();
    }
    try {
        const prs = new Set(await invoke("keys_with_pr"));
        if (g !== gen) return false;
        for (const i of issues) i.hasPR = prs.has(i.key);
        prMode = "pr";
    } catch (e) {
        if (g !== gen || handleAuth(e)) return false;
        if (quiet && prMode === "pr" && docState === "done")
            return false; /* what was already known about the development panel is kept */
        prMode =
            "keywords"; /* no development panel: the keyword rule is used */
    }
    /* `needDoc` is computed by Rust from `hasPR`: the view is requested with the current issues and hours before choosing candidates */
    const v = await render();
    if (g !== gen) return false;
    if (!v) {
        if (quiet && docState === "done") return false;
        docState = "error";
        docError = viewError || "No se pudo calcular la vista de tareas.";
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
        const found = await invoke("find_documents", {
            keys: cands.map((i) => i.key),
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
/* Hours and documents after loading the issues. If everything finishes well and complete, saves the cache. */
async function loadDetails(g, quiet) {
    try {
        await new Promise((r) =>
            setTimeout(r, 0),
        ); /* lets the issues paint first */
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
                "none"; /* with data on screen it is neither emptied nor flickered */
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
        const list = await invoke("load_issues");
        if (g !== gen) return;
        if (quiet) {
            /* hours and documents already read are kept until the new ones arrive */
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
            viewData = null;
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
            return; /* render() shows the board when the view arrives */
        if (!v && !viewData) {
            setGate(
                "No pudimos calcular el tablero",
                (viewError || "Error al calcular la vista") +
                    ". Pulsa Actualizar para reintentar.",
            );
            $("main").hidden = true;
            return;
        }
        await loadDetails(g, quiet);
    } catch (e) {
        if (g !== gen || handleAuth(e)) return;
        if (issues.length > 0 && source !== "none") {
            /* there is data on screen: it is kept */
            $("gate").hidden = true;
            $("main").hidden = false;
            if (e?.code === "network" || e?.code === "rate_limit") {
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
const scrollToEl = (id) =>
    $(id).scrollIntoView({
        behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
            ? "auto"
            : "smooth",
        block: "start",
    });
const goTable = () => scrollToEl("tableHead");
const goDocs = () => scrollToEl("dTitle");
function selectTab(name, { focus = false } = {}) {
    if (!TABS.includes(name)) name = "resumen";
    tab = name;
    for (const t of TABS) {
        const on = t === name;
        $("tab-" + t).setAttribute("aria-selected", String(on));
        $("tab-" + t).tabIndex = on ? 0 : -1;
        $("pane-" + t).hidden = !on;
    }
    try {
        localStorage.setItem("spe.tab", name);
    } catch (e) {}
    if (focus) $("tab-" + name).focus();
    /* The new pane starts right below the sticky bar, not halfway down where the previous one was scrolled. */
    const barBottom = document
            .querySelector(".stickybar")
            .getBoundingClientRect().bottom,
        paneTop = $("pane-" + name).getBoundingClientRect().top;
    if (paneTop < barBottom)
        window.scrollBy({
            top: paneTop - barBottom - 8,
            behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
                ? "auto"
                : "smooth",
        });
}
$("tabs").onclick = (e) => {
    const t = e.target.closest("[data-tab]");
    if (t) selectTab(t.dataset.tab);
};
$("tabs").onkeydown = (e) => {
    const i = TABS.indexOf(tab);
    const next = {
        ArrowRight: (i + 1) % TABS.length,
        ArrowLeft: (i + TABS.length - 1) % TABS.length,
        Home: 0,
        End: TABS.length - 1,
    }[e.key];
    if (next === undefined) return;
    e.preventDefault();
    selectTab(TABS[next], { focus: true });
};
function chooseView(v) {
    view = v;
    if (view === "open") {
        sortKey = "due";
        sortDir = 1;
    } else if (sortKey === "due") {
        sortKey = "act";
        sortDir = -1;
    }
}
$("kpis").onclick = (e) => {
    const k = e.target.closest("[data-view]");
    if (!k) return;
    if (k.dataset.view === "nodoc") {
        selectTab("despliegues");
        return;
    }
    chooseView(k.dataset.view);
    repaint();
    selectTab("tareas");
};
$("viewSeg").onclick = (e) => {
    const b = e.target.closest("[data-seg]");
    if (!b) return;
    chooseView(b.dataset.seg);
    repaint();
};
$("boards").onclick = (e) => {
    const b = e.target.closest("[data-open-board]");
    if (!b) return;
    board = b.dataset.openBoard;
    view = "open";
    sortKey = "due";
    sortDir = 1;
    render();
    selectTab("tareas");
};
$("signals").onclick = (e) => {
    const b = e.target.closest("[data-goview]");
    if (!b) return;
    if (b.dataset.goview === "nodoc") {
        selectTab("despliegues");
        return;
    }
    view = b.dataset.goview;
    repaint();
    selectTab("tareas");
};
selectTab(tab);

/* ---------- connection, sign-out and external links ---------- */
$("connForm").onsubmit = async (ev) => {
    ev.preventDefault();
    const site = $("siteInput").value.trim(),
        email = $("emailInput").value.trim(),
        token = $("tokenInput").value;
    $("tokenInput").value = "";
    $("connError").hidden = true;
    $("connBtn").disabled = true;
    try {
        const info = await invoke("sign_in", { site, email, token });
        enterApp(info);
    } catch (e) {
        $("connError").textContent = errText(e);
        $("connError").hidden = false;
        $("connBtn").disabled = false;
    }
};
$("logoutBtn").onclick = async () => {
    let info = { ...(lastInfo || {}), connected: false, notice: null },
        err = null;
    try {
        info = await invoke("sign_out");
    } catch (e) {
        err = errText(e);
    }
    showConnect(info, err);
};
/* In the app, target="_blank" does not open the browser: https links are opened with the system. */
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

/* ---------- preferences: panel ---------- */
const notificationPrefInputs = () => {
    $("prefEndOfDay").disabled = $("prefWeekly").disabled =
        !$("prefNotifications").checked;
};
function openPrefs() {
    $("prefNotifications").checked = prefs.notifications.enabled;
    $("prefEndOfDay").value = prefs.notifications.endOfDayTime;
    $("prefWeekly").checked = prefs.notifications.weeklySummary;
    $("prefAutostart").checked = prefs.startWithWindows;
    $("prefTray").checked = prefs.minimizeToTray;
    notificationPrefInputs();
    $("prefsSimNote").hidden = !simulated;
    $("prefsError").hidden = true;
    $("prefsSave").disabled = false;
    $("diagText").hidden = $("diagCopy").hidden = true;
    $("diagText").value = $("diagStatus").textContent = "";
    $("diagBtn").disabled = false;
    if (!$("prefsDlg").open) $("prefsDlg").showModal();
}
function showPrefsError(msg) {
    $("prefsError").textContent = msg;
    $("prefsError").hidden = false;
}
$("prefsBtn").onclick = openPrefs;
$("prefsCancel").onclick = () => $("prefsDlg").close();
$("prefNotifications").onchange = notificationPrefInputs;
$("diagBtn").onclick = async () => {
    if (!sessionActive) {
        $("diagStatus").textContent =
            "Conéctate a Jira para generar el diagnóstico.";
        return;
    }
    $("diagBtn").disabled = true;
    $("diagStatus").textContent =
        "Generando… puede tardar como una actualización desde Jira.";
    try {
        $("diagText").value = await invoke("build_diagnostics");
        $("diagText").hidden = $("diagCopy").hidden = false;
        $("diagStatus").textContent = "Listo. Revísalo y cópialo al reporte.";
    } catch (e) {
        if (isAuthErr(e)) {
            $("prefsDlg").close();
            handleAuth(e);
        } else
            $("diagStatus").textContent =
                e?.message || "No se pudo generar el diagnóstico.";
    } finally {
        $("diagBtn").disabled = false;
    }
};
$("diagCopy").onclick = async () => {
    try {
        await navigator.clipboard.writeText($("diagText").value);
        $("diagStatus").textContent = "Copiado. Pégalo en el reporte.";
    } catch (e) {
        $("diagText").select();
        $("diagStatus").textContent =
            "No se pudo copiar automáticamente: el texto quedó seleccionado, cópialo con Ctrl+C.";
    }
};
$("prefsForm").onsubmit = async (ev) => {
    ev.preventDefault();
    if (prefsBusy) return;
    const enabled = $("prefNotifications").checked,
        endOfDay = $("prefEndOfDay").value;
    if (enabled && !endOfDay) {
        showPrefsError("Indica la hora de cierre de la jornada (HH:MM).");
        return;
    }
    const next = {
        ...prefs,
        notifications: {
            enabled: enabled,
            endOfDayTime: endOfDay || prefs.notifications.endOfDayTime,
            weeklySummary: $("prefWeekly").checked,
        },
        startWithWindows: $("prefAutostart").checked,
        minimizeToTray: $("prefTray").checked,
        extraHolidays,
    };
    prefsBusy = true;
    $("prefsSave").disabled = true;
    $("prefsError").hidden = true;
    try {
        await savePrefs(next);
        if (
            next.startWithWindows === true &&
            prefs.startWithWindows === false
        ) {
            $("prefAutostart").checked = false;
            showPrefsError(
                AUTOSTART_NOTICE + " Los demás cambios se guardaron.",
            );
        } else $("prefsDlg").close();
    } catch (e) {
        showPrefsError(errText(e));
    } finally {
        prefsBusy = false;
        $("prefsSave").disabled = false;
    }
};

/* ---------- Rust events (tray and monitor) ---------- */
function listenEvents() {
    const listen = window.__TAURI__?.event?.listen;
    if (!listen) return;
    const on = (name, fn) => {
        try {
            Promise.resolve(listen(name, fn)).catch((e) =>
                console.warn(`Could not listen to ${name}:`, errText(e)),
            );
        } catch (e) {
            console.warn(`Could not listen to ${name}:`, errText(e));
        }
    };
    on("signal://refresh", () => {
        if (sessionActive && !(busy && busyGen === gen)) refresh(true);
    });
    on("signal://preferences", async () => {
        const previous = prefs.startWithWindows;
        try {
            applyPrefs(await invoke("read_preferences"));
        } catch (e) {
            console.warn("Could not reload preferences:", errText(e));
            return;
        }
        if (previous === true && prefs.startWithWindows === false) {
            prefsNotice = AUTOSTART_NOTICE;
            showNotice(AUTOSTART_NOTICE);
        }
        if ($("prefsDlg").open)
            $("prefAutostart").checked =
                prefs.startWithWindows; /* what Rust changes on its own; does not overwrite what the person is editing */
        render(); /* holidays may have changed; with no issues loaded it does nothing */
    });
}

/* ---------- startup ---------- */
weekKey = mondayOf(todayLocal());
setSync("", "Sin datos todavía");
listenEvents();
(async () => {
    await loadPrefs(); /* needs no session and does not throw: the preferences (holidays) must be loaded before requesting the first view */
    try {
        const info = await invoke("restore_session");
        if (info.connected) enterApp(info);
        else showConnect(info);
    } catch (e) {
        showConnect(null, `No se pudo iniciar la aplicación: ${errText(e)}`);
    }
})();
