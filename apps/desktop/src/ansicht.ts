// Zeichenfläche: Ansicht (Zoom, Verschiebung) und Zeichnen auf Canvas.
// Gezeichnet wird alles in einem Canvas statt als einzelne SVG-Elemente,
// damit die Leistung nicht an der Anzahl der Löcher hängt (ADR 0002).

import { pfadeSofort, uebernommeneSofort, type Pfade } from "./kern";
import { kantenzahl, pfadPunkte, punktzahl, type Element, type Muster, type P, type Teil } from "./modell";
import { elementeZeichnen, kanteZeichnen } from "./pfad";

/** Was außer den Teilen zu sehen ist. */
export interface Darstellung {
  /** Maße der Skizzen und Grundformen zeigen. */
  masse: boolean;
  /** Teile in ihrer Lage im Zusammenbau statt im Muster zeigen. */
  zusammenbau: boolean;
  /** Bemaßen: Hervorhebungen in der Skizze des gewählten Teils. */
  skizze: SkizzenAnzeige | null;
}

export interface SkizzenAnzeige {
  teil: number;
  kanten: number[];
  punkte: number[];
  hoverKante: number | null;
  hoverPunkt: number | null;
  /** Je Kante, ob sie noch frei beweglich ist (vom Löser). */
  freieKanten: boolean[] | null;
}

/** Wo ein Teil gezeichnet wird: im Muster oder im Zusammenbau. */
export function lage(t: Teil, zusammenbau: boolean): P | null {
  if (!zusammenbau) return [t.x, t.y];
  return t.montage ? [t.montage.x, t.montage.y] : null;
}

/** Pfad, der gerade gezeichnet wird, mit dem Stück bis zur Maus. */
export interface Entwurf {
  start: P;
  elemente: Element[];
  /** Vorschau des nächsten Stücks (folgt der Maus). */
  naechstes: Element | null;
  /** Bézier-Griffe zum Anzeigen: Ankerpunkt und Griffende. */
  griffe: [P, P][];
  /** Maus ist nah genug am Start zum Schließen. */
  schliessbar: boolean;
}

/** CSS-Pixel pro Millimeter bei „100 %“ (96 dpi). Der echte Bildschirm
 * kann abweichen; maßgenau ist nur der Druck. */
export const PX_PRO_MM = 96 / 25.4;

export interface Vorschau {
  art: "rechteck" | "kreis" | "ellipse" | "tpocket";
  x: number;
  y: number;
  breite: number;
  hoehe: number;
}

export class Ansicht {
  /** CSS-Pixel pro mm. */
  massstab = PX_PRO_MM * 1.5;
  /** Bildschirmposition (CSS-Pixel) des Muster-Nullpunkts. */
  versatzX = 80;
  versatzY = 60;
  private ctx: CanvasRenderingContext2D;
  private breite = 0;
  private hoehe = 0;

  constructor(readonly canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext("2d", { alpha: false });
    if (!ctx) throw new Error("Canvas 2D nicht verfügbar");
    this.ctx = ctx;
  }

  groesseAnpassen(): void {
    const r = this.canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    this.breite = r.width;
    this.hoehe = r.height;
    this.canvas.width = Math.max(1, Math.round(r.width * dpr));
    this.canvas.height = Math.max(1, Math.round(r.height * dpr));
  }

  zuMuster(sx: number, sy: number): [number, number] {
    return [(sx - this.versatzX) / this.massstab, (sy - this.versatzY) / this.massstab];
  }

  /** Zoomen um einen Bildschirmpunkt, der dabei an Ort und Stelle bleibt. */
  zoomen(faktor: number, sx: number, sy: number): void {
    const neu = Math.min(200, Math.max(0.05, this.massstab * faktor));
    const [mx, my] = this.zuMuster(sx, sy);
    this.massstab = neu;
    this.versatzX = sx - mx * neu;
    this.versatzY = sy - my * neu;
  }

