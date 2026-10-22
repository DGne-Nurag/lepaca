// lepaca – Pattern Viewer mit erster Werkzeugleiste.

// Schrift wird mit der App ausgeliefert (offline, auf allen Plattformen gleich).
import "@fontsource-variable/ibm-plex-sans/wght.css";

import { Ansicht, lage, PX_PRO_MM, type Darstellung, type Entwurf, type Vorschau } from "./ansicht";
import {
  ausschnitteLoesen,
  enthaelt,
  formFehler,
  montageAktualisieren,
  pdfExportieren,
  pfadeFuer,
  pfadeSofort,
  skizzeLoesen,
  skizzeSofort,
  uebernommeneSofort,
  uhrarmbandErzeugen,
  vorlageLaden,
  type Vorlage,
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
  type Abstandsmass,
  type Ausschnitt,
  type Bezug,
  type Ecke,
  type Eisen,
  type Bedingung,
  type Druck,
  type Element as Stueck,
  type Form,
  type Muster,
  type P,
  type PfadForm,
  type Teil,
} from "./modell";
import { abstandZuLinie, auf45Grad, bogenAusWoelbung, kantenLaenge, kantePunkte } from "./pfad";
import { fehlertext, leseZahl, t, zahl, type TextSchluessel } from "./texte";
import type { Geometrie } from "./kern";

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
  /** Gewählte Ausschnitte (für Abstandsmaße). */
  ausschnitte: number[];
}
interface Treffer {
  kante: number | null;
  punkt: number | null;
  ausschnitt: number | null;
}
const leereWahl = (): SkizzenWahl => ({ kanten: [], punkte: [], ausschnitte: [] });
const keinTreffer = (): Treffer => ({ kante: null, punkt: null, ausschnitt: null });
let skizzenWahl: SkizzenWahl = leereWahl();
let skizzenHover: Treffer = keinTreffer();
/** Maß, dessen Feld nach dem Neuaufbau den Fokus bekommt. */
let fokusBedingung: number | null = null;
let fokusAbstand: number | null = null;

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

/** Knopf mit Symbol statt Text; Name für Screenreader und Tooltip. */
function symbolKnopf(name: string, taste: string, kuerzel: string, inhalt: string): HTMLButtonElement {
  const b = el("button", { type: "button", class: "symbol-knopf", title: `${name} (${taste})`, "aria-label": name, "aria-keyshortcuts": kuerzel });
  b.append(icon(inhalt));
  return b;
}

const knopfZurueck = symbolKnopf(t("rueckgaengig"), "Strg+Z", "Control+Z", '<path d="M9 14L4 9l5-5"/><path d="M4 9h11a5 5 0 010 10h-3"/>');
const knopfVor = symbolKnopf(t("wiederholen"), "Strg+Umschalt+Z", "Control+Shift+Z", '<path d="M15 14l5-5-5-5"/><path d="M20 9H9a5 5 0 000 10h3"/>');
const knopfAlles = symbolKnopf(t("alles.zeigen"), "0", "0", '<path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/>');
const knopfMasse = el("button", { type: "button", class: "schalter", "aria-pressed": "true", title: `${t("ansicht.masse.titel")} (M)`, "aria-keyshortcuts": "M" }, t("ansicht.masse"));
// Zuschnitt und Zusammenbau sind zwei Ansichten desselben Musters; Z wechselt.
const knopfZuschnitt = el("button", { type: "button", "aria-pressed": "true", title: `${t("ansicht.zuschnitt.titel")} (Z)` }, t("ansicht.zuschnitt"));
const knopfZusammenbau = el("button", { type: "button", "aria-pressed": "false", title: `${t("ansicht.zusammenbau.titel")} (Z)`, "aria-keyshortcuts": "Z" }, t("ansicht.zusammenbau"));
const knopfExport = el(
  "button",
  { type: "button", class: "primaer", title: `${t("export")} (Strg+P)`, "aria-label": t("export"), "aria-keyshortcuts": "Control+P" },
  el("span", { class: "lang" }, t("export")),
  el("span", { class: "kurz" }, t("export.kurz")),
);
// ---------------------------------------------------------------- Vorlagen

/** Zuletzt eingegebene Maße des Uhrarmbands (mm), bleiben in der Sitzung. */
const uhrarmbandMasse = { stegbreite: 22, armumfang: 180, hornabstand: 48 };

