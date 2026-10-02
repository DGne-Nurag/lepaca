// Das Muster als reine Daten, plus Verlauf für Rückgängig/Wiederholen
// (Kernregel 4: jede Aktion ist rückgängig zu machen).

export type Eisen = "french" | "diamond" | "japan";
export type Ecke = "ein" | "aus" | "mitte";

export type Form =
  | { art: "rechteck"; breite: number; hoehe: number; eckenradius: number }
  | { art: "kreis"; durchmesser: number }
  | { art: "ellipse"; breite: number; hoehe: number };

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
}

export function neuesMuster(): Muster {
  return {
    teile: [],
    druck: { papier: "a4", rand: 10, klebekante: 10 },
    standardNaht: { kantenabstand: 3, lochabstand: 3.85, eisen: "diamond", ecke: "ein" },
  };
}

export function ausdehnung(f: Form): { breite: number; hoehe: number } {
  switch (f.art) {
    case "kreis":
      return { breite: f.durchmesser, hoehe: f.durchmesser };
    default:
      return { breite: f.breite, hoehe: f.hoehe };
  }
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
