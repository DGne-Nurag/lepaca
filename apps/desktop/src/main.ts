// lepaca – Pattern Viewer mit erster Werkzeugleiste.

import { Ansicht, lage, PX_PRO_MM, type Darstellung, type Entwurf, type Vorschau } from "./ansicht";
import {
  enthaelt,
  formFehler,
  montageAktualisieren,
  pdfExportieren,
  pfadeFuer,
  pfadeSofort,
  skizzeLoesen,
  skizzeSofort,
  uebernommeneSofort,
  vorlageLaden,
} from "./kern";
import {
  istMass,
  kante,
  kantenzahl,
  neueId,
  pfadPunkte,
  punktzahl,
  neuesMuster,
  neueTPocket,
  Verlauf,
  type Ecke,
  type Eisen,
  type Bedingung,
  type Element as Stueck,
  type Form,
  type Muster,
  type P,
  type PfadForm,
  type Teil,
} from "./modell";
import { abstandZuLinie, auf45Grad, bogenAusWoelbung, kantenLaenge, kantePunkte } from "./pfad";
import { fehlertext, leseZahl, t, zahl, type TextSchluessel } from "./texte";

type Werkzeug = "auswahl" | "hand" | "bemassen" | "rechteck" | "kreis" | "ellipse" | "tpocket" | Stift;
/** Werkzeuge, die Stück für Stück einen Pfad zeichnen. */
type Stift = "linie" | "bogen" | "bezier";
const istStift = (w: Werkzeug): w is Stift => w === "linie" || w === "bogen" || w === "bezier";

const WERKZEUGE: { id: Werkzeug; taste: string; icon: string }[] = [
  { id: "auswahl", taste: "V", icon: '<path d="M6 3l12 9-5 1.2 3 6-2.2 1-3-6L6 18z"/>' },
  { id: "hand", taste: "H", icon: '<path d="M8 11V5.5a1.5 1.5 0 013 0V10m0-5a1.5 1.5 0 013 0V10m0-3.5a1.5 1.5 0 013 0V14a6 6 0 01-6 6h-1a6 6 0 01-5-2.7L4.5 14a1.5 1.5 0 012.5-1.6L8 14"/>' },
  { id: "bemassen", taste: "D", icon: '<path d="M4 7v10M20 7v10M4 12h16"/><path d="M8 9.5L4.5 12 8 14.5M16 9.5l3.5 2.5-3.5 2.5"/>' },
  { id: "rechteck", taste: "R", icon: '<rect x="4" y="6" width="16" height="12" rx="1.5"/>' },
  { id: "kreis", taste: "K", icon: '<circle cx="12" cy="12" r="8"/>' },
  { id: "ellipse", taste: "E", icon: '<ellipse cx="12" cy="12" rx="9" ry="6"/>' },
  { id: "tpocket", taste: "T", icon: '<path d="M3 6h18v4h-4v9H7v-9H3z"/>' },
  { id: "linie", taste: "L", icon: '<path d="M4 18L11 7l9 6"/><rect x="2.5" y="16.5" width="3" height="3"/><rect x="9.5" y="5.5" width="3" height="3"/><rect x="18.5" y="11.5" width="3" height="3"/>' },
  { id: "bogen", taste: "B", icon: '<path d="M4 18A9 9 0 0120 18"/><rect x="2.5" y="16.5" width="3" height="3"/><rect x="18.5" y="16.5" width="3" height="3"/>' },
  { id: "bezier", taste: "P", icon: '<path d="M4 18C6 6 18 18 20 6"/><path d="M4 18L9 9M20 6l-4 8" stroke-dasharray="2 2"/><circle cx="9" cy="9" r="1.5"/><circle cx="16" cy="14" r="1.5"/>' },
];

// ---------------------------------------------------------------- Zustand

const verlauf = new Verlauf(neuesMuster());
let auswahl: number | null = null;
let werkzeug: Werkzeug = "auswahl";
let vorschau: Vorschau | null = null;

/** Pfad, der gerade mit Linie, Bogen oder Bézier gezeichnet wird. */
interface PfadEntwurf {
  start: P;
  elemente: Stueck[];
  /** Bogen: Ziel steht, jetzt wird die Wölbung gezogen. */
  bogenZiel: P | null;
  /** Bézier: ausgehender Griff am letzten Ankerpunkt (relativ). */
  griff: P;
  /** Bézier: Maus ist gedrückt, der Griff wird gezogen. */
  ziehtGriff: boolean;
  /** Letzte Mausposition im Muster (gerastert). */
  maus: P;
}
let pfadEntwurf: PfadEntwurf | null = null;

let zeichnenGeplant = false;

/** Maße auf der Fläche zeigen. */
let zeigeMasse = true;
/** Teile in ihrer Lage im fertigen Stück zeigen statt im Muster. */
let zusammenbau = false;

/** Bemaßen: gewählte Punkte und Kanten im Pfad des gewählten Teils. */
interface SkizzenWahl {
  kanten: number[];
  punkte: number[];
}
let skizzenWahl: SkizzenWahl = { kanten: [], punkte: [] };
let skizzenHover: { kante: number | null; punkt: number | null } = { kante: null, punkt: null };
/** Maß, dessen Feld nach dem Neuaufbau den Fokus bekommt. */
let fokusBedingung: number | null = null;

const muster = (): Muster => verlauf.muster;
const gewaehlt = (): Teil | undefined => muster().teile.find((x) => x.id === auswahl);

// ---------------------------------------------------------------- DOM-Hilfen

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attr: Record<string, string> = {},
  ...kinder: (Node | string)[]
): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attr)) e.setAttribute(k, v);
  e.append(...kinder);
  return e;
}

function icon(inhalt: string): SVGSVGElement {
  const s = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  s.setAttribute("viewBox", "0 0 24 24");
  s.setAttribute("aria-hidden", "true");
  s.innerHTML = inhalt; // nur feste Symbole aus diesem Quelltext
  return s;
}

// ---------------------------------------------------------------- Aufbau

const app = document.getElementById("app")!;

const werkzeugleiste = el("div", { class: "werkzeugleiste", role: "toolbar", "aria-label": t("werkzeuge"), "aria-orientation": "vertical" });
const werkzeugKnoepfe = new Map<Werkzeug, HTMLButtonElement>();
for (const w of WERKZEUGE) {
  const name = t(`werkzeug.${w.id}` as TextSchluessel);
  const b = el("button", { type: "button", class: "werkzeug", title: `${name} (${w.taste})`, "aria-label": name, "aria-keyshortcuts": w.taste });
  b.append(icon(w.icon));
  b.addEventListener("click", () => werkzeugWaehlen(w.id));
  werkzeugKnoepfe.set(w.id, b);
  werkzeugleiste.append(b);
}

const knopfZurueck = el("button", { type: "button", title: `${t("rueckgaengig")} (Strg+Z)`, "aria-keyshortcuts": "Control+Z" }, t("rueckgaengig"));
const knopfVor = el("button", { type: "button", title: `${t("wiederholen")} (Strg+Umschalt+Z)`, "aria-keyshortcuts": "Control+Shift+Z" }, t("wiederholen"));
const knopfAlles = el("button", { type: "button", title: `${t("alles.zeigen")} (0)`, "aria-keyshortcuts": "0" }, t("alles.zeigen"));
const knopfVorlage = el("button", { type: "button", title: t("vorlage.bifold.titel") }, t("vorlage.bifold"));
const knopfMasse = el("button", { type: "button", "aria-pressed": "true", title: `${t("ansicht.masse.titel")} (M)`, "aria-keyshortcuts": "M" }, t("ansicht.masse"));
const knopfZusammenbau = el("button", { type: "button", "aria-pressed": "false", title: `${t("ansicht.zusammenbau.titel")} (Z)`, "aria-keyshortcuts": "Z" }, t("ansicht.zusammenbau"));
const knopfExport = el("button", { type: "button", class: "primaer", title: `${t("export")} (Strg+P)`, "aria-keyshortcuts": "Control+P" }, t("export"));
const kopf = el(
  "header",
  { class: "kopf" },
  el("span", { class: "marke" }, "lepaca"),
  knopfZurueck,
  knopfVor,
  el("span", { class: "trenner" }),
  knopfAlles,
  knopfMasse,
  knopfZusammenbau,
  el("span", { class: "trenner" }),
  knopfVorlage,
  el("span", { class: "fueller" }),
  knopfExport,
);

const canvas = el("canvas", { class: "flaeche", tabindex: "0", role: "img" });
const leerHinweis = el("p", { class: "leer-hinweis" }, t("leer"));
const mitte = el("main", { class: "mitte" }, canvas, leerHinweis);