/** Menü „Vorlagen“: Bifold einsetzen oder ein Uhrarmband erzeugen. */
function vorlagenMenue(): HTMLDetailsElement {
  const bifold = el("button", { type: "button", title: t("vorlage.bifold.titel") }, t("vorlage.einsetzen"));
  bifold.addEventListener("click", () => {
    menue.open = false;
    void vorlageEinsetzen();
  });

  const fehler = el("p", { class: "meldung fehler", role: "alert" });
  const feld = (schluessel: "stegbreite" | "armumfang" | "hornabstand", einheit: string, faktor: number) => {
    const id = `vorlage-${schluessel}`;
    const eingabe = el("input", { id, type: "text", inputmode: "decimal", spellcheck: "false", value: zahl(uhrarmbandMasse[schluessel] / faktor) });
    const lesen = (): number | null => {
      const x = leseZahl(eingabe.value);
      return x === null ? null : x * faktor;
    };
    const knoten = el(
      "div",
      { class: "feld" },
      el("label", { for: id, title: t(`vorlage.${schluessel}.titel`) }, t(`vorlage.${schluessel}`)),
      el("span", { class: "mit-einheit" }, eingabe, el("span", { class: "einheit" }, einheit)),
    );
    return { knoten, eingabe, lesen };
  };
  // Den Armumfang misst man mit dem Maßband in Zentimetern.
  const felder = {
    stegbreite: feld("stegbreite", "mm", 1),
    armumfang: feld("armumfang", "cm", 10),
    hornabstand: feld("hornabstand", "mm", 1),
  };
  const erzeugen = el("button", { type: "submit", class: "primaer" }, t("vorlage.erzeugen"));
  const form = el(
    "form",
    { class: "vorlage-form" },
    felder.stegbreite.knoten,
    felder.armumfang.knoten,
    felder.hornabstand.knoten,
    fehler,
    el("div", { class: "knopfzeile knopfzeile-eng" }, erzeugen),
  );
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    for (const f of Object.values(felder)) f.eingabe.removeAttribute("aria-invalid");
    fehler.textContent = "";
    const werte = { stegbreite: felder.stegbreite.lesen(), armumfang: felder.armumfang.lesen(), hornabstand: felder.hornabstand.lesen() };
    for (const [k, x] of Object.entries(werte)) {
      if (x === null) {
        felder[k as keyof typeof felder].eingabe.setAttribute("aria-invalid", "true");
        fehler.textContent = t("eingabe.zahl");
        return;
      }
    }
    const { stegbreite, armumfang, hornabstand } = werte as typeof uhrarmbandMasse;
    uhrarmbandErzeugen(stegbreite, armumfang, hornabstand)
      .then((v) => {
        Object.assign(uhrarmbandMasse, { stegbreite, armumfang, hornabstand });
        menue.open = false;
        return vorlageAnwenden(v, t("vorlage.uhrarmband.name", { s: zahl(stegbreite) }), t("vorlage.uhrarmband.geladen"));
      })
      .catch((err: unknown) => {
        const code = String(err);
        fehler.textContent = fehlertext(code);
        const k = code.replace("vorlage.", "");
        if (k in felder) {
          const f = felder[k as keyof typeof felder].eingabe;
          f.setAttribute("aria-invalid", "true");
          f.focus();
        }
      });
  });

  const menue = el(
    "details",
    { class: "vorlagen" },
    el("summary", { title: t("vorlagen.titel") }, t("vorlagen")),
    el(
      "div",
      { class: "vorlagen-feld" },
      el("section", {}, el("h2", {}, t("vorlage.bifold")), el("p", { class: "meldung" }, t("vorlage.bifold.info")), el("div", { class: "knopfzeile knopfzeile-eng" }, bifold)),
      el("section", {}, el("h2", {}, t("vorlage.uhrarmband")), el("p", { class: "meldung" }, t("vorlage.uhrarmband.info")), form),
    ),
  );
  // Esc und ein Klick daneben schließen das Menü.
  menue.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && menue.open) {
      e.stopPropagation();
      menue.open = false;
      menue.querySelector("summary")?.focus();
    }
  });
  document.addEventListener("pointerdown", (e) => {
    if (menue.open && !menue.contains(e.target as Node)) menue.open = false;
  });
  menue.addEventListener("toggle", () => {
    if (menue.open) felder.stegbreite.eingabe.focus();
  });
  return menue;
}

// Zeichen: ein Lederteil mit Stichlinie.
const marke = el("span", { class: "marke" });
marke.append(icon('<rect x="2.5" y="4.5" width="19" height="15" rx="3.5" class="marke-leder"/><rect x="5.5" y="7.5" width="13" height="9" rx="1.5" class="marke-naht"/>'), el("span", { class: "marke-text" }, "lepaca"));
const musterTitel = el("h1", { class: "muster-titel" });
const musterInfo = el("span", { class: "muster-info" });
const kopf = el(
  "header",
  { class: "kopf" },
  marke,
  el("div", { class: "titel-block" }, musterTitel, musterInfo),
  el("div", { class: "ansicht-wahl" }, el("div", { class: "segment", role: "group", "aria-label": t("ansicht") }, knopfZuschnitt, knopfZusammenbau), knopfMasse, knopfAlles),
  el("div", { class: "kopf-rechts" }, knopfZurueck, knopfVor, el("span", { class: "trenner" }), vorlagenMenue(), knopfExport),
);

const canvas = el("canvas", { class: "flaeche", tabindex: "0", role: "img" });
const leerHinweis = el("p", { class: "leer-hinweis" }, t("leer"));
const mitte = el("main", { class: "mitte" }, canvas, leerHinweis);

const teileListe = el("ul", { class: "teile-liste", role: "listbox", "aria-label": t("teile") });
const teileAnzahl = el("span", { class: "anzahl" });
const eigenschaften = el("div", { class: "eigenschaften" });
const seite = el(
  "aside",
  { class: "seite" },
  el("h2", { class: "mit-anzahl" }, t("teile"), teileAnzahl),
  teileListe,
  eigenschaften,
);

