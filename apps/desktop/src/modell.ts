// Das Muster als reine Daten, plus Verlauf für Rückgängig/Wiederholen
// (Kernregel 4: jede Aktion ist rückgängig zu machen).

export type Eisen = "french" | "diamond" | "japan" | "round";
export type Ecke = "ein" | "aus" | "mitte";

export type P = [number, number];

/** Ein Stück eines gezeichneten Pfads; Koordinaten relativ zum Teil (mm). */
export type Element =
  | { art: "linie"; ziel: P }
  | { art: "kurve"; k1: P; k2: P; ziel: P }
  | { art: "bogen"; ziel: P; radius: number; uhrzeigersinn: boolean; gross: boolean };

/** Maß oder Bedingung einer Skizze. Punkt 0 ist der Start, Punkt i + 1 das
 * Ziel von Element i; Kante i ist Element i, bei geschlossenem Pfad ohne
 * Rückweg zum Start ist Kante n die Schlusslinie. */
export type Bedingung =
  | { art: "laenge"; kante: number; wert: number }
  | { art: "abstand"; a: number; b: number; wert: number }
  | { art: "abstandX"; a: number; b: number; wert: number }
  | { art: "abstandY"; a: number; b: number; wert: number }
  | { art: "radius"; kante: number; wert: number }
  | { art: "winkel"; a: number; b: number; grad: number }
  | { art: "horizontal"; kante: number }
  | { art: "vertikal"; kante: number }
  | { art: "parallel"; a: number; b: number }
  | { art: "senkrecht"; a: number; b: number }
  | { art: "gleich"; a: number; b: number }
  | { art: "tangential"; a: number; b: number }
  | { art: "fest"; punkt: number; x: number; y: number };

/** Bedingungen mit Zahlenwert (Maße). */
export type Mass = Extract<Bedingung, { wert: number } | { grad: number }>;
export const istMass = (b: Bedingung): b is Mass => "wert" in b || "grad" in b;

export type Form =
  | { art: "rechteck"; breite: number; hoehe: number; eckenradius: number }
  | { art: "kreis"; durchmesser: number }
  | { art: "ellipse"; breite: number; hoehe: number }
  | {
      art: "tpocket";
      breite: number;
      hoehe: number;
      fluegelUeberstand: number;
      fluegelHoehe: number;
      nahtFluegel: boolean;
      nahtSteg: boolean;
      nahtOeffnung: boolean;
    }
  | {
      art: "pfad";
      start: P;
      elemente: Element[];
      geschlossen: boolean;
      bedingungen: Bedingung[];
      /** Je Kante, ob dort genäht wird; fehlt: ringsum. */
      naehte?: boolean[];
    };

export type PfadForm = Extract<Form, { art: "pfad" }>;

/** Wovon aus ein Abstand gemessen wird: Mittelpunkt oder Rand (Umfang). */
export type Bezug = "mitte" | "rand";
export type Richtung = "direkt" | "waagerecht" | "senkrecht";

/** Eine Form, die aus dem Teil ausgestanzt wird (Loch, Fenster). Position
 * der Form relativ zum Teil, mm. */
export interface Ausschnitt {
  name: string;
  form: Form;
  x: number;
  y: number;
}

/** Maß, das einen Ausschnitt im Teil festlegt. Kanten zählen wie die
 * geraden Kanten des Teils (Rechteck: oben, rechts, unten, links). */
export type Abstandsmass =
  | { art: "kante"; ausschnitt: number; kante: number; bezug: Bezug; wert: number }
  | { art: "zwischen"; a: number; b: number; richtung: Richtung; bezug: Bezug; wert: number };

/** Lage im fertigen Stück und Nähschritt. */
export interface Montage {
  x: number;
  y: number;
  schritt: number;
}

export interface Naht {
  kantenabstand: number;
  lochabstand: number;
  eisen: Eisen;
  ecke: Ecke;
}

export interface Teil {
  id: number;
  name: string;
  form: Form;
  /** Linke obere Ecke im Muster, mm. */
  x: number;
  y: number;
  naht: Naht | null;
  /** Lage im Zusammenbau; Nähte gehen durch alle Lagen, die beim Nähen
   * schon da sind. */
  montage?: Montage | null;
  /** Ausgestanzte Formen. */
  ausschnitte?: Ausschnitt[];
  /** Maße, die die Ausschnitte festlegen. */
  abstandsmasse?: Abstandsmass[];
}

export interface Druck {
  papier: "a4" | "a3" | "letter";
  rand: number;
  klebekante: number;
}