const teileListe = el("ul", { class: "teile-liste", role: "listbox", "aria-label": t("teile") });
const eigenschaften = el("div", { class: "eigenschaften" });
const seite = el(
  "aside",
  { class: "seite" },
  el("h2", {}, t("teile")),
  teileListe,
  eigenschaften,
);

const statusPosition = el("span", { class: "status-position" });
const statusZoom = el("span", { class: "status-zoom" });
const statusMeldung = el("span", { class: "status-meldung", role: "status", "aria-live": "polite" });
const knopfTest = el("button", { type: "button", class: "klein", title: t("leistungstest.titel") }, t("leistungstest"));
const fuss = el("footer", { class: "fuss" }, statusPosition, statusZoom, statusMeldung, el("span", { class: "fueller" }), knopfTest);

app.append(kopf, werkzeugleiste, mitte, seite, fuss);

const ansicht = new Ansicht(canvas);

// ---------------------------------------------------------------- Zeichnen

function neuZeichnen(): void {
  if (zeichnenGeplant) return;
  zeichnenGeplant = true;
  requestAnimationFrame(() => {
    zeichnenGeplant = false;
    ansicht.zeichnen(muster(), auswahl, vorschau, entwurfFuerAnsicht(), darstellung());
    statusZoom.textContent = t("status.zoom", { z: Math.round((ansicht.massstab / PX_PRO_MM) * 100) });
  });
}

function darstellung(): Darstellung {
  const g = gewaehlt();
  const f = g?.form;
  const skizze =
    werkzeug === "bemassen" && g && f?.art === "pfad" && !zusammenbau
      ? {
          teil: g.id,
          kanten: skizzenWahl.kanten,
          punkte: skizzenWahl.punkte,
          hoverKante: skizzenHover.kante,
          hoverPunkt: skizzenHover.punkt,
          freieKanten: skizzeSofort(f)?.freieKanten ?? null,
        }
      : null;
  return { masse: zeigeMasse, zusammenbau, skizze };
}

/** Skizzen, die der Kern nicht lösen konnte; nicht immer wieder fragen. */
const skizzeUnloesbar = new Set<string>();

/** Zustand der Skizze des gewählten Teils (Freiheitsgrade) nachladen. */
function skizzeLaden(): void {
  const f = gewaehlt()?.form;
  if (f?.art !== "pfad" || skizzeSofort(f)) return;
  const s = JSON.stringify(f);
  if (skizzeUnloesbar.has(s)) return;
  skizzeLoesen(f)
    .then(() => {
      neuZeichnen();
      eigenschaftenAktualisieren();
    })
    .catch(() => skizzeUnloesbar.add(s));
}

/** Löcher aus dem Zusammenbau nachrechnen lassen. */
function montageLaden(): void {
  montageAktualisieren(muster().teile)
    .then((neu) => {
      if (!neu) return;
      neuZeichnen();
      eigenschaftenAktualisieren();
    })
    .catch((e: unknown) => melden(fehlertext(String(e))));
}

/** Fehlende Geometrie nachladen und danach neu zeichnen. */
function geometrieLaden(): void {
  const offen = muster().teile.filter((x) => !pfadeSofort(x) && !formFehler(x));
  if (offen.length === 0) return;
  Promise.all(offen.map(pfadeFuer))
    .then(() => {
      neuZeichnen();
      eigenschaftenAktualisieren();
    })
    .catch((e: unknown) => melden(fehlertext(String(e))));
}

// ---------------------------------------------------------------- Pfad zeichnen

/** Fangradius zum Schließen am Startpunkt, in Bildschirmpixeln. */
const SCHLIESSEN_PX = 8;

const letzterPunkt = (e: PfadEntwurf): P => e.elemente.at(-1)?.ziel ?? e.start;
const gleich = (a: P, b: P) => Math.abs(a[0] - b[0]) < 1e-9 && Math.abs(a[1] - b[1]) < 1e-9;
const nahAmStart = (e: PfadEntwurf, q: P) =>
  e.elemente.length >= 2 && Math.hypot(q[0] - e.start[0], q[1] - e.start[1]) * ansicht.massstab <= SCHLIESSEN_PX;

/** Das Stück vom letzten Punkt bis `ziel` mit dem aktiven Werkzeug. */
function naechstesStueck(e: PfadEntwurf, ziel: P): Stueck | null {
  const von = letzterPunkt(e);
  if (gleich(von, ziel)) return null;
  if (werkzeug === "bezier") {
    const k1: P = [von[0] + e.griff[0], von[1] + e.griff[1]];
    return { art: "kurve", k1, k2: ziel, ziel };
  }
  return { art: "linie", ziel };
}

function entwurfFuerAnsicht(): Entwurf | null {
  const e = pfadEntwurf;
  if (!e) return null;
  let naechstes: Stueck | null = null;
  const griffe: [P, P][] = [];
  const von = letzterPunkt(e);
  if (e.bogenZiel) {
    const b = bogenAusWoelbung(von, e.bogenZiel, e.maus);
    naechstes = b ? { art: "bogen", ziel: e.bogenZiel, ...b } : { art: "linie", ziel: e.bogenZiel };
  } else if (!e.ziehtGriff) {
    naechstes = naechstesStueck(e, nahAmStart(e, e.maus) ? e.start : e.maus);
  }
  if (werkzeug === "bezier" && (e.griff[0] !== 0 || e.griff[1] !== 0)) {
    griffe.push([von, [von[0] + e.griff[0], von[1] + e.griff[1]]]);
    griffe.push([von, [von[0] - e.griff[0], von[1] - e.griff[1]]]);
  }
  return { start: e.start, elemente: e.elemente, naechstes, griffe, schliessbar: nahAmStart(e, e.maus) };
}

/** Mausposition im Muster: auf 1 mm gerastert (Alt: frei), mit Umschalt
 * in 45°-Schritten zum letzten Punkt. */
function stiftPunkt(ev: PointerEvent | MouseEvent): P {
  const [mx, my] = ansicht.zuMuster(ev.offsetX, ev.offsetY);
  let q: P = [mx, my];
  if (pfadEntwurf && ev.shiftKey && !pfadEntwurf.ziehtGriff) q = auf45Grad(letzterPunkt(pfadEntwurf), q);
  return ev.altKey ? q : [Math.round(q[0]), Math.round(q[1])];
}

function stiftMeldung(): void {
  const e = pfadEntwurf;
  if (!e) return melden(t(`hinweis.${werkzeug}` as TextSchluessel));
  if (e.bogenZiel) {
    const b = bogenAusWoelbung(letzterPunkt(e), e.bogenZiel, e.maus);
    return melden(t("hinweis.bogenWoelben", { r: b ? zahl(b.radius, 1) : "–" }));
  }
  melden(t("hinweis.weiterZeichnen"));
}

function stiftDruecken(ev: PointerEvent): void {
  const q = stiftPunkt(ev);
  const e = pfadEntwurf;
  if (!e) {
    pfadEntwurf = { start: q, elemente: [], bogenZiel: null, griff: [0, 0], ziehtGriff: werkzeug === "bezier", maus: q };
    stiftMeldung();
    return;
  }
  e.maus = q;
  if (nahAmStart(e, q) && !e.bogenZiel) {
    if (werkzeug === "bogen") {
      e.bogenZiel = e.start;
    } else {
      const s = naechstesStueck(e, e.start);
      if (s) e.elemente.push(s);
      pfadFertig(true);
    }
    return;
  }
  if (werkzeug === "bogen") {
    if (!e.bogenZiel) {
      if (!gleich(letzterPunkt(e), q)) e.bogenZiel = q;
    } else {
      const ziel = e.bogenZiel;
      const b = bogenAusWoelbung(letzterPunkt(e), ziel, q);
      // Radius auf 0,5 mm runden, aber nie kleiner als die halbe Sehne.
      if (b) {
        const von = letzterPunkt(e);
        const halb = Math.hypot(ziel[0] - von[0], ziel[1] - von[1]) / 2;
        b.radius = Math.max(halb, ev.altKey ? b.radius : Math.round(b.radius * 2) / 2);
      }
      e.elemente.push(b ? { art: "bogen", ziel, ...b } : { art: "linie", ziel });
      e.bogenZiel = null;
      if (gleich(ziel, e.start) && e.elemente.length >= 2) return pfadFertig(true);
    }
  } else {
    const s = naechstesStueck(e, q);
    if (s) e.elemente.push(s);
    e.griff = [0, 0];
    e.ziehtGriff = werkzeug === "bezier";
  }
  stiftMeldung();
}