const statusPosition = el("span", { class: "status-position" });
const statusZoom = el("span", { class: "status-zoom", title: "Zoom" });
const statusMeldung = el("span", { class: "status-meldung", role: "status", "aria-live": "polite" });
// Der Leistungstest (ADR 0002) ist ein Werkzeug für Entwickler: sichtbar nur
// im Entwicklungsmodus, sonst über Strg+Alt+Umschalt+L erreichbar.
const knopfTest = el("button", { type: "button", class: "klein", title: `${t("leistungstest.titel")} (Strg+Alt+Umschalt+L)` }, t("leistungstest"));
const fuss = el("footer", { class: "fuss" }, statusZoom, statusMeldung, el("span", { class: "fueller" }), statusPosition);
if (import.meta.env.DEV) fuss.append(knopfTest);

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
    werkzeug === "bemassen" && g && f && bemassbar(g) && !zusammenbau
      ? {
          teil: g.id,
          kanten: skizzenWahl.kanten,
          punkte: skizzenWahl.punkte,
          hoverKante: skizzenHover.kante,
          hoverPunkt: skizzenHover.punkt,
          freieKanten: f.art === "pfad" ? (skizzeSofort(f)?.freieKanten ?? null) : null,
          ausschnitte: skizzenWahl.ausschnitte,
          hoverAusschnitt: skizzenHover.ausschnitt,
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
    .catch((e: unknown) => fehlerMelden(e));
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
    .catch((e: unknown) => fehlerMelden(e));
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
  knopfZuschnitt.setAttribute("aria-pressed", String(!zusammenbau));
  knopfZusammenbau.setAttribute("aria-pressed", String(zusammenbau));
  const n = muster().teile.length;
  musterTitel.textContent = muster().name ?? t("muster.neu");
  musterInfo.textContent = n === 1 ? t("teile.eins") : t("teile.anzahl", { n });
  teileAnzahl.textContent = n > 0 ? String(n) : "";
  listeAktualisieren();
  eigenschaftenAktualisieren();
  knopfZurueck.disabled = !verlauf.kannZurueck;
  knopfVor.disabled = !verlauf.kannVor;
  leerHinweis.hidden = muster().teile.length > 0;
  canvas.setAttribute("aria-label", `${t("muster")}: ${muster().teile.map((x) => x.name).join(", ") || t("teile.keine")}`);
  neuZeichnen();
}

/** Meldung in der Statuszeile; technische Details nur im Tooltip. */
function melden(text: string, details = ""): void {
  statusMeldung.textContent = text;
  statusMeldung.title = details;
}

