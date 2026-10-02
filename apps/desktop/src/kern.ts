// Aufrufe in den Rust-Kern. Geometrie wird pro Teil an der Position (0, 0)
// berechnet und zwischengespeichert; Verschieben braucht also keine neue
// Berechnung.

import { invoke } from "@tauri-apps/api/core";
import type { Druck, Teil } from "./modell";

type P = [number, number];

export interface Geometrie {
  kontur: P[];
  linie: P[];
  loecher: [P, P, P, P][];
  ecklocher: number;
  abstaende: number[];
  stichfehler: string | null;
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

function teilDaten(t: Teil, x: number, y: number) {
  return { form: t.form, x, y, stiche: t.naht };
}

function pfad(punkte: P[]): Path2D {
  const p = new Path2D();
  punkte.forEach(([x, y], i) => (i === 0 ? p.moveTo(x, y) : p.lineTo(x, y)));
  p.closePath();
  return p;
}

const cache = new Map<string, Pfade>();
const laufend = new Map<string, Promise<Pfade>>();

function schluessel(t: Teil): string {
  return JSON.stringify([t.form, t.naht]);
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
  const offen = laufend.get(s);
  if (offen) return offen;
  const p = invoke<Geometrie>("teil_geometrie", { teil: teilDaten(t, 0, 0) }).then((g) => {
    const loecher = new Path2D();
    for (const l of g.loecher) loecher.addPath(pfad(l));
    let min: P = [Infinity, Infinity];
    let max: P = [-Infinity, -Infinity];
    for (const [x, y] of g.kontur) {
      min = [Math.min(min[0], x), Math.min(min[1], y)];
      max = [Math.max(max[0], x), Math.max(max[1], y)];
    }
    const fertig: Pfade = { geometrie: g, kontur: pfad(g.kontur), linie: pfad(g.linie), loecher, min, max };
    if (cache.size > 2000) cache.clear();
    cache.set(s, fertig);
    laufend.delete(s);
    return fertig;
  });
  p.catch(() => laufend.delete(s));
  laufend.set(s, p);
  return p;
}

export function pdfExportieren(teile: Teil[], druck: Druck): Promise<ExportErgebnis> {
  return invoke<ExportErgebnis>("pdf_exportieren", {
    teile: teile.map((t) => teilDaten(t, t.x, t.y)),
    druck,
  });
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
