// lepaca – Pattern Viewer mit erster Werkzeugleiste.

import { Ansicht, PX_PRO_MM, type Entwurf, type Vorschau } from "./ansicht";
import { enthaelt, formFehler, pdfExportieren, pfadeFuer, pfadeSofort } from "./kern";
import {
  neueId,
  neuesMuster,
  neueTPocket,
  Verlauf,
  type Ecke,
  type Eisen,
  type Element as Stueck,
  type Form,
  type Muster,
  type P,
  type Teil,
} from "./modell";
import { abstandZuLinie, auf45Grad, bogenAusWoelbung } from "./pfad";
import { fehlertext, leseZahl, t, zahl, type TextSchluessel } from "./texte";

type Werkzeug = "auswahl" | "hand" | "rechteck" | "kreis" | "ellipse" | "tpocket" | Stift;
/** Werkzeuge, die Stück für Stück einen Pfad zeichnen. */
type Stift = "linie" | "bogen" | "bezier";
const istStift = (w: Werkzeug): w is Stift => w === "linie" || w === "bogen" || w === "bezier";

const WERKZEUGE: { id: Werkzeug; taste: string; icon: string }[] = [
  { id: "auswahl", taste: "V", icon: '<path d="M6 3l12 9-5 1.2 3 6-2.2 1-3-6L6 18z"/>' },
  { id: "hand", taste: "H", icon: '<path d="M8 11V5.5a1.5 1.5 0 013 0V10m0-5a1.5 1.5 0 013 0V10m0-3.5a1.5 1.5 0 013 0V14a6 6 0 01-6 6h-1a6 6 0 01-5-2.7L4.5 14a1.5 1.5 0 012.5-1.6L8 14"/>' },
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
const knopfExport = el("button", { type: "button", class: "primaer", title: `${t("export")} (Strg+P)`, "aria-keyshortcuts": "Control+P" }, t("export"));
const kopf = el(
  "header",
  { class: "kopf" },
  el("span", { class: "marke" }, "lepaca"),
  knopfZurueck,
  knopfVor,
  el("span", { class: "trenner" }),
  knopfAlles,
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
    ansicht.zeichnen(muster(), auswahl, vorschau, entwurfFuerAnsicht());
    statusZoom.textContent = t("status.zoom", { z: Math.round((ansicht.massstab / PX_PRO_MM) * 100) });
  });
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
    const form: Form = { art: "pfad", start: [0, 0], elemente, geschlossen };
    // Linien sind oft Markierungen: ohne Naht anlegen.
    m.teile.push({ id, name, form, x: x0, y: y0, naht: geschlossen ? { ...m.standardNaht } : null });
    auswahl = id;
  });
  stiftMeldung();
}

function allesAktualisieren(): void {
  geometrieLaden();
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
  for (const [id, b] of werkzeugKnoepfe) b.setAttribute("aria-pressed", String(id === w));
  canvas.dataset["werkzeug"] = w;
  stiftMeldung();
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
  // Fokus merken, damit Tippen nicht unterbrochen wird.
  const fokusIndex = [...eigenschaften.querySelectorAll("input,select,button")].indexOf(document.activeElement as Element);
  eigenschaften.replaceChildren();
  const g = gewaehlt();
  if (g) eigenschaften.append(...teilFelder(g));
  else eigenschaften.append(...musterFelder());
  if (fokusIndex >= 0) (eigenschaften.querySelectorAll("input,select,button")[fokusIndex] as HTMLElement | undefined)?.focus();
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
    felder.push(...pfadFelder(f));
  } else {
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
      auswahlFeld<Eisen>(t("naht.eisen"), n.eisen, [["french", t("eisen.french")], ["diamond", t("eisen.diamond")], ["japan", t("eisen.japan")]], (x) => aendern((m) => ((n.eisen = x), merke(m)))),
      auswahlFeld<Ecke>(t("naht.ecke"), n.ecke, [["ein", t("ecke.ein")], ["aus", t("ecke.aus")], ["mitte", t("ecke.mitte")]], (x) => aendern((m) => ((n.ecke = x), merke(m)))),
    );
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
  k.addEventListener("change", () => aendern(() => (f.geschlossen = k.checked)));
  felder.push(el("div", { class: "feld feld-zeile" }, k, el("label", { for: zu }, t("pfad.geschlossen"))));
  let nr = 0;
  for (const e of f.elemente) {
    if (e.art !== "bogen") continue;
    nr++;
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
  ];
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
    if (!p) continue;
    const g = p.geometrie;
    const nah = abstandZuLinie(g.geschlossen ? [...g.kontur, g.kontur[0]!] : g.kontur, mx - x.x, my - x.y) * ansicht.massstab <= 6;
    if (nah || (g.geschlossen && enthaelt(g.kontur, mx - x.x, my - x.y))) return x;
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
  } else if (werkzeug === "auswahl") {
    const treffer = trefferBei(mx, my);
    auswahl = treffer?.id ?? null;
    if (treffer) ziehen = { art: "verschieben", ...basis, teilX: treffer.x, teilY: treffer.y };
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
    g.x = runden(z.teilX + mx - z.mx, e.altKey);
    g.y = runden(z.teilY + my - z.my, e.altKey);
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

canvas.addEventListener("dblclick", () => {
  if (pfadEntwurf) pfadFertig(false);
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
    ansicht.allesZeigen(muster());
    neuZeichnen();
  } else if (taste === "+" || taste === "-") {
    ansicht.zoomen(taste === "+" ? 1.25 : 0.8, canvas.clientWidth / 2, canvas.clientHeight / 2);
    neuZeichnen();
  } else if ((taste === "delete" || taste === "backspace") && e.target === canvas) {
    loeschen();
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
  ansicht.allesZeigen(muster());
  neuZeichnen();
});
knopfExport.addEventListener("click", () => void exportieren());
knopfTest.addEventListener("click", () => void leistungstest());

new ResizeObserver(() => {
  ansicht.groesseAnpassen();
  neuZeichnen();
}).observe(canvas);

werkzeugWaehlen("auswahl");
allesAktualisieren();