/** Fehler aus dem Kern melden; der technische Code steht im Tooltip. */
function fehlerMelden(e: unknown): void {
  melden(fehlertext(String(e)), String(e));
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
  skizzenWahl = leereWahl();
  skizzenHover = keinTreffer();
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
    const li = el("li", { role: "option", tabindex: "0", "aria-selected": String(x.id === auswahl) }, ...teilZeile(x));
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

/** Umriss als kleines Bild, Name und Außenmaße eines Teils für die Liste. */
function teilZeile(x: Teil): Node[] {
  const p = pfadeSofort(x);
  const bild = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  bild.setAttribute("class", "umriss");
  bild.setAttribute("aria-hidden", "true");
  const masse = el("span", { class: "teil-masse" });
  if (p && p.geometrie.kontur.length > 1) {
    const [x0, y0] = p.min;
    const b = p.max[0] - x0;
    const h = p.max[1] - y0;
    // Etwas Rand, damit die Linie nicht abgeschnitten wird.
    const r = Math.max(b, h) * 0.06;
    bild.setAttribute("viewBox", `${x0 - r} ${y0 - r} ${b + 2 * r} ${h + 2 * r}`);
    const pfad = document.createElementNS("http://www.w3.org/2000/svg", "path");
    const d = p.geometrie.kontur.map(([px, py], i) => `${i === 0 ? "M" : "L"}${px} ${py}`).join("");
    pfad.setAttribute("d", p.geometrie.geschlossen ? `${d}Z` : d);
    if (!p.geometrie.geschlossen) pfad.setAttribute("class", "offen");
    bild.append(pfad);
    // Offene Linien (z. B. eine Aufnähnaht) haben eine Länge, keine Fläche.
    const k = p.geometrie.kontur;
    const laenge = k.slice(1).reduce((l, q, i) => l + Math.hypot(q[0] - k[i]![0], q[1] - k[i]![1]), 0);
    masse.textContent = p.geometrie.geschlossen ? `${zahl(b, 1)} × ${zahl(h, 1)} mm` : `${zahl(laenge, 1)} mm`;
  }
  return [bild, el("span", { class: "teil-name" }, x.name), masse];
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
  if (fokusBedingung !== null || fokusAbstand !== null) {
    const feld = eigenschaften.querySelector<HTMLInputElement>(
      fokusBedingung !== null ? `[data-bedingung="${fokusBedingung}"] input` : `[data-abstand="${fokusAbstand}"] input`,
    );
    fokusBedingung = null;
    fokusAbstand = null;
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
  felder.push(...ausschnittFelder(g), ...ausstanzenFelder(g));

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
      if (geo.loecherAmAusschnitt > 0) {
        felder.push(el("p", { class: "meldung fehler", role: "alert" }, t("naht.amAusschnitt", { n: geo.loecherAmAusschnitt })));
      }
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
  const papiere: [Druck["papier"], string][] = [["a4", "A4"], ["a3", "A3"], ["letter", "US Letter"]];
  // Druckeinstellungen braucht man erst beim Export: zugeklappt mit
  // Zusammenfassung, damit die Nähanleitung oben steht.
  const druck = el(
    "details",
    { class: "druck" },
    el(
      "summary",
      {},
      el("span", { class: "druck-titel" }, t("druck")),
      el("span", { class: "druck-werte" }, t("druck.zusammenfassung", { p: papiere.find(([v]) => v === d.papier)?.[1] ?? d.papier, r: zahl(d.rand), k: zahl(d.klebekante) })),
    ),
    auswahlFeld(t("druck.papier"), d.papier, papiere, (x) => aendern((m) => (m.druck.papier = x))),
    zahlFeld(t("druck.rand"), d.rand, (x) => aendern((m) => (m.druck.rand = x))),
    zahlFeld(t("druck.klebekante"), d.klebekante, (x) => aendern((m) => (m.druck.klebekante = x))),
  );
  druck.open = druckOffen;
  druck.addEventListener("toggle", () => (druckOffen = druck.open));
  return [...anleitungFelder(), druck];
}

/** Druckeinstellungen aufgeklappt (bleibt beim Neuaufbau erhalten). */
let druckOffen = false;

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
    fehlerMelden(err);
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
  // Mit einem Ausschnitt in der Wahl geht es um Abstandsmaße.
  if (w.ausschnitte.length > 0) return aus;
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
      if (skizzenWahl.ausschnitte.length === 0) felder.push(el("p", { class: "meldung" }, t("skizze.waehlen")));
    } else {
      const zeile = el("div", { class: "knopfzeile knopfzeile-eng bedingungen-knoepfe" });
      for (const m of moeglich) {
        const b = el("button", { type: "button" }, t(m.text));
        b.addEventListener("click", () => {
          const neu = [...f.bedingungen, m.b];
          skizzenWahl = leereWahl();
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

/** Treffer auf einen Punkt (Vorrang), einen Ausschnitt oder eine Kante. */
function skizzenTreffer(g: Teil, mx: number, my: number): Treffer {
  const f = g.form;
  const x = mx - g.x;
  const y = my - g.y;
  const fang = 7 / ansicht.massstab;
  const geo = pfadeSofort(g)?.geometrie;
  let best = fang;
  if (f.art === "pfad") {
    let punkt: number | null = null;
    pfadPunkte(f)
      .slice(0, punktzahl(f))
      .forEach((q, i) => {
        const d = Math.hypot(q[0] - x, q[1] - y);
        if (d <= best) {
          best = d;
          punkt = i;
        }
      });
    if (punkt !== null) return { ...keinTreffer(), punkt };
  }
  // Ausschnitte: innen oder nah am Rand.
  let ausschnitt: number | null = null;
  best = fang;
  geo?.ausschnitte.forEach((a, i) => {
    const d = enthaelt(a, x, y) ? 0 : abstandZuLinie([...a, a[0]!], x, y);
    if (d <= best) {
      best = d;
      ausschnitt = i;
    }
  });
  if (ausschnitt !== null) return { ...keinTreffer(), ausschnitt };
  let kanteNr: number | null = null;
  best = fang;
  if (f.art === "pfad") {
    for (let k = 0; k < kantenzahl(f); k++) {
      const d = abstandZuLinie(kantePunkte(f, k), x, y);
      if (d <= best) {
        best = d;
        kanteNr = k;
      }
    }
  } else {
    geo?.geradeKanten.forEach((k, i) => {
      if (!k) return;
      const d = abstandZuLinie(k, x, y);
      if (d <= best) {
        best = d;
        kanteNr = i;
      }
    });
  }
  return { ...keinTreffer(), kante: kanteNr };
}

/** Bemaßen: Klick wählt Punkte, Kanten und Ausschnitte (bis zu zwei),
 * ein Klick auf ein anderes Teil wählt dieses. */
function bemassenKlick(mx: number, my: number): void {
  const g = gewaehlt();
  const treffer = g ? skizzenTreffer(g, mx, my) : null;
  if (g && treffer && (treffer.punkt !== null || treffer.kante !== null || treffer.ausschnitt !== null)) {
    const art = treffer.punkt !== null ? "punkte" : treffer.ausschnitt !== null ? "ausschnitte" : "kanten";
    const i = (treffer.punkt ?? treffer.ausschnitt ?? treffer.kante)!;
    const liste = skizzenWahl[art];
    if (liste.includes(i)) liste.splice(liste.indexOf(i), 1);
    else {
      const w = skizzenWahl;
      if (w.punkte.length + w.kanten.length + w.ausschnitte.length >= 2) skizzenWahl = leereWahl();
      skizzenWahl[art].push(i);
    }
  } else {
    const anderes = trefferBei(mx, my);
    auswahl = anderes?.id ?? null;
    skizzenWahl = leereWahl();
    if (anderes && !bemassbar(anderes)) melden(t("skizze.nurPfad"));
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
  const m = moeglicheBedingungen(f, { ...leereWahl(), kanten: [tr.kante] }).find((x) => istMass(x.b));
  if (!m) return;
  skizzenWahl = leereWahl();
  const neu = [...f.bedingungen, m.b];
  void skizzeSetzen(g, neu, neu.length - 1);
}

// ---------------------------------------------------------------- Ausschnitte

/** Lässt sich am Teil etwas bemaßen (Skizze oder Ausschnitte)? */
function bemassbar(g: Teil): boolean {
  return g.form.art === "pfad" || (g.ausschnitte?.length ?? 0) > 0;
}

const istOffen = (f: Form) => f.art === "pfad" && !f.geschlossen;
const kreisRadius = (a: Ausschnitt | undefined) => (a?.form.art === "kreis" ? a.form.durchmesser / 2 : null);
const rundeMass = (x: number) => Math.round(x * 100) / 100;

/** Der Wert, den ein Abstandsmaß in der jetzigen Zeichnung hat; null,
 * wenn es nicht passt (z. B. Rand zu Rand zwischen Nicht-Kreisen). */
function gemessen(g: Teil, geo: Geometrie, m: Abstandsmass): number | null {
  const liste = g.ausschnitte ?? [];
  if (m.art === "kante") {
    const k = geo.geradeKanten[m.kante];
    const c = geo.ausschnittMitten[m.ausschnitt];
    const umriss = geo.ausschnitte[m.ausschnitt];
    if (!k || !c || !umriss) return null;
    const [p0, p1] = k;
    const l = Math.hypot(p1[0] - p0[0], p1[1] - p0[1]);
    if (l < 1e-9) return null;
    const n: P = [-(p1[1] - p0[1]) / l, (p1[0] - p0[0]) / l];
    const d = (q: P) => (q[0] - p0[0]) * n[0] + (q[1] - p0[1]) * n[1];
    const mitte = Math.abs(d(c));
    if (m.bezug === "mitte") return mitte;
    const r = kreisRadius(liste[m.ausschnitt]);
    const s = d(c) < 0 ? -1 : 1;
    return r !== null ? mitte - r : Math.min(...umriss.map((q) => s * d(q)));
  }
  const ca = geo.ausschnittMitten[m.a];
  const cb = geo.ausschnittMitten[m.b];
  const ua = geo.ausschnitte[m.a];
  const ub = geo.ausschnitte[m.b];
  if (!ca || !cb || !ua || !ub) return null;
  if (m.richtung === "direkt") {
    const d = Math.hypot(cb[0] - ca[0], cb[1] - ca[1]);
    if (m.bezug === "mitte") return d;
    const ra = kreisRadius(liste[m.a]);
    const rb = kreisRadius(liste[m.b]);
    return ra !== null && rb !== null ? d - ra - rb : null;
  }
  const achse = m.richtung === "waagerecht" ? 0 : 1;
  const d = cb[achse] - ca[achse];
  if (m.bezug === "mitte") return Math.abs(d);
  const min = (u: P[]) => Math.min(...u.map((q) => q[achse]));
  const max = (u: P[]) => Math.max(...u.map((q) => q[achse]));
  return d >= 0 ? min(ub) - max(ua) : min(ua) - max(ub);
}

/** Abstandsmaße, die zur Wahl passen, mit den gezeichneten Werten; immer
 * zur Mitte und zum Rand. */
function moeglicheAbstaende(g: Teil, w: SkizzenWahl): { text: string; m: Abstandsmass }[] {
  const geo = pfadeSofort(g)?.geometrie;
  if (!geo || w.punkte.length > 0) return [];
  const kandidaten: Abstandsmass[] = [];
  const bezuege: Bezug[] = ["mitte", "rand"];
  if (w.ausschnitte.length === 1 && w.kanten.length === 1) {
    const [ausschnitt, kante] = [w.ausschnitte[0]!, w.kanten[0]!];
    for (const bezug of bezuege) kandidaten.push({ art: "kante", ausschnitt, kante, bezug, wert: 0 });
  } else if (w.ausschnitte.length === 2 && w.kanten.length === 0) {
    const [a, b] = w.ausschnitte as [number, number];
    for (const richtung of ["waagerecht", "senkrecht", "direkt"] as const) {
      for (const bezug of bezuege) kandidaten.push({ art: "zwischen", a, b, richtung, bezug, wert: 0 });
    }
  }
  const aus: { text: string; m: Abstandsmass }[] = [];
  for (const m of kandidaten) {
    const wert = gemessen(g, geo, m);
    if (wert === null || wert <= 0.005) continue;
    m.wert = rundeMass(wert);
    const art = m.art === "kante" ? "kante" : m.richtung;
    aus.push({ text: t(`abstand.${art}.${m.bezug}` as TextSchluessel, { w: zahl(m.wert) }), m });
  }
  return aus;
}

/** Bezeichnung eines Abstandsmaßes; Kanten ab 1 gezählt. */
function abstandText(g: Teil, m: Abstandsmass): string {
  const name = (i: number) => g.ausschnitte?.[i]?.name ?? String(i + 1);
  if (m.art === "kante") return t("abstand.kante.text", { a: name(m.ausschnitt), k: m.kante + 1 });
  return t(`abstand.${m.richtung}.text` as TextSchluessel, { a: name(m.a), b: name(m.b) });
}

/** Neue Abstandsmaße lösen lassen; übernehmen nur, wenn alle Ausschnitte
 * danach noch im Teil liegen und kein Maß doppelt ist. */
async function abstandSetzen(g: Teil, masse: Abstandsmass[], fokus: number | null = null): Promise<boolean> {
  const stand = (x: Teil) => JSON.stringify([x.form, x.ausschnitte, x.abstandsmasse]);
  const vorher = stand(g);
  const versuch: Teil = { ...structuredClone(g), abstandsmasse: masse };
  try {
    const e = await ausschnitteLoesen(versuch);
    if (e.ueberbestimmt) {
      melden(t("skizze.ueberbestimmt"));
      eigenschaftenAktualisieren();
      neuZeichnen();
      return false;
    }
    const jetzt = muster().teile.find((x) => x.id === g.id);
    if (!jetzt || stand(jetzt) !== vorher) return false;
    fokusAbstand = fokus;
    aendern(() => {
      jetzt.abstandsmasse = masse;
      jetzt.ausschnitte?.forEach((a, i) => {
        const p = e.positionen[i];
        if (p) [a.x, a.y] = p;
      });
    });
    melden(e.freiheitsgrade === 0 ? t("abstand.bestimmt") : t("abstand.frei"));
    return true;
  } catch (err) {
    fehlerMelden(err);
    eigenschaftenAktualisieren();
    neuZeichnen();
    return false;
  }
}

/** Maße ohne Ausschnitt `i`; spätere Ausschnitte rücken eins vor. */
function ohneAusschnitt(masse: Abstandsmass[], i: number): Abstandsmass[] {
  const neu = (j: number) => (j > i ? j - 1 : j);
  return masse
    .filter((m) => (m.art === "kante" ? m.ausschnitt !== i : m.a !== i && m.b !== i))
    .map((m) => (m.art === "kante" ? { ...m, ausschnitt: neu(m.ausschnitt) } : { ...m, a: neu(m.a), b: neu(m.b) }));
}

/** Ausschnitt entfernen; mit `loesen` wird er wieder ein eigenes Teil. */
function ausschnittEntfernen(g: Teil, i: number, loesen: boolean): void {
  const lage = pfadeSofort(g)?.geometrie.ausschnittLagen[i];
  aendern((m) => {
    const a = g.ausschnitte?.[i];
    if (!a) return;
    g.ausschnitte = g.ausschnitte!.filter((_, j) => j !== i);
    g.abstandsmasse = ohneAusschnitt(g.abstandsmasse ?? [], i);
    if (loesen) {
      const id = neueId(m);
      const [x, y] = lage ?? [a.x, a.y];
      m.teile.push({ id, name: a.name, form: a.form, x: g.x + x, y: g.y + y, naht: null });
      auswahl = id;
    }
  });
}

/** Teil `g` aus Teil `zielId` ausstanzen: Es wird ein Ausschnitt dort, wo
 * es gerade liegt. Der Kern prüft vorher, ob es ganz im Teil liegt. */
async function ausstanzen(g: Teil, zielId: number): Promise<void> {
  const ziel = muster().teile.find((x) => x.id === zielId);
  if (!ziel) return;
  const neu: Ausschnitt = { name: g.name, form: structuredClone(g.form), x: g.x - ziel.x, y: g.y - ziel.y };
  const versuch: Teil = { ...structuredClone(ziel), ausschnitte: [...(ziel.ausschnitte ?? []), neu] };
  try {
    await pfadeFuer(versuch);
  } catch (e) {
    fehlerMelden(e);
    return;
  }
  aendern((m) => {
    const z = m.teile.find((x) => x.id === zielId);
    if (!z) return;
    m.teile = m.teile.filter((x) => x.id !== g.id);
    z.ausschnitte = [...(z.ausschnitte ?? []), neu];
    auswahl = zielId;
  });
  melden(t("ausstanzen.fertig", { a: g.name, z: ziel.name }));
}

/** Abschnitt „Ausstanzen“: dieses Teil aus einem anderen ausstanzen. */
function ausstanzenFelder(g: Teil): HTMLElement[] {
  if (istOffen(g.form) || (g.ausschnitte?.length ?? 0) > 0) return [];
  const ziele = muster().teile.filter((x) => x.id !== g.id && !istOffen(x.form));
  if (ziele.length === 0) return [];
  // Vorschlag: das Teil, auf dem dieses gerade liegt.
  const p = pfadeSofort(g);
  const mitte: P = p ? [g.x + (p.min[0] + p.max[0]) / 2, g.y + (p.min[1] + p.max[1]) / 2] : [g.x, g.y];
  const darunter = [...ziele].reverse().find((x) => {
    const q = pfadeSofort(x)?.geometrie;
    return q && enthaelt(q.kontur, mitte[0] - x.x, mitte[1] - x.y);
  });
  let zielId = (darunter ?? ziele[0]!).id;
  const wahl = auswahlFeld(
    t("ausstanzen.aus"),
    String(zielId),
    ziele.map((x) => [String(x.id), x.name] as [string, string]),
    (x) => (zielId = Number(x)),
  );
  const knopf = el("button", { type: "button", title: t("ausstanzen.titel") }, t("ausstanzen"));
  knopf.addEventListener("click", () => void ausstanzen(g, zielId));
  return [el("h3", {}, t("ausstanzen")), wahl, el("div", { class: "knopfzeile knopfzeile-eng" }, knopf)];
}

/** Abschnitt „Ausschnitte“: Größe, Lage und Abstandsmaße. */
function ausschnittFelder(g: Teil): HTMLElement[] {
  const liste = g.ausschnitte ?? [];
  if (liste.length === 0) return [];
  const geo = pfadeSofort(g)?.geometrie;
  const felder: HTMLElement[] = [el("h3", {}, t("ausschnitte"))];
  const fest = geo ? geo.ausschnittFrei.every((x) => !x) : false;
  felder.push(el("p", { class: fest ? "meldung gut" : "meldung" }, !geo ? t("skizze.unbekannt") : fest ? t("abstand.bestimmt") : t("abstand.frei")));

  if (werkzeug !== "bemassen") {
    // Bei gezeichneten Teilen steht der Knopf schon bei der Skizze.
    if (g.form.art !== "pfad") {
      const b = el("button", { type: "button", title: `${t("werkzeug.bemassen")} (D)` }, t("skizze.bemassen"));
      b.addEventListener("click", () => werkzeugWaehlen("bemassen"));
      felder.push(el("div", { class: "knopfzeile knopfzeile-eng" }, b));
    }
  } else {
    const moeglich = moeglicheAbstaende(g, skizzenWahl);
    if (moeglich.length > 0) {
      const zeile = el("div", { class: "knopfzeile knopfzeile-eng bedingungen-knoepfe" });
      for (const m of moeglich) {
        const b = el("button", { type: "button" }, m.text);
        b.addEventListener("click", () => {
          const neu = [...(g.abstandsmasse ?? []), m.m];
          skizzenWahl = leereWahl();
          void abstandSetzen(g, neu, neu.length - 1);
        });
        zeile.append(b);
      }
      felder.push(zeile);
    } else if (g.form.art !== "pfad" || skizzenWahl.ausschnitte.length > 0) {
      felder.push(el("p", { class: "meldung" }, t("abstand.waehlen")));
    }
  }

  liste.forEach((a, i) => {
    const f = a.form;
    felder.push(el("p", { class: "feld-titel" }, `${i + 1}. ${a.name}`));
    if (f.art === "kreis") {
      felder.push(zahlFeld(t("teil.durchmesser"), f.durchmesser, (x) => aendern(() => (f.durchmesser = x))));
    } else if (f.art === "rechteck" || f.art === "ellipse") {
      felder.push(
        zahlFeld(t("teil.breite"), f.breite, (x) => aendern(() => (f.breite = x))),
        zahlFeld(t("teil.hoehe"), f.hoehe, (x) => aendern(() => (f.hoehe = x))),
      );
      if (f.art === "rechteck") felder.push(zahlFeld(t("teil.radius"), f.eckenradius, (x) => aendern(() => (f.eckenradius = x))));
    }
    const lage = geo?.ausschnittLagen[i] ?? [a.x, a.y];
    felder.push(
      zahlFeld(t("ausschnitt.x"), lage[0], (x) => aendern(() => ([a.x, a.y] = [x, lage[1]]))),
      zahlFeld(t("ausschnitt.y"), lage[1], (y) => aendern(() => ([a.x, a.y] = [lage[0], y]))),
    );
    const loesen = el("button", { type: "button", title: t("ausschnitt.loesen.titel") }, t("ausschnitt.loesen"));
    loesen.addEventListener("click", () => ausschnittEntfernen(g, i, true));
    const weg = el("button", { type: "button", class: "gefahr" }, t("ausschnitt.entfernen"));
    weg.addEventListener("click", () => ausschnittEntfernen(g, i, false));
    felder.push(el("div", { class: "knopfzeile knopfzeile-eng" }, loesen, weg));
  });

  const masse = g.abstandsmasse ?? [];
  if (masse.length > 0) {
    felder.push(el("p", { class: "feld-titel" }, t("abstand.masse")));
    masse.forEach((m, i) => {
      const aendereMass = (neu: Abstandsmass) => {
        const alle = structuredClone(masse);
        alle[i] = neu;
        void abstandSetzen(g, alle);
      };
      const feld = zahlFeld(abstandText(g, m), m.wert, (x) => aendereMass({ ...m, wert: x }));
      // Wovon aus gemessen wird. Beim Umschalten bleibt der Ausschnitt,
      // wo er ist, und der Wert passt sich an.
      const bezugWahl = auswahlFeld<Bezug>(t("abstand.bezug"), m.bezug, [["mitte", t("abstand.mitte")], ["rand", t("abstand.rand")]], (b) => {
        const neu = { ...m, bezug: b };
        const w = geo ? gemessen(g, geo, neu) : null;
        if (w === null || w <= 0) {
          melden(fehlertext("ausschnitt.unpassend"));
          eigenschaftenAktualisieren();
          return;
        }
        aendereMass({ ...neu, wert: rundeMass(w) });
      });
      // Immer beide Werte zeigen: den gewählten im Feld, den anderen hier.
      const anders = geo ? gemessen(g, geo, { ...m, bezug: m.bezug === "mitte" ? "rand" : "mitte" }) : null;
      const hinweis =
        anders !== null && anders > 0
          ? [el("p", { class: "meldung" }, t(m.bezug === "mitte" ? "abstand.randIst" : "abstand.mitteIst", { w: zahl(anders) }))]
          : [];
      const knopf = el("button", { type: "button", class: "klein entfernen", title: t("skizze.entfernen"), "aria-label": t("skizze.entfernen") }, "×");
      knopf.addEventListener("click", () => void abstandSetzen(g, masse.filter((_, j) => j !== i)));
      felder.push(el("div", { class: "mass-zeile", "data-abstand": String(i) }, feld, knopf), bezugWahl, ...hinweis);
    });
  }
  return felder;
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
    await vorlageAnwenden(await vorlageLaden("bifold"), t("vorlage.bifold.name"), t("vorlage.geladen"));
  } catch (e) {
    fehlerMelden(e);
  }
}

/** Ersetzt das Muster durch eine Vorlage (rückgängig machbar). */
async function vorlageAnwenden(v: Vorlage, name: string, meldung: string): Promise<void> {
  aendern((m) => {
    const teile: Teil[] = [];
    for (const x of v.teile) {
      teile.push({
        id: neueId({ ...m, teile }),
        name: x.name ?? "",
        form: x.form,
        x: x.x,
        y: x.y,
        naht: x.stiche,
        montage: x.montage,
        // Ausschnitte (z. B. Dornlöcher) heißen nach ihrer Nummer, wenn
        // die Vorlage keinen Namen mitbringt.
        ausschnitte: x.ausschnitte.map((a, i) => ({ ...a, name: a.name ?? t("ausschnitt.nummer", { n: i + 1 }) })),
        abstandsmasse: x.abstandsmasse,
      });
    }
    m.teile = teile;
    m.anleitung = v.schritte;
    m.name = name;
  });
  auswahl = null;
  // Erst die Geometrie, dann passt „Alles zeigen“.
  await Promise.all(muster().teile.map(pfadeFuer));
  ansicht.allesZeigen(muster(), zusammenbau);
  melden(meldung);
  allesAktualisieren();
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
    const [px, py] = [mx - ort[0], my - ort[1]];
    const nah = abstandZuLinie(g.geschlossen ? [...g.kontur, g.kontur[0]!] : g.kontur, px, py) * ansicht.massstab <= 6;
    // Im Loch eines Ausschnitts ist kein Leder, außer man trifft seinen Rand.
    const imLoch = g.ausschnitte.some((a) => enthaelt(a, px, py) && abstandZuLinie([...a, a[0]!], px, py) * ansicht.massstab > 6);
    if (nah || (g.geschlossen && enthaelt(g.kontur, px, py) && !imLoch)) return x;
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
    const neu = g && !zusammenbau ? skizzenTreffer(g, mx, my) : keinTreffer();
    if (neu.kante !== skizzenHover.kante || neu.punkt !== skizzenHover.punkt || neu.ausschnitt !== skizzenHover.ausschnitt) {
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
  if (skizzenHover.kante === null && skizzenHover.punkt === null && skizzenHover.ausschnitt === null) return;
  skizzenHover = keinTreffer();
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
  if (strg && e.altKey && e.shiftKey && taste === "l") {
    e.preventDefault();
    if (!knopfTest.disabled) void leistungstest();
  } else if (strg && taste === "z") {
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
  } else if (taste === "escape" && werkzeug === "bemassen" && skizzenWahl.kanten.length + skizzenWahl.punkte.length + skizzenWahl.ausschnitte.length > 0) {
    skizzenWahl = leereWahl();
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
  skizzenWahl = leereWahl();
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
    fehlerMelden(e);
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
    fehlerMelden(e);
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
knopfZuschnitt.addEventListener("click", () => zusammenbau && zusammenbauUmschalten());
knopfZusammenbau.addEventListener("click", () => zusammenbau || zusammenbauUmschalten());
knopfExport.addEventListener("click", () => void exportieren());
knopfTest.addEventListener("click", () => void leistungstest());

new ResizeObserver(() => {
  ansicht.groesseAnpassen();
  neuZeichnen();
}).observe(canvas);

// Maßzahlen auf der Fläche erst mit der geladenen Schrift neu zeichnen.
document.fonts.addEventListener("loadingdone", neuZeichnen);

werkzeugWaehlen("auswahl");
allesAktualisieren();