function stiftBewegen(ev: PointerEvent): void {
  const e = pfadEntwurf;
  if (!e) return;
  const q = stiftPunkt(ev);
  if (e.ziehtGriff && ev.buttons & 1) {
    // Bézier: Griff am letzten Ankerpunkt ziehen; der eingehende Griff des
    // letzten Stücks wird gespiegelt, damit der Übergang glatt ist.
    const a = letzterPunkt(e);
    e.griff = [q[0] - a[0], q[1] - a[1]];
    const l = e.elemente.at(-1);
    if (l?.art === "kurve") l.k2 = [a[0] - e.griff[0], a[1] - e.griff[1]];
  }
  e.maus = q;
  if (e.bogenZiel) stiftMeldung();
}

function stiftLoslassen(): void {
  const e = pfadEntwurf;
  if (!e) return;
  e.ziehtGriff = false;
  // Kurven ohne Griffe sind gerade Linien.
  const l = e.elemente.at(-1);
  if (l?.art === "kurve") {
    const von = e.elemente.length > 1 ? e.elemente.at(-2)!.ziel : e.start;
    if (gleich(l.k1, von) && gleich(l.k2, l.ziel)) e.elemente[e.elemente.length - 1] = { art: "linie", ziel: l.ziel };
  }
}

/** Waagerecht und senkrecht gezeichnete Linien bekommen gleich die
 * passende Bedingung, wie in CAD-Skizzen üblich. */
function geradeKanten(f: PfadForm): Bedingung[] {
  const aus: Bedingung[] = [];
  const punkte = pfadPunkte(f);
  for (let k = 0; k < kantenzahl(f); k++) {
    const e = kante(f, k);
    if (e.element.art !== "linie") continue;
    const a = punkte[e.von]!;
    const b = punkte[e.bis]!;
    const dx = Math.abs(b[0] - a[0]);
    const dy = Math.abs(b[1] - a[1]);
    if (dy < 1e-9 && dx > 1e-9) aus.push({ art: "horizontal", kante: k });
    else if (dx < 1e-9 && dy > 1e-9) aus.push({ art: "vertikal", kante: k });
  }
  return aus;
}

/** Entwurf als Teil (geschlossen) oder Linie (offen) übernehmen. */
function pfadFertig(geschlossen: boolean): void {
  const e = pfadEntwurf;
  pfadEntwurf = null;
  if (!e || e.elemente.length === 0 || (geschlossen && e.elemente.length < 2)) {
    stiftMeldung();
    neuZeichnen();
    return;
  }
  // Koordinaten relativ zum Startpunkt; der Startpunkt wird die Position.
  const [x0, y0] = e.start;
  const rel = (q: P): P => [q[0] - x0, q[1] - y0];
  const elemente: Stueck[] = e.elemente.map((el) =>
    el.art === "kurve" ? { ...el, k1: rel(el.k1), k2: rel(el.k2), ziel: rel(el.ziel) } : { ...el, ziel: rel(el.ziel) },
  );
  aendern((m) => {
    const id = neueId(m);
    const name = `${t(geschlossen ? "form.pfad" : "form.linie")} ${id}`;
    const form: PfadForm = { art: "pfad", start: [0, 0], elemente, geschlossen, bedingungen: [] };
    form.bedingungen = geradeKanten(form);
    // Linien sind oft Markierungen: ohne Naht anlegen.
    m.teile.push({ id, name, form, x: x0, y: y0, naht: geschlossen ? { ...m.standardNaht } : null });
    auswahl = id;
  });
  stiftMeldung();
}

function allesAktualisieren(): void {
  geometrieLaden();
  skizzeLaden();
  montageLaden();
  knopfMasse.setAttribute("aria-pressed", String(zeigeMasse));
  knopfZusammenbau.setAttribute("aria-pressed", String(zusammenbau));
  listeAktualisieren();
  eigenschaftenAktualisieren();
  knopfZurueck.disabled = !verlauf.kannZurueck;
  knopfVor.disabled = !verlauf.kannVor;
  leerHinweis.hidden = muster().teile.length > 0;
  canvas.setAttribute("aria-label", `${t("muster")}: ${muster().teile.map((x) => x.name).join(", ") || t("teile.keine")}`);
  neuZeichnen();
}

function melden(text: string): void {
  statusMeldung.textContent = text;
}

// ---------------------------------------------------------------- Änderungen

function aendern(f: (m: Muster) => void): void {
  verlauf.merken();
  f(muster());
  allesAktualisieren();
}

function werkzeugWaehlen(w: Werkzeug): void {
  // Zwischen Linie, Bogen und Bézier wechseln, ohne den Pfad abzubrechen:
  // so lassen sich die Stückarten in einem Pfad mischen.
  if (pfadEntwurf && !istStift(w)) pfadFertig(false);
  if (pfadEntwurf) {
    pfadEntwurf.bogenZiel = null;
    pfadEntwurf.griff = [0, 0];
  }
  werkzeug = w;
  skizzenWahl = { kanten: [], punkte: [] };
  skizzenHover = { kante: null, punkt: null };
  for (const [id, b] of werkzeugKnoepfe) b.setAttribute("aria-pressed", String(id === w));
  canvas.dataset["werkzeug"] = w;
  stiftMeldung();
  eigenschaftenAktualisieren();
  neuZeichnen();
}

function teilAnlegen(form: Form, x: number, y: number): void {
  aendern((m) => {
    const id = neueId(m);
    const art = t(`form.${form.art}` as TextSchluessel);
    m.teile.push({ id, name: `${art} ${id}`, form, x, y, naht: { ...m.standardNaht } });
    auswahl = id;
  });
  werkzeugWaehlen("auswahl");
}

function rueckgaengig(): void {
  if (verlauf.rueckgaengig()) allesAktualisieren();
}

function wiederholen(): void {
  if (verlauf.wiederholen()) allesAktualisieren();
}

function loeschen(): void {
  const g = gewaehlt();
  if (!g) return;
  aendern((m) => {
    m.teile = m.teile.filter((x) => x.id !== g.id);
  });
  auswahl = null;
  allesAktualisieren();
}

function duplizieren(): void {
  const g = gewaehlt();
  if (!g) return;
  aendern((m) => {
    const id = neueId(m);
    const kopie: Teil = { ...structuredClone(g), id, name: `${g.name} (2)`, x: g.x + 10, y: g.y + 10 };
    m.teile.push(kopie);
    auswahl = id;
  });
}

function verschieben(dx: number, dy: number): void {
  const g = gewaehlt();
  if (!g) return;
  aendern(() => {
    g.x += dx;
    g.y += dy;
  });
}

// ---------------------------------------------------------------- Liste

function listeAktualisieren(): void {
  teileListe.replaceChildren();
  if (muster().teile.length === 0) {
    teileListe.append(el("li", { class: "leer" }, t("teile.keine")));
    return;
  }
  for (const x of muster().teile) {
    const li = el("li", { role: "option", tabindex: "0", "aria-selected": String(x.id === auswahl) }, x.name);
    li.addEventListener("click", () => {
      auswahl = x.id;
      allesAktualisieren();
    });
    li.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        auswahl = x.id;
        allesAktualisieren();
      }
    });
    teileListe.append(li);
  }
}

// ---------------------------------------------------------------- Eigenschaften

let feldNummer = 0;

/** Zahlenfeld mit Einheit; übernimmt den Wert bei Enter oder Verlassen. */
function zahlFeld(beschriftung: string, wert: number, setzen: (x: number) => void, einheit = "mm"): HTMLElement {
  const id = `feld-${++feldNummer}`;
  const fehlerId = `${id}-fehler`;
  const eingabe = el("input", { id, type: "text", inputmode: "decimal", value: zahl(wert), "aria-describedby": fehlerId, spellcheck: "false" });
  const fehler = el("span", { id: fehlerId, class: "feld-fehler" });
  const uebernehmen = () => {
    const x = leseZahl(eingabe.value);
    if (x === null) {
      eingabe.setAttribute("aria-invalid", "true");
      fehler.textContent = t("eingabe.zahl");
      return;
    }
    eingabe.removeAttribute("aria-invalid");
    fehler.textContent = "";
    if (x !== wert) setzen(x);
  };
  eingabe.addEventListener("change", uebernehmen);
  eingabe.addEventListener("keydown", (e) => {
    if (e.key === "Enter") uebernehmen();
  });
  return el("div", { class: "feld" }, el("label", { for: id }, beschriftung), el("span", { class: "mit-einheit" }, eingabe, el("span", { class: "einheit" }, einheit)), fehler);
}