  allesZeigen(muster: Muster, zusammenbau = false): void {
    let min = [Infinity, Infinity];
    let max = [-Infinity, -Infinity];
    for (const t of muster.teile) {
      const p = pfadeSofort(t);
      const ort = lage(t, zusammenbau);
      if (!p || !ort) continue;
      min = [Math.min(min[0]!, ort[0] + p.min[0]), Math.min(min[1]!, ort[1] + p.min[1])];
      max = [Math.max(max[0]!, ort[0] + p.max[0]), Math.max(max[1]!, ort[1] + p.max[1])];
    }
    if (!Number.isFinite(min[0]!)) {
      min = [0, 0];
      max = [210, 297];
    }
    const rand = 40;
    const b = max[0]! - min[0]!;
    const h = max[1]! - min[1]!;
    this.massstab = Math.min(200, Math.max(0.05, Math.min((this.breite - 2 * rand) / b, (this.hoehe - 2 * rand) / h)));
    this.versatzX = (this.breite - b * this.massstab) / 2 - min[0]! * this.massstab;
    this.versatzY = (this.hoehe - h * this.massstab) / 2 - min[1]! * this.massstab;
  }

  zeichnen(
    muster: Muster,
    auswahl: number | null,
    vorschau: Vorschau | null,
    entwurf: Entwurf | null = null,
    darstellung: Darstellung = { masse: false, zusammenbau: false, skizze: null },
  ): void {
    const c = this.ctx;
    const stil = getComputedStyle(this.canvas);
    const farbe = (n: string) => stil.getPropertyValue(n).trim() || "#000";
    const dpr = window.devicePixelRatio || 1;

    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    c.fillStyle = farbe("--flaeche");
    c.fillRect(0, 0, this.breite, this.hoehe);

    this.raster(farbe("--raster"), farbe("--raster-stark"));

    // Ab hier in Millimetern.
    const s = this.massstab;
    c.setTransform(dpr * s, 0, 0, dpr * s, dpr * this.versatzX, dpr * this.versatzY);
    const px = 1 / s;
    const [sichtX0, sichtY0] = this.zuMuster(0, 0);
    const [sichtX1, sichtY1] = this.zuMuster(this.breite, this.hoehe);

    // Im Zusammenbau in Nähreihenfolge und durchscheinend, damit alle
    // Lagen zu sehen sind.
    const teile = darstellung.zusammenbau
      ? muster.teile.filter((t) => t.montage).sort((a, b) => a.montage!.schritt - b.montage!.schritt)
      : muster.teile;
    for (const t of teile) {
      const p = pfadeSofort(t);
      const ort = lage(t, darstellung.zusammenbau);
      if (!p || !ort) continue;
      // Teile außerhalb der Sicht überspringen (Maße ragen etwas heraus).
      const r = darstellung.masse ? 30 : 0;
      if (ort[0] + p.max[0] + r < sichtX0 || ort[0] + p.min[0] - r > sichtX1 || ort[1] + p.max[1] + r < sichtY0 || ort[1] + p.min[1] - r > sichtY1) {
        continue;
      }
      const skizze = darstellung.skizze?.teil === t.id ? darstellung.skizze : null;
      this.teil(t, ort, p, t.id === auswahl, px, farbe, darstellung.zusammenbau, skizze);
    }
    if (darstellung.masse && !darstellung.zusammenbau) {
      for (const t of muster.teile) {
        const p = pfadeSofort(t);
        if (p) this.masse(t, p, t.id === auswahl, farbe);
      }
    }

    if (entwurf) this.entwurf(entwurf, px, farbe);

    if (vorschau) {
      c.save();
      c.setLineDash([4 * px, 3 * px]);
      c.lineWidth = 1.5 * px;
      c.strokeStyle = farbe("--auswahl");
      c.beginPath();
      if (vorschau.art === "rechteck") {
        c.rect(vorschau.x, vorschau.y, vorschau.breite, vorschau.hoehe);
      } else if (vorschau.art === "tpocket") {
        const { x, y, breite: b, hoehe: h } = vorschau;
        const u = Math.min(12.7, Math.max(0, (b - 10) / 2));
        const fh = Math.min(12, h / 3);
        c.moveTo(x, y);
        c.lineTo(x + b, y);
        c.lineTo(x + b, y + fh);
        c.lineTo(x + b - u, y + fh);
        c.lineTo(x + b - u, y + h);
        c.lineTo(x + u, y + h);
        c.lineTo(x + u, y + fh);
        c.lineTo(x, y + fh);
        c.closePath();
      } else {
        c.ellipse(
          vorschau.x + vorschau.breite / 2,
          vorschau.y + vorschau.hoehe / 2,
          vorschau.breite / 2,
          vorschau.hoehe / 2,
          0,
          0,
          Math.PI * 2,
        );
      }
      c.stroke();
      c.restore();
    }
  }

