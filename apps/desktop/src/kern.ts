// Aufrufe in den Rust-Kern. Geometrie wird pro Teil an der Position (0, 0)
// berechnet und zwischengespeichert; Verschieben braucht also keine neue
// Berechnung.

import { invoke } from "@tauri-apps/api/core";
import type { Abstandsmass, Ausschnitt, Druck, Element, Form, Montage, Naht, P, Teil } from "./modell";

export interface MassGrafik {
  /** Index der Bedingung; null bei Maßen einer Grundform. */
  bedingung: number | null;
  striche: [P, P][];
  pfeile: [P, P][];
  text: string;
  textPos: P;
  textWinkel: number;
}

export interface SymbolGrafik {
  bedingung: number;
  pos: P;
  text: string;
}

export interface Geometrie {
  kontur: P[];
  /** false: offene Linie, nicht füllen. */
  geschlossen: boolean;
  linien: { punkte: P[]; geschlossen: boolean }[];
  loecher: P[][];
  ecklocher: number;
  abstaende: number[];
  stichfehler: string | null;
  masse: MassGrafik[];
  symbole: SymbolGrafik[];
  /** Umrisse der Ausschnitte, relativ zum Teil. */
  ausschnitte: P[][];
  /** Gelöste Position je Ausschnitt (relativ zum Teil). */
  ausschnittLagen: P[];
  /** Mittelpunkt je Ausschnitt (relativ zum Teil). */
  ausschnittMitten: P[];
  /** Je Ausschnitt, ob er sich noch verschieben lässt. */
  ausschnittFrei: boolean[];
  /** Gerade Kanten des Teils für Abstandsmaße; null: gebogen. */
  geradeKanten: ([P, P] | null)[];
  /** Stichlöcher, die in einem Ausschnitt oder zu nah daran liegen. */
  loecherAmAusschnitt: number;
}

export interface Pfade {
  geometrie: Geometrie;
  kontur: Path2D;
  linie: Path2D;
  loecher: Path2D;
  /** Umschließendes Rechteck relativ zur Teilposition. */
  min: P;
  max: P;
}

export interface ExportErgebnis {
  abgebrochen: boolean;
  blaetter: number;
  datei: string;
}

function teilDaten(t: Teil, x: number, y: number, mitMontage = false) {
  return {
    name: mitMontage ? t.name : null,
    form: t.form,
    x,
    y,
    stiche: t.naht,
    montage: mitMontage ? (t.montage ?? null) : null,
    ausschnitte: t.ausschnitte ?? [],
    abstandsmasse: t.abstandsmasse ?? [],
  };
}

function pfad(punkte: P[], geschlossen = true): Path2D {
  const p = new Path2D();
  punkte.forEach(([x, y], i) => (i === 0 ? p.moveTo(x, y) : p.lineTo(x, y)));
  if (geschlossen) p.closePath();
  return p;
}

const cache = new Map<string, Pfade>();
/** Fehlercodes des Kerns je Form, z. B. ein unmöglicher Bogenradius. */
const fehler = new Map<string, string>();
const laufend = new Map<string, Promise<Pfade>>();

function schluessel(t: Teil): string {
  return JSON.stringify([t.form, t.naht, t.ausschnitte ?? [], t.abstandsmasse ?? []]);
}

/** Fehlercode, falls der Kern die Form abgelehnt hat. */
export function formFehler(t: Teil): string | undefined {
  return fehler.get(schluessel(t));
}

/** Geometrie aus dem Zwischenspeicher, falls vorhanden. */
export function pfadeSofort(t: Teil): Pfade | undefined {
  return cache.get(schluessel(t));
}

/** Geometrie berechnen lassen (oder aus dem Zwischenspeicher). */
export async function pfadeFuer(t: Teil): Promise<Pfade> {
  const s = schluessel(t);
  const da = cache.get(s);
  if (da) return da;
  const abgelehnt = fehler.get(s);
  if (abgelehnt) throw abgelehnt;
  const offen = laufend.get(s);
  if (offen) return offen;
  const p = invoke<Geometrie>("teil_geometrie", { teil: teilDaten(t, 0, 0) }).then((g) => {
    const linie = new Path2D();
    for (const l of g.linien) linie.addPath(pfad(l.punkte, l.geschlossen));
    const loecher = new Path2D();
    for (const l of g.loecher) loecher.addPath(pfad(l));
    let min: P = [Infinity, Infinity];
    let max: P = [-Infinity, -Infinity];
    for (const [x, y] of g.kontur) {
      min = [Math.min(min[0], x), Math.min(min[1], y)];
      max = [Math.max(max[0], x), Math.max(max[1], y)];
    }
    // Ausschnitte gehören zur Schnittkante; gefüllt wird mit „evenodd“.
    const kontur = pfad(g.kontur, g.geschlossen);
    for (const a of g.ausschnitte) kontur.addPath(pfad(a));
    const fertig: Pfade = { geometrie: g, kontur, linie, loecher, min, max };
    if (cache.size > 2000) cache.clear();
    cache.set(s, fertig);
    laufend.delete(s);
    return fertig;
  });
  p.catch((e: unknown) => {
    laufend.delete(s);
    if (fehler.size > 2000) fehler.clear();
    fehler.set(s, String(e));
  });
  laufend.set(s, p);
  return p;
}