function auswahlFeld<T extends string>(beschriftung: string, wert: T, optionen: [T, string][], setzen: (x: T) => void): HTMLElement {
  const id = `feld-${++feldNummer}`;
  const s = el("select", { id });
  for (const [v, text] of optionen) {
    const o = el("option", { value: v }, text);
    o.selected = v === wert;
    s.append(o);
  }
  s.addEventListener("change", () => setzen(s.value as T));
  return el("div", { class: "feld" }, el("label", { for: id }, beschriftung), s);
}

function textFeld(beschriftung: string, wert: string, setzen: (x: string) => void): HTMLElement {
  const id = `feld-${++feldNummer}`;
  const e = el("input", { id, type: "text", value: wert, maxlength: "80" });
  e.addEventListener("change", () => setzen(e.value.trim() || wert));
  return el("div", { class: "feld" }, el("label", { for: id }, beschriftung), e);
}

function eigenschaftenAktualisieren(): void {
  // Fokus merken, damit Tippen nicht unterbrochen wird. Wiedergefunden
  // wird das Feld an Art, Beschriftung und Reihenfolge, damit der Fokus nie
  // auf ein anderes Feld springt, wenn sich der Abschnitt verändert hat.
  const bedienelemente = () => [...eigenschaften.querySelectorAll<HTMLElement>("input,select,button")];
  const kennung = (e: Element) =>
    `${e.tagName}|${e.closest(".feld")?.querySelector("label")?.textContent ?? ""}|${e.tagName === "BUTTON" ? e.textContent : ""}`;
  const vorher = bedienelemente();
  const fokusIndex = vorher.indexOf(document.activeElement as HTMLElement);
  const fokusKennung = fokusIndex >= 0 ? kennung(vorher[fokusIndex]!) : "";
  eigenschaften.replaceChildren();
  const g = gewaehlt();
  if (g) eigenschaften.append(...teilFelder(g));
  else eigenschaften.append(...musterFelder());
  if (fokusBedingung !== null) {
    const feld = eigenschaften.querySelector<HTMLInputElement>(`[data-bedingung="${fokusBedingung}"] input`);
    fokusBedingung = null;
    if (feld) {
      feld.focus();
      feld.select();
      return;
    }
  }
  if (fokusIndex >= 0) {
    const nachher = bedienelemente();
    const gleich = nachher[fokusIndex];
    if (gleich && kennung(gleich) === fokusKennung) gleich.focus();
    else nachher.find((e) => kennung(e) === fokusKennung)?.focus();
  }
}

function teilFelder(g: Teil): HTMLElement[] {
  const f = g.form;
  const felder: HTMLElement[] = [
    el("h2", {}, t("eigenschaften")),
    textFeld(t("teil.name"), g.name, (x) => aendern(() => (g.name = x))),
  ];
  const fehler = formFehler(g);
  if (fehler) felder.push(el("p", { class: "meldung fehler", role: "alert" }, fehlertext(fehler)));
  if (f.art === "kreis") {
    felder.push(zahlFeld(t("teil.durchmesser"), f.durchmesser, (x) => aendern(() => (f.durchmesser = x))));
  } else if (f.art === "pfad") {
    felder.push(...pfadFelder(f), ...skizzenFelder(g, f));
  } else {
    if (f.art !== "tpocket" && zeigeMasse) felder.push(el("p", { class: "meldung" }, t("skizze.grundform")));
    felder.push(
      zahlFeld(t("teil.breite"), f.breite, (x) => aendern(() => (f.breite = x))),
      zahlFeld(t("teil.hoehe"), f.hoehe, (x) => aendern(() => (f.hoehe = x))),
    );
    if (f.art === "rechteck") felder.push(zahlFeld(t("teil.radius"), f.eckenradius, (x) => aendern(() => (f.eckenradius = x))));
    if (f.art === "tpocket") {
      felder.push(
        zahlFeld(t("teil.fluegelUeberstand"), f.fluegelUeberstand, (x) => aendern(() => (f.fluegelUeberstand = x))),
        zahlFeld(t("teil.fluegelHoehe"), f.fluegelHoehe, (x) => aendern(() => (f.fluegelHoehe = x))),
      );
    }
  }
  felder.push(
    zahlFeld(t("teil.x"), g.x, (x) => aendern(() => (g.x = x))),
    zahlFeld(t("teil.y"), g.y, (y) => aendern(() => (g.y = y))),
  );

  // Naht
  const anId = `feld-${++feldNummer}`;
  const an = el("input", { id: anId, type: "checkbox" });
  an.checked = g.naht !== null;
  an.addEventListener("change", () => aendern((m) => (g.naht = an.checked ? { ...m.standardNaht } : null)));
  felder.push(el("h3", {}, t("naht")), el("div", { class: "feld feld-zeile" }, an, el("label", { for: anId }, t("naht.an"))));
  const n = g.naht;
  if (n) {
    const merke = (m: Muster) => (m.standardNaht = { ...n });
    // Auf einer offenen Linie sitzen die Löcher direkt auf der Linie.
    if (f.art !== "pfad" || f.geschlossen) felder.push(
      zahlFeld(t("naht.kantenabstand"), n.kantenabstand, (x) => aendern((m) => ((n.kantenabstand = x), merke(m)))),
    );
    felder.push(
      zahlFeld(t("naht.lochabstand"), n.lochabstand, (x) => aendern((m) => ((n.lochabstand = x), merke(m)))),
      auswahlFeld<Eisen>(t("naht.eisen"), n.eisen, [["french", t("eisen.french")], ["diamond", t("eisen.diamond")], ["japan", t("eisen.japan")], ["round", t("eisen.round")]], (x) => aendern((m) => ((n.eisen = x), merke(m)))),
      auswahlFeld<Ecke>(t("naht.ecke"), n.ecke, [["ein", t("ecke.ein")], ["aus", t("ecke.aus")], ["mitte", t("ecke.mitte")]], (x) => aendern((m) => ((n.ecke = x), merke(m)))),
    );
    if (f.art === "pfad" && f.geschlossen) {
      felder.push(el("p", { class: "feld-titel" }, t("naht.kanten")));
      const n = kantenzahl(f);
      const zeile = el("div", { class: "kanten-wahl" });
      for (let k = 0; k < n; k++) {
        const id = `feld-${++feldNummer}`;
        const box = el("input", { id, type: "checkbox" });
        box.checked = f.naehte?.[k] ?? true;
        box.addEventListener("change", () =>
          aendern(() => {
            const neu = Array.from({ length: n }, (_, i) => f.naehte?.[i] ?? true);
            neu[k] = box.checked;
            f.naehte = neu.every((x) => x) ? undefined : neu;
          }),
        );
        zeile.append(el("span", { class: "feld-zeile" }, box, el("label", { for: id }, t("skizze.kante", { n: k + 1 }))));
      }
      felder.push(zeile);
    }
    if (f.art === "tpocket") {
      felder.push(el("p", { class: "feld-titel" }, t("naht.kanten")));
      const kante = (text: TextSchluessel, wert: boolean, setzen: (x: boolean) => void) => {
        const id = `feld-${++feldNummer}`;
        const k = el("input", { id, type: "checkbox" });
        k.checked = wert;
        k.addEventListener("change", () => aendern(() => setzen(k.checked)));
        return el("div", { class: "feld feld-zeile" }, k, el("label", { for: id }, t(text)));
      };
      felder.push(
        kante("naht.fluegel", f.nahtFluegel, (x) => (f.nahtFluegel = x)),
        kante("naht.steg", f.nahtSteg, (x) => (f.nahtSteg = x)),
        kante("naht.oeffnung", f.nahtOeffnung, (x) => (f.nahtOeffnung = x)),
      );
    }
    const geo = pfadeSofort(g)?.geometrie;
    if (geo?.stichfehler) {
      felder.push(el("p", { class: "meldung fehler", role: "alert" }, fehlertext(geo.stichfehler)));
    } else if (geo) {
      felder.push(el("p", { class: "meldung" }, t("naht.info", { n: geo.loecher.length, e: geo.ecklocher })));
      const abweichend = [...new Set(geo.abstaende.filter((a) => Math.abs(a - n.lochabstand) >= 0.005).map((a) => zahl(a)))];
      if (abweichend.length > 0) felder.push(el("p", { class: "meldung" }, t("naht.angepasst", { a: abweichend.join(" / ") })));
    }
  }

  felder.push(...montageFelder(g));

  const dup = el("button", { type: "button", title: `${t("teil.duplizieren")} (Strg+D)` }, t("teil.duplizieren"));
  dup.addEventListener("click", duplizieren);
  const weg = el("button", { type: "button", class: "gefahr", title: `${t("teil.loeschen")} (Entf)` }, t("teil.loeschen"));
  weg.addEventListener("click", loeschen);
  felder.push(el("div", { class: "knopfzeile" }, dup, weg));
  return felder;
}

