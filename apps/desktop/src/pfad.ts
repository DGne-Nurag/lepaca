// Geometrie-Helfer für gezeichnete Pfade in der Oberfläche: nur für die
// Vorschau beim Zeichnen und für Bedienhilfen. Maßgeblich bleibt die
// Berechnung im Rust-Kern.

import type { Element, P } from "./modell";

/** Mittelpunkt und Winkel eines Bogens wie im Kern (`bogen_flach`);
 * `null`, wenn der Bogen unmöglich ist. */
export function bogenDaten(
  von: P,
  ziel: P,
  r: number,
  uhrzeigersinn: boolean,
  gross: boolean,
): { mitte: P; w0: number; w1: number } | null {
  const sx = ziel[0] - von[0];
  const sy = ziel[1] - von[1];
  const d = Math.hypot(sx, sy);
  if (d < 1e-6 || r < d / 2 - 1e-6) return null;
  const rr = Math.max(r, d / 2);
  const h = Math.sqrt(Math.max(0, rr * rr - (d * d) / 4));
  const m: P = [von[0] + sx / 2, von[1] + sy / 2];
  const q: P = [-sy / d, sx / d];
  const richtung = uhrzeigersinn ? 1 : -1;
  const spanne = (c: P) => {
    const w0 = Math.atan2(von[1] - c[1], von[0] - c[0]);
    const w1 = Math.atan2(ziel[1] - c[1], ziel[0] - c[0]);
    const s = ((((w1 - w0) * richtung) % (2 * Math.PI)) + 2 * Math.PI) % (2 * Math.PI);
    return { w0, s };
  };
  let mitte: P = [m[0] + q[0] * h, m[1] + q[1] * h];
  let { w0, s } = spanne(mitte);
  if (s > Math.PI + 1e-9 !== gross) {
    mitte = [m[0] - q[0] * h, m[1] - q[1] * h];
    ({ w0, s } = spanne(mitte));
  }
  return { mitte, w0, w1: w0 + richtung * s };
}

/** Bogen durch Start, Ziel und einen Wölbungspunkt: Radius aus der Pfeilhöhe
 * (Abstand des Punkts von der Sehne), Seite aus dem Vorzeichen. */
export function bogenAusWoelbung(
  von: P,
  ziel: P,
  maus: P,
): { radius: number; uhrzeigersinn: boolean; gross: boolean } | null {
  const sx = ziel[0] - von[0];
  const sy = ziel[1] - von[1];
  const d = Math.hypot(sx, sy);
  if (d < 1e-6) return null;
  // Vorzeichenbehafteter Abstand der Maus von der Sehne.
  const hoehe = ((maus[0] - von[0]) * -sy + (maus[1] - von[1]) * sx) / d;
  const s = Math.abs(hoehe);
  if (s < 0.05) return null;
  const radius = (s * s + (d * d) / 4) / (2 * s);
  // Wölbung nach links der Laufrichtung (negative Querachse) heißt auf dem
  // Bildschirm: im Uhrzeigersinn.
  return { radius, uhrzeigersinn: hoehe < 0, gross: s > d / 2 };
}

/** Hängt einen Pfad an einen Canvas-Pfad an (ohne moveTo zum Start). */
export function elementeZeichnen(c: CanvasPath, start: P, elemente: Element[]): void {
  let von = start;
  for (const e of elemente) {
    if (e.art === "linie") {
      c.lineTo(e.ziel[0], e.ziel[1]);
    } else if (e.art === "kurve") {
      c.bezierCurveTo(e.k1[0], e.k1[1], e.k2[0], e.k2[1], e.ziel[0], e.ziel[1]);
    } else {
      const b = bogenDaten(von, e.ziel, e.radius, e.uhrzeigersinn, e.gross);
      if (b) {
        c.arc(b.mitte[0], b.mitte[1], Math.max(e.radius, 0), b.w0, b.w1, !e.uhrzeigersinn);
      } else {
        c.lineTo(e.ziel[0], e.ziel[1]);
      }
    }
    von = e.ziel;
  }
}

/** Abstand eines Punkts zu einem Linienzug. */
export function abstandZuLinie(punkte: P[], x: number, y: number): number {
  let best = Infinity;
  for (let i = 1; i < punkte.length; i++) {
    const [ax, ay] = punkte[i - 1]!;
    const [bx, by] = punkte[i]!;
    const dx = bx - ax;
    const dy = by - ay;
    const l2 = dx * dx + dy * dy;
    const t = l2 > 0 ? Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / l2)) : 0;
    best = Math.min(best, Math.hypot(x - (ax + t * dx), y - (ay + t * dy)));
  }
  return best;
}

/** Richtung auf Vielfache von 45° einrasten (Umschalt beim Zeichnen). */
export function auf45Grad(von: P, nach: P): P {
  const dx = nach[0] - von[0];
  const dy = nach[1] - von[1];
  const w = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
  const l = Math.hypot(dx, dy);
  return [von[0] + Math.cos(w) * l, von[1] + Math.sin(w) * l];
}