  private entwurf(e: Entwurf, px: number, farbe: (n: string) => string): void {
    const c = this.ctx;
    c.save();
    c.lineWidth = 1.5 * px;
    c.strokeStyle = farbe("--kante");
    c.beginPath();
    c.moveTo(e.start[0], e.start[1]);
    elementeZeichnen(c, e.start, e.elemente);
    c.stroke();
    if (e.naechstes) {
      const letzter = e.elemente.at(-1)?.ziel ?? e.start;
      c.setLineDash([4 * px, 3 * px]);
      c.strokeStyle = farbe("--auswahl");
      c.beginPath();
      c.moveTo(letzter[0], letzter[1]);
      elementeZeichnen(c, letzter, [e.naechstes]);
      c.stroke();
      c.setLineDash([]);
    }
    // Griffe und Ankerpunkte.
    c.strokeStyle = farbe("--auswahl");
    c.fillStyle = farbe("--flaeche");
    c.lineWidth = 1 * px;
    for (const [a, g] of e.griffe) {
      c.beginPath();
      c.moveTo(a[0], a[1]);
      c.lineTo(g[0], g[1]);
      c.stroke();
      c.beginPath();
      c.arc(g[0], g[1], 3 * px, 0, Math.PI * 2);
      c.fill();
      c.stroke();
    }
    const anker = [e.start, ...e.elemente.map((x) => x.ziel)];
    for (const [i, a] of anker.entries()) {
      const r = (i === 0 && e.schliessbar ? 6 : 3.5) * px;
      c.fillStyle = i === 0 && e.schliessbar ? farbe("--auswahl") : farbe("--flaeche");
      c.fillRect(a[0] - r, a[1] - r, 2 * r, 2 * r);
      c.strokeRect(a[0] - r, a[1] - r, 2 * r, 2 * r);
    }
    c.restore();
  }

  private teil(
    t: Teil,
    ort: P,
    p: Pfade,
    gewaehlt: boolean,
    px: number,
    farbe: (n: string) => string,
    durchscheinend: boolean,
    skizze: SkizzenAnzeige | null,
  ): void {
    const c = this.ctx;
    c.save();
    c.translate(ort[0], ort[1]);
    if (p.geometrie.geschlossen) {
      c.fillStyle = farbe("--leder");
      if (durchscheinend) c.globalAlpha = 0.45;
      c.fill(p.kontur);
      c.globalAlpha = 1;
    }
    c.lineWidth = (gewaehlt ? 2.5 : 1.25) * px;
    c.strokeStyle = gewaehlt ? farbe("--auswahl") : farbe("--kante");
    const f = t.form;
    if (skizze && f.art === "pfad") {
      // Skizze: Kanten einzeln, freie in Blau, bestimmte dunkel.
      const n = kantenzahl(f);
      for (let k = 0; k < n; k++) {
        const frei = skizze.freieKanten?.[k] ?? false;
        const markiert = skizze.kanten.includes(k) || skizze.hoverKante === k;
        c.beginPath();
        kanteZeichnen(c, f, k);
        c.lineWidth = (markiert ? 4 : 2) * px;
        c.strokeStyle = markiert ? farbe("--markiert") : frei ? farbe("--frei") : farbe("--kante");
        c.stroke();
      }
      // Der letzte Punkt eines geschlossenen Pfads ist oft der Start selbst.
      const punkte = pfadPunkte(f).slice(0, punktzahl(f));
      punkte.forEach((q, i) => {
        const markiert = skizze.punkte.includes(i) || skizze.hoverPunkt === i;
        const r = (markiert ? 5 : 3) * px;
        c.fillStyle = markiert ? farbe("--markiert") : farbe("--flaeche");
        c.strokeStyle = farbe("--kante");
        c.lineWidth = 1 * px;
        c.fillRect(q[0] - r, q[1] - r, 2 * r, 2 * r);
        c.strokeRect(q[0] - r, q[1] - r, 2 * r, 2 * r);
      });
    } else {
      c.stroke(p.kontur);
    }

    const uebernommen = uebernommeneSofort(t);
    const zuKlein = (t.naht ? t.naht.lochabstand : 3.85) * this.massstab < 4;
    if (p.geometrie.linien.length > 0 || uebernommen) {
      if (!zuKlein) {
        // Nah genug: jedes Loch zeichnen.
        c.fillStyle = farbe("--loch");
        c.fill(p.loecher);
        if (uebernommen) c.fill(uebernommen.loecher);
      } else {
        // Weit weg: Detailstufe, nur die Nahtlinie.
        c.setLineDash([3 * px, 2 * px]);
        c.lineWidth = 1 * px;
        c.strokeStyle = farbe("--loch");
        c.stroke(p.linie);
        if (uebernommen) {
          c.fillStyle = farbe("--loch");
          c.fill(uebernommen.loecher);
        }
      }
    }
    c.restore();
  }