/** Felder für einen gezeichneten Pfad: offen/geschlossen und je Bogen
 * Radius, Richtung und großer Bogen. */
function pfadFelder(f: Extract<Form, { art: "pfad" }>): HTMLElement[] {
  const felder: HTMLElement[] = [];
  const zu = `feld-${++feldNummer}`;
  const k = el("input", { id: zu, type: "checkbox" });
  k.checked = f.geschlossen;
  k.addEventListener("change", () =>
    aendern(() => {
      f.geschlossen = k.checked;
      // Die Schlusslinie kommt oder geht: Bedingungen und Nähte daran fallen weg.
      f.bedingungen = f.bedingungen.filter((b) => bedingungGueltig(f, b));
      f.naehte = undefined;
    }),
  );
  felder.push(el("div", { class: "feld feld-zeile" }, k, el("label", { for: zu }, t("pfad.geschlossen"))));
  let nr = 0;
  for (const e of f.elemente) {
    if (e.art !== "bogen") continue;
    nr++;
    // Mit Bemaßung bestimmt die Skizze die Bögen.
    if (f.bedingungen.length > 0) continue;
    felder.push(el("p", { class: "feld-titel" }, t("pfad.bogen", { n: nr })));
    felder.push(zahlFeld(t("pfad.radius"), e.radius, (x) => aendern(() => (e.radius = x))));
    const umdrehen = el("button", { type: "button" }, t("pfad.umkehren"));
    umdrehen.addEventListener("click", () => aendern(() => (e.uhrzeigersinn = !e.uhrzeigersinn)));
    const id = `feld-${++feldNummer}`;
    const gross = el("input", { id, type: "checkbox" });
    gross.checked = e.gross;
    gross.addEventListener("change", () => aendern(() => (e.gross = gross.checked)));
    felder.push(
      el("div", { class: "feld feld-zeile" }, gross, el("label", { for: id }, t("pfad.gross"))),
      el("div", { class: "knopfzeile knopfzeile-eng" }, umdrehen),
    );
  }
  const anzahl = { linien: 0, boegen: nr, kurven: 0 };
  for (const e of f.elemente) {
    if (e.art === "linie") anzahl.linien++;
    if (e.art === "kurve") anzahl.kurven++;
  }
  felder.push(el("p", { class: "meldung" }, t("pfad.info", { l: anzahl.linien, b: anzahl.boegen, k: anzahl.kurven })));
  return felder;
}

function musterFelder(): HTMLElement[] {
  const d = muster().druck;
  return [
    el("h2", {}, t("druck")),
    auswahlFeld(t("druck.papier"), d.papier, [["a4", "A4"], ["a3", "A3"], ["letter", "US Letter"]], (x) => aendern((m) => (m.druck.papier = x))),
    zahlFeld(t("druck.rand"), d.rand, (x) => aendern((m) => (m.druck.rand = x))),
    zahlFeld(t("druck.klebekante"), d.klebekante, (x) => aendern((m) => (m.druck.klebekante = x))),
    ...anleitungFelder(),
  ];
}

function anleitungFelder(): HTMLElement[] {
  const a = muster().anleitung;
  if (!a || a.length === 0) return [];
  return [el("h2", {}, t("anleitung")), el("ol", { class: "anleitung" }, ...a.map((x) => el("li", {}, x)))];
}

// ---------------------------------------------------------------- Skizze

const kantenArt = (f: PfadForm, k: number) => kante(f, k).element.art;

/** Passt die Bedingung (noch) zum Pfad? */
function bedingungGueltig(f: PfadForm, b: Bedingung): boolean {
  const n = kantenzahl(f);
  const p = punktzahl(f);
  switch (b.art) {
    case "laenge":
    case "radius":
    case "horizontal":
    case "vertikal":
      return b.kante < n;
    case "abstand":
    case "abstandX":
    case "abstandY":
      return b.a < p && b.b < p;
    case "fest":
      return b.punkt < p;
    default:
      return b.a < n && b.b < n;
  }
}

const freiText = (n: number) => (n === 1 ? t("skizze.frei1") : t("skizze.frei", { n }));

/** Neue Bedingungen lösen lassen und nur übernehmen, wenn die Skizze
 * lösbar und nicht überbestimmt ist. */
async function skizzeSetzen(g: Teil, bedingungen: Bedingung[], fokus: number | null = null): Promise<boolean> {
  const f = g.form;
  if (f.art !== "pfad") return false;
  const vorher = JSON.stringify(f);
  const versuch: PfadForm = { ...structuredClone(f), bedingungen };
  try {
    const e = await skizzeLoesen(versuch);
    if (e.ueberbestimmt) {
      melden(t("skizze.ueberbestimmt"));
      eigenschaftenAktualisieren();
      neuZeichnen();
      return false;
    }
    // Inzwischen geändert (z. B. rückgängig gemacht): nichts überschreiben.
    const jetzt = muster().teile.find((x) => x.id === g.id);
    if (!jetzt || JSON.stringify(jetzt.form) !== vorher || jetzt.form.art !== "pfad") return false;
    const form = jetzt.form;
    fokusBedingung = fokus;
    aendern(() => {
      form.bedingungen = bedingungen;
      form.elemente = e.elemente;
    });
    melden(e.freiheitsgrade === 0 ? t("skizze.bestimmt") : freiText(e.freiheitsgrade));
    return true;
  } catch (err) {
    melden(fehlertext(String(err)));
    eigenschaftenAktualisieren();
    neuZeichnen();
    return false;
  }
}

function winkelZwischen(f: PfadForm, a: number, b: number): number {
  const punkte = pfadPunkte(f);
  const r = (k: number): P => {
    const e = kante(f, k);
    const p = punkte[e.von]!;
    const q = punkte[e.bis]!;
    return [q[0] - p[0], q[1] - p[1]];
  };
  const [ax, ay] = r(a);
  const [bx, by] = r(b);
  const c = (ax * bx + ay * by) / (Math.hypot(ax, ay) * Math.hypot(bx, by) || 1);
  return (Math.acos(Math.max(-1, Math.min(1, c))) * 180) / Math.PI;
}

/** Bedingungen, die zur Auswahl passen, mit den gezeichneten Werten. */
function moeglicheBedingungen(f: PfadForm, w: SkizzenWahl): { text: TextSchluessel; b: Bedingung }[] {
  const aus: { text: TextSchluessel; b: Bedingung }[] = [];
  const runde = (x: number) => Math.round(x * 100) / 100;
  const punkte = pfadPunkte(f);
  if (w.kanten.length === 1 && w.punkte.length === 0) {
    const k = w.kanten[0]!;
    const art = kantenArt(f, k);
    if (art === "linie") {
      aus.push(
        { text: "skizze.laenge", b: { art: "laenge", kante: k, wert: runde(kantenLaenge(f, k)) } },
        { text: "skizze.horizontal", b: { art: "horizontal", kante: k } },
        { text: "skizze.vertikal", b: { art: "vertikal", kante: k } },
      );
    } else if (art === "bogen") {
      const e = kante(f, k).element;
      if (e.art === "bogen") aus.push({ text: "skizze.radius", b: { art: "radius", kante: k, wert: runde(e.radius) } });
    }
  } else if (w.kanten.length === 2 && w.punkte.length === 0) {
    const [a, b] = w.kanten as [number, number];
    const artA = kantenArt(f, a);
    const artB = kantenArt(f, b);
    if (artA === "linie" && artB === "linie") {
      aus.push(
        { text: "skizze.parallel", b: { art: "parallel", a, b } },
        { text: "skizze.senkrecht", b: { art: "senkrecht", a, b } },
        { text: "skizze.gleich", b: { art: "gleich", a, b } },
        { text: "skizze.winkel", b: { art: "winkel", a, b, grad: runde(winkelZwischen(f, a, b)) } },
      );
    } else if (artA === "bogen" && artB === "bogen") {
      aus.push({ text: "skizze.gleich", b: { art: "gleich", a, b } });
    }
    // Tangential nur an einem gemeinsamen Punkt.
    const ka = kante(f, a);
    const kb = kante(f, b);
    const teilen = ka.bis === kb.von || kb.bis === ka.von;
    if (teilen && (artA !== "linie" || artB !== "linie")) aus.push({ text: "skizze.tangential", b: { art: "tangential", a, b } });
  } else if (w.punkte.length === 2 && w.kanten.length === 0) {
    const [a, b] = w.punkte as [number, number];
    const p = punkte[a]!;
    const q = punkte[b]!;
    const dx = Math.abs(q[0] - p[0]);
    const dy = Math.abs(q[1] - p[1]);
    if (dx > 1e-6) aus.push({ text: "skizze.abstandX", b: { art: "abstandX", a, b, wert: runde(dx) } });
    if (dy > 1e-6) aus.push({ text: "skizze.abstandY", b: { art: "abstandY", a, b, wert: runde(dy) } });
    aus.push({ text: "skizze.abstand", b: { art: "abstand", a, b, wert: runde(Math.hypot(dx, dy)) } });
  } else if (w.punkte.length === 1 && w.kanten.length === 0) {
    const a = w.punkte[0]!;
    const p = punkte[a]!;
    aus.push({ text: "skizze.fest", b: { art: "fest", punkt: a, x: runde(p[0]), y: runde(p[1]) } });
  }
  return aus;
}