export function pdfExportieren(teile: Teil[], druck: Druck): Promise<ExportErgebnis> {
  return invoke<ExportErgebnis>("pdf_exportieren", {
    teile: teile.map((t) => teilDaten(t, t.x, t.y, true)),
    druck,
  });
}

// ---------------------------------------------------------------- Skizze

export interface SkizzenErgebnis {
  elemente: Element[];
  freiheitsgrade: number;
  ueberbestimmt: boolean;
  freiePunkte: boolean[];
  freieKanten: boolean[];
}

const skizzen = new Map<string, SkizzenErgebnis>();

/** Löst die Skizze; wirft den Fehlercode des Kerns. */
export async function skizzeLoesen(form: Extract<Form, { art: "pfad" }>): Promise<SkizzenErgebnis> {
  const s = JSON.stringify(form);
  const da = skizzen.get(s);
  if (da) return da;
  const e = await invoke<SkizzenErgebnis>("skizze_loesen", { form });
  if (skizzen.size > 500) skizzen.clear();
  skizzen.set(s, e);
  // Das gelöste Ergebnis hat denselben Zustand.
  skizzen.set(JSON.stringify({ ...form, elemente: e.elemente }), e);
  return e;
}

/** Zustand der Skizze aus dem Zwischenspeicher. */
export function skizzeSofort(form: Extract<Form, { art: "pfad" }>): SkizzenErgebnis | undefined {
  return skizzen.get(JSON.stringify(form));
}

// ---------------------------------------------------------------- Ausschnitte

export interface AusschnittErgebnis {
  positionen: P[];
  freiheitsgrade: number;
  ueberbestimmt: boolean;
  frei: boolean[];
}

/** Richtet die Ausschnitte nach ihren Maßen aus; wirft den Fehlercode des
 * Kerns, z. B. wenn ein Ausschnitt dann aus dem Teil ragt. */
export function ausschnitteLoesen(t: Teil): Promise<AusschnittErgebnis> {
  return invoke<AusschnittErgebnis>("ausschnitte_loesen", { teil: teilDaten(t, 0, 0) });
}

// ---------------------------------------------------------------- Zusammenbau

export interface MontageErgebnis {
  loecher: P[][];
  kantennah: number;
  zuDicht: number;
}

export interface Uebernommen {
  loecher: Path2D;
  anzahl: number;
  kantennah: number;
  zuDicht: number;
}

let montageSchluessel = "";
let montageStand = new Map<number, Uebernommen>();
let montageLaeuft: Promise<void> | null = null;

/** Teile, die zum Zusammenbau gehören, und alles, was ihre Löcher bestimmt. */
function montageTeile(teile: Teil[]): Teil[] {
  return teile.filter((t) => t.montage);
}

/** Übernommene Löcher je Teil-ID, falls schon berechnet. */
export function uebernommeneSofort(t: Teil): Uebernommen | undefined {
  return montageStand.get(t.id);
}

/** Übernommene Löcher für alle Teile berechnen lassen. Liefert true, wenn
 * sich etwas geändert hat. */
export async function montageAktualisieren(teile: Teil[]): Promise<boolean> {
  const beteiligt = montageTeile(teile);
  const s = JSON.stringify(beteiligt.map((t) => [t.id, t.form, t.naht, t.x, t.y, t.montage, t.ausschnitte, t.abstandsmasse]));
  if (s === montageSchluessel) {
    await montageLaeuft;
    return false;
  }
  montageSchluessel = s;
  const lauf = (async () => {
    if (beteiligt.length === 0) {
      montageStand = new Map();
      return;
    }
    const e = await invoke<MontageErgebnis[]>("montage_loecher", {
      teile: beteiligt.map((t) => teilDaten(t, t.x, t.y, true)),
    });
    // Nur übernehmen, wenn inzwischen nichts Neueres angefragt wurde.
    if (s !== montageSchluessel) return;
    const neu = new Map<number, Uebernommen>();
    beteiligt.forEach((t, i) => {
      const r = e[i];
      if (!r) return;
      const p = new Path2D();
      for (const l of r.loecher) p.addPath(pfad(l));
      neu.set(t.id, { loecher: p, anzahl: r.loecher.length, kantennah: r.kantennah, zuDicht: r.zuDicht });
    });
    montageStand = neu;
  })();
  montageLaeuft = lauf;
  await lauf;
  return true;
}

// ---------------------------------------------------------------- Vorlagen

/** Teil, wie es aus dem Kern kommt (Naht heißt dort „stiche“). */
export interface VorlageTeil {
  name: string | null;
  form: Form;
  x: number;
  y: number;
  stiche: Naht | null;
  montage: Montage | null;
  ausschnitte: (Omit<Ausschnitt, "name"> & { name: string | null })[];
  abstandsmasse: Abstandsmass[];
}

export interface Vorlage {
  teile: VorlageTeil[];
  schritte: string[];
}

export function vorlageLaden(name: string): Promise<Vorlage> {
  return invoke<Vorlage>("vorlage", { name });
}

/** Punkt-in-Polygon (gerade/ungerade Regel). */
export function enthaelt(punkte: P[], x: number, y: number): boolean {
  let drin = false;
  for (let i = 0, j = punkte.length - 1; i < punkte.length; j = i++) {
    const [xi, yi] = punkte[i]!;
    const [xj, yj] = punkte[j]!;
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) drin = !drin;
  }
  return drin;
}