  /** Maße und (beim gewählten Teil) Symbole der Bedingungen. Linien in mm,
   * Schrift in festen Bildschirmpixeln. */
  private masse(t: Teil, p: Pfade, gewaehlt: boolean, farbe: (n: string) => string): void {
    const g = p.geometrie;
    if (g.masse.length === 0 && (!gewaehlt || g.symbole.length === 0)) return;
    const c = this.ctx;
    const dpr = window.devicePixelRatio || 1;
    const s = this.massstab;
    const bild = (q: P): P => [(t.x + q[0]) * s + this.versatzX, (t.y + q[1]) * s + this.versatzY];
    c.save();
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    const linie = gewaehlt ? farbe("--mass-stark") : farbe("--mass");
    c.strokeStyle = linie;
    c.fillStyle = linie;
    c.lineWidth = 1;
    for (const m of g.masse) {
      c.beginPath();
      for (const [a, b] of m.striche) {
        const [ax, ay] = bild(a);
        const [bx, by] = bild(b);
        c.moveTo(ax, ay);
        c.lineTo(bx, by);
      }
      c.stroke();
      for (const [spitze, r] of m.pfeile) {
        const [x, y] = bild(spitze);
        const l = 7;
        c.beginPath();
        c.moveTo(x, y);
        c.lineTo(x - r[0] * l - r[1] * 2.5, y - r[1] * l + r[0] * 2.5);
        c.lineTo(x - r[0] * l + r[1] * 2.5, y - r[1] * l - r[0] * 2.5);
        c.closePath();
        c.fill();
      }
    }
    c.font = "12px system-ui, sans-serif";
    c.textAlign = "center";
    c.textBaseline = "middle";
    for (const m of g.masse) {
      const [x, y] = bild(m.textPos);
      c.save();
      c.translate(x, y);
      c.rotate(m.textWinkel);
      const b = c.measureText(m.text).width + 6;
      c.fillStyle = farbe("--flaeche");
      c.fillRect(-b / 2, -8, b, 16);
      c.fillStyle = linie;
      c.fillText(m.text, 0, 0);
      c.restore();
    }
    if (gewaehlt) {
      c.font = "bold 10px system-ui, sans-serif";
      for (const sy of g.symbole) {
        const [x, y] = bild(sy.pos);
        c.fillStyle = farbe("--symbol");
        c.fillRect(x - 7, y - 7, 14, 14);
        c.fillStyle = farbe("--symbol-text");
        c.fillText(sy.text, x, y + 0.5);
      }
    }
    c.restore();
  }

  /** Millimeterraster, das sich der Zoomstufe anpasst. */
  private raster(fein: string, stark: string): void {
    const c = this.ctx;
    const s = this.massstab;
    const schritte = [1, 5, 10, 50, 100, 500, 1000];
    const klein = schritte.find((m) => m * s >= 8) ?? 1000;
    const gross = schritte.find((m) => m >= klein * 5 && m * s >= 40) ?? klein * 10;
    const [x0, y0] = this.zuMuster(0, 0);
    const [x1, y1] = this.zuMuster(this.breite, this.hoehe);
    const linien = (abstand: number, farbe: string) => {
      c.beginPath();
      for (let x = Math.floor(x0 / abstand) * abstand; x <= x1; x += abstand) {
        const sx = Math.round(x * s + this.versatzX) + 0.5;
        c.moveTo(sx, 0);
        c.lineTo(sx, this.hoehe);
      }
      for (let y = Math.floor(y0 / abstand) * abstand; y <= y1; y += abstand) {
        const sy = Math.round(y * s + this.versatzY) + 0.5;
        c.moveTo(0, sy);
        c.lineTo(this.breite, sy);
      }
      c.strokeStyle = farbe;
      c.lineWidth = 1;
      c.stroke();
    };
    linien(klein, fein);
    linien(gross, stark);
  }
}