/** Bezeichnung einer Bedingung in der Liste, Kanten und Punkte ab 1 gezählt. */
function bedingungText(b: Bedingung): string {
  switch (b.art) {
    case "laenge":
    case "radius":
    case "horizontal":
    case "vertikal":
      return t(`skizze.${b.art}.text` as TextSchluessel, { k: b.kante + 1 });
    case "abstand":
    case "abstandX":
    case "abstandY":
      return t(`skizze.${b.art}.text` as TextSchluessel, { a: b.a + 1, b: b.b + 1 });
    case "fest":
      return t("skizze.fest.text", { a: b.punkt + 1 });
    default:
      return t(`skizze.${b.art}.text` as TextSchluessel, { a: b.a + 1, b: b.b + 1 });
  }
}

/** Abschnitt „Skizze“: Zustand, Bemaßen, Maße und Bedingungen. */
function skizzenFelder(g: Teil, f: PfadForm): HTMLElement[] {
  const felder: HTMLElement[] = [el("h3", {}, t("skizze"))];
  const e = skizzeSofort(f);
  const zustand = !e
    ? t("skizze.unbekannt")
    : e.freiheitsgrade === 0
      ? t("skizze.bestimmt")
      : freiText(e.freiheitsgrade);
  felder.push(el("p", { class: e?.freiheitsgrade === 0 ? "meldung gut" : "meldung" }, zustand));

  if (werkzeug !== "bemassen") {
    const b = el("button", { type: "button", title: `${t("werkzeug.bemassen")} (D)` }, t("skizze.bemassen"));
    b.addEventListener("click", () => werkzeugWaehlen("bemassen"));
    felder.push(el("div", { class: "knopfzeile knopfzeile-eng" }, b));
  } else {
    const moeglich = moeglicheBedingungen(f, skizzenWahl);
    if (moeglich.length === 0) {
      felder.push(el("p", { class: "meldung" }, t("skizze.waehlen")));
    } else {
      const zeile = el("div", { class: "knopfzeile knopfzeile-eng bedingungen-knoepfe" });
      for (const m of moeglich) {
        const b = el("button", { type: "button" }, t(m.text));
        b.addEventListener("click", () => {
          const neu = [...f.bedingungen, m.b];
          skizzenWahl = { kanten: [], punkte: [] };
          void skizzeSetzen(g, neu, istMass(m.b) ? neu.length - 1 : null);
        });
        zeile.append(b);
      }
      felder.push(zeile);
    }
  }

  const masse = f.bedingungen.map((b, i) => [b, i] as const).filter(([b]) => istMass(b));
  const andere = f.bedingungen.map((b, i) => [b, i] as const).filter(([b]) => !istMass(b));
  const entfernen = (i: number) => {
    const knopf = el("button", { type: "button", class: "klein entfernen", title: t("skizze.entfernen"), "aria-label": t("skizze.entfernen") }, "×");
    knopf.addEventListener("click", () => void skizzeSetzen(g, f.bedingungen.filter((_, j) => j !== i)));
    return knopf;
  };
  if (masse.length > 0) {
    felder.push(el("p", { class: "feld-titel" }, t("skizze.masse")));
    for (const [b, i] of masse) {
      if (!istMass(b)) continue;
      const wert = "wert" in b ? b.wert : b.grad;
      const feld = zahlFeld(
        bedingungText(b),
        wert,
        (x) => {
          const neu = structuredClone(f.bedingungen);
          const z = neu[i]!;
          if ("wert" in z) z.wert = x;
          else if ("grad" in z) z.grad = x;
          void skizzeSetzen(g, neu);
        },
        b.art === "winkel" ? "°" : "mm",
      );
      felder.push(el("div", { class: "mass-zeile", "data-bedingung": String(i) }, feld, entfernen(i)));
    }
  }
  if (andere.length > 0) {
    felder.push(el("p", { class: "feld-titel" }, t("skizze.bedingungen")));
    const liste = el("ul", { class: "bedingungen" });
    for (const [b, i] of andere) liste.append(el("li", {}, el("span", {}, bedingungText(b)), entfernen(i)));
    felder.push(liste);
  }
  return felder;
}

/** Treffer auf einen Punkt (Vorrang) oder eine Kante der Skizze. */
function skizzenTreffer(g: Teil, mx: number, my: number): { punkt: number | null; kante: number | null } {
  const f = g.form;
  if (f.art !== "pfad") return { punkt: null, kante: null };
  const x = mx - g.x;
  const y = my - g.y;
  const fang = 7 / ansicht.massstab;
  let punkt: number | null = null;
  let best = fang;
  pfadPunkte(f)
    .slice(0, punktzahl(f))
    .forEach((q, i) => {
      const d = Math.hypot(q[0] - x, q[1] - y);
      if (d <= best) {
        best = d;
        punkt = i;
      }
    });
  if (punkt !== null) return { punkt, kante: null };
  let kanteNr: number | null = null;
  best = fang;
  for (let k = 0; k < kantenzahl(f); k++) {
    const d = abstandZuLinie(kantePunkte(f, k), x, y);
    if (d <= best) {
      best = d;
      kanteNr = k;
    }
  }
  return { punkt: null, kante: kanteNr };
}

/** Bemaßen: Klick wählt Punkte und Kanten (bis zu zwei), ein Klick auf ein
 * anderes Teil wählt dieses. */
function bemassenKlick(mx: number, my: number): void {
  const g = gewaehlt();
  const treffer = g ? skizzenTreffer(g, mx, my) : null;
  if (g && treffer && (treffer.punkt !== null || treffer.kante !== null)) {
    const w = skizzenWahl;
    const liste = treffer.punkt !== null ? w.punkte : w.kanten;
    const i = (treffer.punkt ?? treffer.kante)!;
    if (liste.includes(i)) liste.splice(liste.indexOf(i), 1);
    else {
      if (w.punkte.length + w.kanten.length >= 2) skizzenWahl = { kanten: [], punkte: [] };
      (treffer.punkt !== null ? skizzenWahl.punkte : skizzenWahl.kanten).push(i);
    }
  } else {
    const anderes = trefferBei(mx, my);
    auswahl = anderes?.id ?? null;
    skizzenWahl = { kanten: [], punkte: [] };
    if (anderes && anderes.form.art !== "pfad") melden(t("skizze.nurPfad"));
  }
  allesAktualisieren();
}

/** Doppelklick beim Bemaßen: Länge oder Radius der Kante als Maß. */
function bemassenDoppelklick(mx: number, my: number): void {
  const g = gewaehlt();
  if (!g || g.form.art !== "pfad") return;
  const f = g.form;
  const tr = skizzenTreffer(g, mx, my);
  if (tr.kante === null) return;
  const m = moeglicheBedingungen(f, { kanten: [tr.kante], punkte: [] }).find((x) => istMass(x.b));
  if (!m) return;
  skizzenWahl = { kanten: [], punkte: [] };
  const neu = [...f.bedingungen, m.b];
  void skizzeSetzen(g, neu, neu.length - 1);
}

// ---------------------------------------------------------------- Zusammenbau