export interface Muster {
  teile: Teil[];
  druck: Druck;
  /** Nahteinstellung für neue Teile: die zuletzt benutzte. */
  standardNaht: Naht;
  /** Nähanleitung, z. B. aus einer Vorlage. */
  anleitung?: string[];
}

export function neuesMuster(): Muster {
  return {
    teile: [],
    druck: { papier: "a4", rand: 10, klebekante: 10 },
    standardNaht: { kantenabstand: 3, lochabstand: 3.85, eisen: "diamond", ecke: "ein" },
  };
}

/** Startwerte einer T-Pocket (Springfield Leather: 4" × 1¾", Flügel ½"). */
export function neueTPocket(breite = 101.6, hoehe = 44.5): Form {
  return {
    art: "tpocket",
    breite,
    hoehe,
    fluegelUeberstand: Math.min(12.7, Math.max(1, (breite - 10) / 2 - 0.1)),
    fluegelHoehe: Math.min(12, hoehe / 3),
    nahtFluegel: true,
    nahtSteg: true,
    nahtOeffnung: false,
  };
}

export function ausdehnung(f: Form): { breite: number; hoehe: number } {
  switch (f.art) {
    case "kreis":
      return { breite: f.durchmesser, hoehe: f.durchmesser };
    case "pfad": {
      // Grob über alle Punkte; nur für die Anzeige.
      const xs = [f.start[0], ...f.elemente.map((e) => e.ziel[0])];
      const ys = [f.start[1], ...f.elemente.map((e) => e.ziel[1])];
      return { breite: Math.max(...xs) - Math.min(...xs), hoehe: Math.max(...ys) - Math.min(...ys) };
    }
    default:
      return { breite: f.breite, hoehe: f.hoehe };
  }
}

/** Punkte eines Pfads: Start und das Ziel jedes Elements. */
export function pfadPunkte(f: PfadForm): P[] {
  return [f.start, ...f.elemente.map((e) => e.ziel)];
}

/** Endet der Pfad nicht am Start, kommt beim Schließen eine Linie dazu. */
export function hatSchlusslinie(f: PfadForm): boolean {
  const z = f.elemente.at(-1)?.ziel;
  return !z || Math.hypot(z[0] - f.start[0], z[1] - f.start[1]) >= 1e-9;
}

export function kantenzahl(f: PfadForm): number {
  return f.elemente.length + (f.geschlossen && hatSchlusslinie(f) ? 1 : 0);
}

/** Kante k als Punktindizes und Art; die Schlusslinie ist eine Linie. */
export function kante(f: PfadForm, k: number): { von: number; bis: number; element: Element } {
  const n = f.elemente.length;
  if (k < n) {
    const amStart = f.geschlossen && k === n - 1 && !hatSchlusslinie(f);
    return { von: k, bis: amStart ? 0 : k + 1, element: f.elemente[k]! };
  }
  return { von: n, bis: 0, element: { art: "linie", ziel: f.start } };
}

/** Zahl der Punkte, die eine Bedingung nennen darf (der letzte Punkt eines
 * geschlossenen Pfads, der am Start endet, ist der Start selbst). */
export function punktzahl(f: PfadForm): number {
  return f.elemente.length + 1 - (f.geschlossen && !hatSchlusslinie(f) ? 1 : 0);
}

const MAX_SCHRITTE = 200;

/** Verlauf über vollständige Zustände: einfach und robust. */
export class Verlauf {
  private zurueck: Muster[] = [];
  private vor: Muster[] = [];

  constructor(public muster: Muster) {}

  /** Vor jeder Änderung aufrufen. */
  merken(): void {
    this.zurueck.push(structuredClone(this.muster));
    if (this.zurueck.length > MAX_SCHRITTE) this.zurueck.shift();
    this.vor = [];
  }

  rueckgaengig(): boolean {
    const m = this.zurueck.pop();
    if (!m) return false;
    this.vor.push(this.muster);
    this.muster = m;
    return true;
  }

  wiederholen(): boolean {
    const m = this.vor.pop();
    if (!m) return false;
    this.zurueck.push(this.muster);
    this.muster = m;
    return true;
  }

  get kannZurueck(): boolean {
    return this.zurueck.length > 0;
  }

  get kannVor(): boolean {
    return this.vor.length > 0;
  }
}

let naechsteId = 1;

export function neueId(muster: Muster): number {
  naechsteId = Math.max(naechsteId, ...muster.teile.map((t) => t.id + 1));
  return naechsteId++;
}