function montageFelder(g: Teil): HTMLElement[] {
  const felder: HTMLElement[] = [el("h3", {}, t("montage"))];
  const id = `feld-${++feldNummer}`;
  const an = el("input", { id, type: "checkbox" });
  an.checked = !!g.montage;
  an.addEventListener("change", () => aendern(() => (g.montage = an.checked ? { x: g.x, y: g.y, schritt: 1 } : null)));
  felder.push(el("div", { class: "feld feld-zeile" }, an, el("label", { for: id }, t("montage.an"))));
  const m = g.montage;
  if (!m) return felder;
  felder.push(
    zahlFeld(t("montage.x"), m.x, (x) => aendern(() => (m.x = x))),
    zahlFeld(t("montage.y"), m.y, (y) => aendern(() => (m.y = y))),
    zahlFeld(t("montage.schritt"), m.schritt, (x) => aendern(() => (m.schritt = Math.max(0, Math.min(1000, Math.round(x))))), ""),
  );
  const u = uebernommeneSofort(g);
  if (u) {
    if (u.anzahl > 0) felder.push(el("p", { class: "meldung" }, t("montage.uebernommen", { n: u.anzahl })));
    if (u.kantennah > 0) felder.push(el("p", { class: "meldung fehler", role: "alert" }, t("montage.kantennah", { n: u.kantennah })));
    if (u.zuDicht > 0) felder.push(el("p", { class: "meldung fehler", role: "alert" }, t("montage.zuDicht", { n: u.zuDicht })));
  }
  return felder;
}

async function vorlageEinsetzen(): Promise<void> {
  try {
    const v = await vorlageLaden("bifold");
    aendern((m) => {
      const teile: Teil[] = [];
      for (const x of v.teile) {
        teile.push({ id: neueId({ ...m, teile }), name: x.name ?? "", form: x.form, x: x.x, y: x.y, naht: x.stiche, montage: x.montage });
      }
      m.teile = teile;
      m.anleitung = v.schritte;
    });
    auswahl = null;
    // Erst die Geometrie, dann passt „Alles zeigen“.
    await Promise.all(muster().teile.map(pfadeFuer));
    ansicht.allesZeigen(muster(), zusammenbau);
    melden(t("vorlage.geladen"));
    allesAktualisieren();
  } catch (e) {
    melden(fehlertext(String(e)));
  }
}

// ---------------------------------------------------------------- Zeiger

interface Ziehen {
  art: "schwenken" | "verschieben" | "aufziehen";
  startX: number;
  startY: number;
  /** Musterkoordinaten beim Start. */
  mx: number;
  my: number;
  teilX: number;
  teilY: number;
  gemerkt: boolean;
}
let ziehen: Ziehen | null = null;
let leertaste = false;

const runden = (x: number, aus: boolean) => (aus ? x : Math.round(x));

function trefferBei(mx: number, my: number): Teil | undefined {
  const teile = muster().teile;
  for (let i = teile.length - 1; i >= 0; i--) {
    const x = teile[i]!;
    const p = pfadeSofort(x);
    const ort = lage(x, zusammenbau);
    if (!p || !ort) continue;
    const g = p.geometrie;
    const nah = abstandZuLinie(g.geschlossen ? [...g.kontur, g.kontur[0]!] : g.kontur, mx - ort[0], my - ort[1]) * ansicht.massstab <= 6;
    if (nah || (g.geschlossen && enthaelt(g.kontur, mx - ort[0], my - ort[1]))) return x;
  }
  return undefined;
}

canvas.addEventListener("pointerdown", (e) => {
  canvas.focus();
  canvas.setPointerCapture(e.pointerId);
  const [mx, my] = ansicht.zuMuster(e.offsetX, e.offsetY);
  const basis = { startX: e.offsetX, startY: e.offsetY, mx, my, teilX: 0, teilY: 0, gemerkt: false };
  if (e.button === 1 || werkzeug === "hand" || leertaste) {
    ziehen = { art: "schwenken", ...basis, teilX: ansicht.versatzX, teilY: ansicht.versatzY };
  } else if (istStift(werkzeug)) {
    if (e.button === 0) stiftDruecken(e);
    neuZeichnen();
  } else if (werkzeug === "bemassen") {
    if (e.button === 0) bemassenKlick(mx, my);
  } else if (werkzeug === "auswahl") {
    const treffer = trefferBei(mx, my);
    auswahl = treffer?.id ?? null;
    const ort = treffer && lage(treffer, zusammenbau);
    if (ort) ziehen = { art: "verschieben", ...basis, teilX: ort[0], teilY: ort[1] };
    allesAktualisieren();
  } else {
    ziehen = { art: "aufziehen", ...basis };
  }
});

canvas.addEventListener("pointermove", (e) => {
  const [mx, my] = ansicht.zuMuster(e.offsetX, e.offsetY);
  statusPosition.textContent = t("status.position", { x: zahl(mx, 1), y: zahl(my, 1) });
  if (!ziehen && istStift(werkzeug)) {
    stiftBewegen(e);
    neuZeichnen();
    return;
  }
  if (!ziehen && werkzeug === "bemassen") {
    const g = gewaehlt();
    const neu = g && !zusammenbau ? skizzenTreffer(g, mx, my) : { kante: null, punkt: null };
    if (neu.kante !== skizzenHover.kante || neu.punkt !== skizzenHover.punkt) {
      skizzenHover = neu;
      neuZeichnen();
    }
    return;
  }
  if (!ziehen) return;
  const z = ziehen;
  if (z.art === "schwenken") {
    ansicht.versatzX = z.teilX + e.offsetX - z.startX;
    ansicht.versatzY = z.teilY + e.offsetY - z.startY;
  } else if (z.art === "verschieben") {
    const g = gewaehlt();
    if (!g) return;
    if (!z.gemerkt) {
      verlauf.merken();
      z.gemerkt = true;
    }
    const ziel = zusammenbau ? g.montage : g;
    if (!ziel) return;
    ziel.x = runden(z.teilX + mx - z.mx, e.altKey);
    ziel.y = runden(z.teilY + my - z.my, e.altKey);
  } else {
    let x0 = runden(z.mx, e.altKey);
    let y0 = runden(z.my, e.altKey);
    let b = runden(mx, e.altKey) - x0;
    let h = runden(my, e.altKey) - y0;
    if (e.shiftKey || werkzeug === "kreis") {
      const s = Math.max(Math.abs(b), Math.abs(h));
      b = Math.sign(b || 1) * s;
      h = Math.sign(h || 1) * s;
    }
    if (b < 0) [x0, b] = [x0 + b, -b];
    if (h < 0) [y0, h] = [y0 + h, -h];
    vorschau = { art: werkzeug as Vorschau["art"], x: x0, y: y0, breite: b, hoehe: h };
  }
  neuZeichnen();
});

canvas.addEventListener("pointerup", () => {
  if (istStift(werkzeug)) stiftLoslassen();
  const z = ziehen;
  ziehen = null;
  if (!z) return;
  if (z.art === "verschieben" && z.gemerkt) {
    allesAktualisieren();
  } else if (z.art === "aufziehen") {
    const v = vorschau;
    vorschau = null;
    const x = v?.x ?? Math.round(z.mx);
    const y = v?.y ?? Math.round(z.my);
    const gross = v && v.breite >= 5 && v.hoehe >= 5;
    if (werkzeug === "rechteck") {
      teilAnlegen({ art: "rechteck", breite: gross ? v.breite : 80, hoehe: gross ? v.hoehe : 50, eckenradius: 0 }, x, y);
    } else if (werkzeug === "kreis") {
      teilAnlegen({ art: "kreis", durchmesser: gross ? v.breite : 60 }, x, y);
    } else if (werkzeug === "ellipse") {
      teilAnlegen({ art: "ellipse", breite: gross ? v.breite : 80, hoehe: gross ? v.hoehe : 50 }, x, y);
    } else if (werkzeug === "tpocket") {
      // Unter 20 × 15 mm bleibt kein sinnvoller Steg: dann die Startwerte.
      const passt = v && v.breite >= 20 && v.hoehe >= 15;
      teilAnlegen(passt ? neueTPocket(v.breite, v.hoehe) : neueTPocket(), x, y);
    }
  }
  neuZeichnen();
});

canvas.addEventListener("pointerleave", () => {
  if (skizzenHover.kante === null && skizzenHover.punkt === null) return;
  skizzenHover = { kante: null, punkt: null };
  neuZeichnen();
});

canvas.addEventListener("dblclick", (e) => {
  if (pfadEntwurf) pfadFertig(false);
  else if (werkzeug === "bemassen") {
    const [mx, my] = ansicht.zuMuster(e.offsetX, e.offsetY);
    bemassenDoppelklick(mx, my);
  }
});

canvas.addEventListener(
  "wheel",
  (e) => {
    e.preventDefault();
    ansicht.zoomen(Math.exp(-e.deltaY * (e.deltaMode === 1 ? 0.05 : 0.0015)), e.offsetX, e.offsetY);
    neuZeichnen();
  },
  { passive: false },
);

// ---------------------------------------------------------------- Tastatur

const TASTEN: Record<string, Werkzeug> = {
  v: "auswahl",
  h: "hand",
  r: "rechteck",
  k: "kreis",
  e: "ellipse",
  t: "tpocket",
  l: "linie",
  b: "bogen",
  p: "bezier",
  d: "bemassen",
};

window.addEventListener("keydown", (e) => {
  const imFeld = e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement;
  const strg = e.ctrlKey || e.metaKey;
  const taste = e.key.toLowerCase();
  if (strg && taste === "z") {
    if (imFeld) return;
    e.preventDefault();
    if (e.shiftKey) wiederholen();
    else rueckgaengig();
  } else if (strg && taste === "y") {
    e.preventDefault();
    wiederholen();
  } else if (strg && taste === "p") {
    e.preventDefault();
    exportieren();
  } else if (strg && taste === "d") {
    e.preventDefault();
    duplizieren();
  } else if (imFeld || strg || e.altKey) {
    return;
  } else if (taste === " ") {
    leertaste = true;
  } else if (pfadEntwurf && taste === "enter") {
    e.preventDefault();
    pfadFertig(false);
  } else if (pfadEntwurf && taste === "escape") {
    // Abbrechen: der halbe Pfad wird verworfen.
    pfadEntwurf = null;
    stiftMeldung();
    neuZeichnen();
  } else if (pfadEntwurf && taste === "backspace") {
    e.preventDefault();
    if (pfadEntwurf.bogenZiel) pfadEntwurf.bogenZiel = null;
    else if (pfadEntwurf.elemente.pop() === undefined) pfadEntwurf = null;
    stiftMeldung();
    neuZeichnen();
  } else if (TASTEN[taste]) {
    werkzeugWaehlen(TASTEN[taste]);
  } else if (taste === "0") {
    ansicht.allesZeigen(muster(), zusammenbau);
    neuZeichnen();
  } else if (taste === "m") {
    masseUmschalten();
  } else if (taste === "z") {
    zusammenbauUmschalten();
  } else if (taste === "+" || taste === "-") {
    ansicht.zoomen(taste === "+" ? 1.25 : 0.8, canvas.clientWidth / 2, canvas.clientHeight / 2);
    neuZeichnen();
  } else if ((taste === "delete" || taste === "backspace") && e.target === canvas) {
    loeschen();
  } else if (taste === "escape" && werkzeug === "bemassen" && skizzenWahl.kanten.length + skizzenWahl.punkte.length > 0) {
    skizzenWahl = { kanten: [], punkte: [] };
    allesAktualisieren();
  } else if (taste === "escape") {
    auswahl = null;
    vorschau = null;
    ziehen = null;
    allesAktualisieren();
  } else if (taste.startsWith("arrow") && e.target === canvas && gewaehlt()) {
    e.preventDefault();
    const s = e.shiftKey ? 10 : 1;
    const d: Record<string, [number, number]> = { arrowleft: [-s, 0], arrowright: [s, 0], arrowup: [0, -s], arrowdown: [0, s] };
    const [dx, dy] = d[taste] ?? [0, 0];
    verschieben(dx, dy);
  }
});
window.addEventListener("keyup", (e) => {
  if (e.key === " ") leertaste = false;
});

function masseUmschalten(): void {
  zeigeMasse = !zeigeMasse;
  allesAktualisieren();
}

function zusammenbauUmschalten(): void {
  zusammenbau = !zusammenbau;
  skizzenWahl = { kanten: [], punkte: [] };
  melden(t(zusammenbau ? "ansicht.zusammenbau.an" : "ansicht.zusammenbau.aus"));
  ansicht.allesZeigen(muster(), zusammenbau);
  allesAktualisieren();
}

// ---------------------------------------------------------------- Export

async function exportieren(): Promise<void> {
  try {
    const r = await pdfExportieren(muster().teile, muster().druck);
    melden(r.abgebrochen ? t("export.abgebrochen") : t("export.fertig", { d: r.datei, b: r.blaetter }));
  } catch (e) {
    melden(fehlertext(String(e)));
  }
}

// ---------------------------------------------------------------- Leistungstest (ADR 0002)

async function leistungstest(): Promise<void> {
  melden(t("leistungstest.laeuft"));
  knopfTest.disabled = true;
  try {
    // 50 Teile à gut 400 Löcher: 310 × 250 mm, 2,7 mm Lochabstand.
    const start = performance.now();
    aendern((m) => {
      m.teile = [];
      for (let i = 0; i < 50; i++) {
        const id = neueId(m);
        m.teile.push({
          id,
          name: `Test ${i + 1}`,
          form: { art: "rechteck", breite: 310, hoehe: 250, eckenradius: 8 },
          x: (i % 10) * 330,
          y: Math.floor(i / 10) * 270,
          naht: { kantenabstand: 3, lochabstand: 2.7, eisen: "diamond", ecke: "ein" },
        });
      }
    });
    auswahl = null;
    // Jedes Teil einzeln berechnen lassen (gleiche Form, trotzdem ehrlich messen).
    await Promise.all(muster().teile.map((x, i) => pfadeFuer({ ...x, naht: { ...x.naht!, kantenabstand: 3 + i * 1e-6 } })));
    await Promise.all(muster().teile.map(pfadeFuer));
    const rechnen = performance.now() - start;
    const loecher = muster().teile.reduce((s, x) => s + (pfadeSofort(x)?.geometrie.loecher.length ?? 0), 0);

    // 240 Bilder: hinein- und herauszoomen und dabei verschieben. Gemessen
    // werden die Zeichenbefehle und der Abstand der Bilder, der auch das
    // Rastern durch WebKit enthält.
    const dauern: number[] = [];
    ansicht.allesZeigen(muster());
    const basis = ansicht.massstab;
    const bilder = 240;
    let erstes = 0;
    for (let f = 0; f < bilder; f++) {
      const phase = (f / bilder) * Math.PI * 2;
      ansicht.massstab = basis * (1 + 9 * (1 - Math.cos(phase)) / 2);
      ansicht.versatzX = canvas.clientWidth / 2 - (1650 + 1400 * Math.sin(phase)) * ansicht.massstab;
      ansicht.versatzY = canvas.clientHeight / 2 - 675 * ansicht.massstab;
      const t0 = performance.now();
      ansicht.zeichnen(muster(), null, null);
      dauern.push(performance.now() - t0);
      const jetzt = await new Promise<number>((r) => requestAnimationFrame(r));
      if (f === 0) erstes = jetzt;
      else if (f === bilder - 1) erstes = (bilder - 1) / ((jetzt - erstes) / 1000);
    }
    const sortiert = [...dauern].sort((a, b) => a - b);
    const mittel = dauern.reduce((a, b) => a + b, 0) / dauern.length;
    const p95 = sortiert[Math.floor(sortiert.length * 0.95)] ?? mittel;
    const bilderProSekunde = erstes;
    melden(
      t("leistungstest.ergebnis", {
        t: muster().teile.length,
        l: loecher,
        g: Math.round(rechnen),
        m: zahl(mittel, 1),
        p: zahl(p95, 1),
        f: Math.round(bilderProSekunde),
      }),
    );
    ansicht.allesZeigen(muster());
    allesAktualisieren();
  } catch (e) {
    melden(fehlertext(String(e)));
  } finally {
    knopfTest.disabled = false;
  }
}

// ---------------------------------------------------------------- Start

knopfZurueck.addEventListener("click", rueckgaengig);
knopfVor.addEventListener("click", wiederholen);
knopfAlles.addEventListener("click", () => {
  ansicht.allesZeigen(muster(), zusammenbau);
  neuZeichnen();
});
knopfMasse.addEventListener("click", masseUmschalten);
knopfZusammenbau.addEventListener("click", zusammenbauUmschalten);
knopfVorlage.addEventListener("click", () => void vorlageEinsetzen());
knopfExport.addEventListener("click", () => void exportieren());
knopfTest.addEventListener("click", () => void leistungstest());

new ResizeObserver(() => {
  ansicht.groesseAnpassen();
  neuZeichnen();
}).observe(canvas);

werkzeugWaehlen("auswahl");
allesAktualisieren();
