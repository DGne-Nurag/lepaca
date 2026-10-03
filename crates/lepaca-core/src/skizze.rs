//! Parametrische Skizze: Maße und geometrische Bedingungen für einen frei
//! gezeichneten Pfad, wie bei den Skizzen in Fusion 360.
//!
//! Ein Pfad wird mit seinen Bedingungen an den Löser gegeben. Der Löser
//! verschiebt Punkte, Bogenmittelpunkte und Kurvengriffe so wenig wie
//! möglich, bis alle Bedingungen erfüllt sind. Danach meldet er, wie viele
//! Freiheitsgrade noch offen sind und welche Punkte sich noch bewegen
//! lassen, damit die Oberfläche „vollständig bestimmt" anzeigen kann.
//!
//! Zählweise: Punkt 0 ist der Start, Punkt `i + 1` das Ziel von Element
//! `i`. Kante `i` ist Element `i` (von Punkt `i` zu Punkt `i + 1`). Ein
//! geschlossener Pfad, der nicht am Start endet, hat als letzte Kante die
//! gerade Schlusslinie (siehe [`Pfad::kantenzahl`]). Endet er am Start,
//! ist der letzte Punkt derselbe wie Punkt 0.
//!
//! Der Startpunkt bleibt immer liegen: Er ist der Bezugspunkt des Teils.

use std::f64::consts::PI;

use crate::form::{FormFehler, Pfad, PfadElement, bogen_mitte, bogen_spanne};
use crate::geometrie::Punkt;
use crate::loeser::{self, System};
use crate::mass::Mm;

/// Höchstzahl Elemente in einer Skizze mit Bedingungen. Der Löser rechnet
/// mit dichten Matrizen; mehr wäre spürbar langsam (Playbook 01).
pub const MAX_ELEMENTE: usize = 200;
/// Höchstzahl Bedingungen je Skizze.
pub const MAX_BEDINGUNGEN: usize = 400;
/// Winkelbedingungen werden mit dieser Länge (mm) gewichtet, damit sie
/// neben Längen nicht untergehen.
const WINKEL_GEWICHT: f64 = 100.0;

/// Ein Maß oder eine geometrische Bedingung.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bedingung {
    /// Länge einer geraden Kante.
    Laenge {
        kante: usize,
        wert: Mm,
    },
    /// Direkter Abstand zweier Punkte.
    Abstand {
        a: usize,
        b: usize,
        wert: Mm,
    },
    /// Waagerechter Abstand zweier Punkte. Die Seite (links/rechts)
    /// bleibt, wie sie gezeichnet ist.
    AbstandX {
        a: usize,
        b: usize,
        wert: Mm,
    },
    /// Senkrechter Abstand zweier Punkte, Seite wie gezeichnet.
    AbstandY {
        a: usize,
        b: usize,
        wert: Mm,
    },
    /// Radius eines Bogens.
    Radius {
        kante: usize,
        wert: Mm,
    },
    /// Winkel zwischen zwei geraden Kanten in Grad (0 bis 180), gemessen
    /// zwischen ihren Laufrichtungen; die Drehrichtung bleibt wie gezeichnet.
    Winkel {
        a: usize,
        b: usize,
        grad: f64,
    },
    Horizontal {
        kante: usize,
    },
    Vertikal {
        kante: usize,
    },
    Parallel {
        a: usize,
        b: usize,
    },
    Senkrecht {
        a: usize,
        b: usize,
    },
    /// Zwei gerade Kanten gleich lang oder zwei Bögen mit gleichem Radius.
    Gleich {
        a: usize,
        b: usize,
    },
    /// Zwei Kanten, die sich einen Punkt teilen, gehen dort ohne Knick
    /// ineinander über.
    Tangential {
        a: usize,
        b: usize,
    },
    /// Ein Punkt liegt fest an einer Stelle (relativ zum Teil).
    Fest {
        punkt: usize,
        x: f64,
        y: f64,
    },
}

/// Warum eine Skizze nicht gelöst werden kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkizzenFehler {
    /// Mehr als [`MAX_ELEMENTE`] Elemente oder [`MAX_BEDINGUNGEN`]
    /// Bedingungen.
    ZuGross,
    /// Eine Bedingung nennt einen Punkt oder eine Kante, die es nicht gibt.
    Verweis,
    /// Die Bedingung passt nicht zur Kante, z. B. ein Radius an einer Linie.
    KanteUnpassend,
    /// Ein Maß ist negativ, null oder keine Zahl, ein Winkel nicht
    /// zwischen 0 und 180 Grad.
    UngueltigerWert,
    /// Die Bedingungen widersprechen sich; es gibt keine Lösung.
    Widerspruch,
    Form(FormFehler),
}

impl From<FormFehler> for SkizzenFehler {
    fn from(f: FormFehler) -> Self {
        SkizzenFehler::Form(f)
    }
}

/// Ergebnis des Lösers.
#[derive(Debug, Clone, PartialEq)]
pub struct Ergebnis {
    /// Der Pfad mit allen Bedingungen erfüllt.
    pub pfad: Pfad,
    /// Wie viele Freiheitsgrade noch offen sind; 0 heißt vollständig
    /// bestimmt.
    pub freiheitsgrade: usize,
    /// Mindestens eine Bedingung ist doppelt, d. h. schon durch andere
    /// festgelegt (überbestimmt, aber widerspruchsfrei).
    pub ueberbestimmt: bool,
    /// Je Punkt, ob er sich noch bewegen lässt.
    pub freie_punkte: Vec<bool>,
    /// Je Kante, ob sie sich noch verändern lässt (ein Endpunkt, der
    /// Bogenmittelpunkt oder ein Kurvengriff ist frei).
    pub freie_kanten: Vec<bool>,
}

/// Wie eine Kante gebaut ist; Indizes zeigen in den Variablenvektor.
#[derive(Debug, Clone, Copy)]
enum Art {
    Linie,
    Bogen { mitte: usize, uhrzeigersinn: bool },
    Kurve { k1: usize, k2: usize },
}

#[derive(Debug, Clone, Copy)]
struct Kante {
    von: usize,
    bis: usize,
    art: Art,
}

/// Die Skizze als Gleichungssystem.
#[derive(Debug)]
struct Modell {
    start: Punkt,
    /// Je Punkt der Index seiner x-Variablen (y folgt); `None` für den
    /// festen Start.
    punkt_var: Vec<Option<usize>>,
    kanten: Vec<Kante>,
    anzahl: usize,
}

impl Modell {
    fn neu(pfad: &Pfad, geschlossen: bool) -> Result<(Modell, Vec<f64>), SkizzenFehler> {
        let n = pfad.elemente.len();
        if n > MAX_ELEMENTE {
            return Err(SkizzenFehler::ZuGross);
        }
        let mut v: Vec<f64> = Vec::new();
        let mut punkt_var: Vec<Option<usize>> = vec![None];
        // Endet ein geschlossener Pfad am Start, ist sein letzter Punkt der
        // Start selbst.
        let am_start = geschlossen && n > 0 && !pfad.hat_schlusslinie();
        for (i, e) in pfad.elemente.iter().enumerate() {
            if am_start && i == n - 1 {
                punkt_var.push(None);
            } else {
                punkt_var.push(Some(v.len()));
                let z = e.ziel();
                v.extend([z.x, z.y]);
            }
        }
        let mut kanten = Vec::with_capacity(n + 1);
        let mut von_punkt = pfad.start;
        for (i, e) in pfad.elemente.iter().enumerate() {
            let art = match *e {
                PfadElement::Linie(_) => Art::Linie,
                PfadElement::Bogen {
                    ziel,
                    radius,
                    uhrzeigersinn,
                    gross,
                } => {
                    let m = bogen_mitte(von_punkt, ziel, radius.wert(), uhrzeigersinn, gross)?;
                    let mitte = v.len();
                    v.extend([m.x, m.y]);
                    Art::Bogen {
                        mitte,
                        uhrzeigersinn,
                    }
                }
                PfadElement::Kurve { k1, k2, .. } => {
                    let a = v.len();
                    v.extend([k1.x, k1.y, k2.x, k2.y]);
                    Art::Kurve { k1: a, k2: a + 2 }
                }
            };
            // Endet der Pfad am Start, läuft die letzte Kante zu Punkt 0.
            let bis = if am_start && i == n - 1 { 0 } else { i + 1 };
            kanten.push(Kante { von: i, bis, art });
            von_punkt = e.ziel();
        }
        if geschlossen && pfad.hat_schlusslinie() && n > 0 {
            kanten.push(Kante {
                von: n,
                bis: 0,
                art: Art::Linie,
            });
        }
        if v.iter().any(|x| !x.is_finite()) || !pfad.start.ist_endlich() {
            return Err(SkizzenFehler::UngueltigerWert);
        }
        let anzahl = v.len();
        Ok((
            Modell {
                start: pfad.start,
                punkt_var,
                kanten,
                anzahl,
            },
            v,
        ))
    }

    fn punkt(&self, v: &[f64], i: usize) -> Punkt {
        match self.punkt_var[i] {
            Some(k) => Punkt::neu(v[k], v[k + 1]),
            None => self.start,
        }
    }

    fn var(v: &[f64], k: usize) -> Punkt {
        Punkt::neu(v[k], v[k + 1])
    }

    /// Variablen, von denen eine Kante abhängt.
    fn kanten_vars(&self, k: usize, aus: &mut Vec<usize>) {
        let kante = self.kanten[k];
        for p in [kante.von, kante.bis] {
            if let Some(i) = self.punkt_var[p] {
                aus.extend([i, i + 1]);
            }
        }
        match kante.art {
            Art::Linie => {}
            Art::Bogen { mitte, .. } => aus.extend([mitte, mitte + 1]),
            Art::Kurve { k1, k2 } => aus.extend([k1, k1 + 1, k2, k2 + 1]),
        }
    }

    fn richtung(&self, v: &[f64], k: usize) -> Punkt {
        let kante = self.kanten[k];
        self.punkt(v, kante.bis).minus(self.punkt(v, kante.von))
    }

    fn radius(&self, v: &[f64], k: usize) -> f64 {
        match self.kanten[k].art {
            Art::Bogen { mitte, .. } => self
                .punkt(v, self.kanten[k].von)
                .abstand(Modell::var(v, mitte)),
            _ => 0.0,
        }
    }

    /// Laufrichtung der Kante an ihrem Anfang oder Ende.
    fn tangente(&self, v: &[f64], k: usize, am_ende: bool) -> Punkt {
        let kante = self.kanten[k];
        let von = self.punkt(v, kante.von);
        let bis = self.punkt(v, kante.bis);
        match kante.art {
            Art::Linie => bis.minus(von),
            Art::Bogen {
                mitte,
                uhrzeigersinn,
            } => {
                let p = if am_ende { bis } else { von };
                let r = p.minus(Modell::var(v, mitte));
                // Senkrecht zum Radius, in Laufrichtung.
                let t = Punkt::neu(-r.y, r.x);
                if uhrzeigersinn { t } else { t.mal(-1.0) }
            }
            Art::Kurve { k1, k2 } => {
                let (a, b) = (Modell::var(v, k1), Modell::var(v, k2));
                if am_ende {
                    if bis.abstand(b) > 1e-9 {
                        bis.minus(b)
                    } else {
                        bis.minus(a)
                    }
                } else if a.abstand(von) > 1e-9 {
                    a.minus(von)
                } else {
                    b.minus(von)
                }
            }
        }
    }

    fn ist_linie(&self, k: usize) -> bool {
        matches!(self.kanten[k].art, Art::Linie)
    }

    fn ist_bogen(&self, k: usize) -> bool {
        matches!(self.kanten[k].art, Art::Bogen { .. })
    }
}

/// Eine einzelne Gleichung `wert(v) = 0`.
#[derive(Debug, Clone, Copy)]
enum Gleichung {
    /// |b − a| = soll
    Abstand {
        a: usize,
        b: usize,
        soll: f64,
    },
    /// (b − a).x = soll bzw. .y
    DiffX {
        a: usize,
        b: usize,
        soll: f64,
    },
    DiffY {
        a: usize,
        b: usize,
        soll: f64,
    },
    Radius {
        kante: usize,
        soll: f64,
    },
    /// Beide Enden eines Bogens liegen gleich weit vom Mittelpunkt.
    BogenRund {
        kante: usize,
    },
    Horizontal {
        kante: usize,
    },
    Vertikal {
        kante: usize,
    },
    Parallel {
        a: usize,
        b: usize,
    },
    Senkrecht {
        a: usize,
        b: usize,
    },
    /// Vorzeichenbehafteter Winkel von a nach b (Bogenmaß).
    Winkel {
        a: usize,
        b: usize,
        soll: f64,
    },
    GleichLang {
        a: usize,
        b: usize,
    },
    GleichRadius {
        a: usize,
        b: usize,
    },
    /// Tangente von a an ihrem Ende parallel zur Tangente von b an ihrem
    /// Anfang (oder umgekehrt, je nach gemeinsamem Punkt).
    Tangential {
        a: usize,
        a_ende: bool,
        b: usize,
        b_ende: bool,
    },
    FestX {
        punkt: usize,
        x: f64,
    },
    FestY {
        punkt: usize,
        y: f64,
    },
}

fn normierte_kreuz(a: Punkt, b: Punkt) -> f64 {
    let l = a.laenge() * b.laenge();
    if l < 1e-12 { 0.0 } else { a.kreuz(b) / l }
}

fn winkel_zwischen(a: Punkt, b: Punkt) -> f64 {
    a.kreuz(b).atan2(a.skalar(b))
}

impl Gleichung {
    fn wert(&self, m: &Modell, v: &[f64]) -> f64 {
        match *self {
            Gleichung::Abstand { a, b, soll } => m.punkt(v, a).abstand(m.punkt(v, b)) - soll,
            Gleichung::DiffX { a, b, soll } => m.punkt(v, b).x - m.punkt(v, a).x - soll,
            Gleichung::DiffY { a, b, soll } => m.punkt(v, b).y - m.punkt(v, a).y - soll,
            Gleichung::Radius { kante, soll } => m.radius(v, kante) - soll,
            Gleichung::BogenRund { kante } => {
                let k = m.kanten[kante];
                let Art::Bogen { mitte, .. } = k.art else {
                    return 0.0;
                };
                let c = Modell::var(v, mitte);
                m.punkt(v, k.von).abstand(c) - m.punkt(v, k.bis).abstand(c)
            }
            Gleichung::Horizontal { kante } => m.richtung(v, kante).y,
            Gleichung::Vertikal { kante } => m.richtung(v, kante).x,
            Gleichung::Parallel { a, b } => {
                WINKEL_GEWICHT * normierte_kreuz(m.richtung(v, a), m.richtung(v, b))
            }
            Gleichung::Senkrecht { a, b } => {
                let (da, db) = (m.richtung(v, a), m.richtung(v, b));
                let l = da.laenge() * db.laenge();
                if l < 1e-12 {
                    0.0
                } else {
                    WINKEL_GEWICHT * da.skalar(db) / l
                }
            }
            Gleichung::Winkel { a, b, soll } => {
                let ist = winkel_zwischen(m.richtung(v, a), m.richtung(v, b));
                let d = (ist - soll + PI).rem_euclid(2.0 * PI) - PI;
                WINKEL_GEWICHT * d
            }
            Gleichung::GleichLang { a, b } => m.richtung(v, a).laenge() - m.richtung(v, b).laenge(),
            Gleichung::GleichRadius { a, b } => m.radius(v, a) - m.radius(v, b),
            Gleichung::Tangential {
                a,
                a_ende,
                b,
                b_ende,
            } => {
                WINKEL_GEWICHT * normierte_kreuz(m.tangente(v, a, a_ende), m.tangente(v, b, b_ende))
            }
            Gleichung::FestX { punkt, x } => m.punkt(v, punkt).x - x,
            Gleichung::FestY { punkt, y } => m.punkt(v, punkt).y - y,
        }
    }

    fn variablen(&self, m: &Modell) -> Vec<usize> {
        let mut aus = Vec::with_capacity(12);
        let punkt = |p: usize, aus: &mut Vec<usize>| {
            if let Some(i) = m.punkt_var[p] {
                aus.extend([i, i + 1]);
            }
        };
        match *self {
            Gleichung::Abstand { a, b, .. }
            | Gleichung::DiffX { a, b, .. }
            | Gleichung::DiffY { a, b, .. } => {
                punkt(a, &mut aus);
                punkt(b, &mut aus);
            }
            Gleichung::FestX { punkt: p, .. } | Gleichung::FestY { punkt: p, .. } => {
                punkt(p, &mut aus)
            }
            Gleichung::Radius { kante, .. }
            | Gleichung::BogenRund { kante }
            | Gleichung::Horizontal { kante }
            | Gleichung::Vertikal { kante } => m.kanten_vars(kante, &mut aus),
            Gleichung::Parallel { a, b }
            | Gleichung::Senkrecht { a, b }
            | Gleichung::Winkel { a, b, .. }
            | Gleichung::GleichLang { a, b }
            | Gleichung::GleichRadius { a, b }
            | Gleichung::Tangential { a, b, .. } => {
                m.kanten_vars(a, &mut aus);
                m.kanten_vars(b, &mut aus);
            }
        }
        aus.sort_unstable();
        aus.dedup();
        aus
    }
}

pub(crate) fn pruefe_wert(w: Mm) -> Result<f64, SkizzenFehler> {
    let x = w.wert();
    if x > 0.0 && x <= 10_000.0 {
        Ok(x)
    } else {
        Err(SkizzenFehler::UngueltigerWert)
    }
}

/// Vorzeichen, das die gezeichnete Seite festhält (0 zählt als +).
pub(crate) fn seite(x: f64) -> f64 {
    if x < 0.0 { -1.0 } else { 1.0 }
}

/// Übersetzt die Bedingungen in Gleichungen. Seiten und Drehrichtungen
/// werden aus der Zeichnung übernommen.
fn gleichungen(
    m: &Modell,
    v: &[f64],
    bedingungen: &[Bedingung],
) -> Result<Vec<Gleichung>, SkizzenFehler> {
    let np = m.punkt_var.len();
    let nk = m.kanten.len();
    let punkt = |p: usize| {
        if p < np {
            Ok(p)
        } else {
            Err(SkizzenFehler::Verweis)
        }
    };
    let kante = |k: usize| {
        if k < nk {
            Ok(k)
        } else {
            Err(SkizzenFehler::Verweis)
        }
    };
    let linie = |k: usize| {
        kante(k)?;
        if m.ist_linie(k) {
            Ok(k)
        } else {
            Err(SkizzenFehler::KanteUnpassend)
        }
    };
    let bogen = |k: usize| {
        kante(k)?;
        if m.ist_bogen(k) {
            Ok(k)
        } else {
            Err(SkizzenFehler::KanteUnpassend)
        }
    };

    let mut g: Vec<Gleichung> = Vec::new();
    // Feste Gleichungen: Bögen bleiben Kreisbögen.
    for k in 0..nk {
        if m.ist_bogen(k) {
            g.push(Gleichung::BogenRund { kante: k });
        }
    }
    for b in bedingungen {
        match *b {
            Bedingung::Laenge { kante: k, wert } => {
                let k = linie(k)?;
                let e = m.kanten[k];
                g.push(Gleichung::Abstand {
                    a: e.von,
                    b: e.bis,
                    soll: pruefe_wert(wert)?,
                });
            }
            Bedingung::Abstand { a, b, wert } => g.push(Gleichung::Abstand {
                a: punkt(a)?,
                b: punkt(b)?,
                soll: pruefe_wert(wert)?,
            }),
            Bedingung::AbstandX { a, b, wert } => {
                let (a, b) = (punkt(a)?, punkt(b)?);
                let s = seite(m.punkt(v, b).x - m.punkt(v, a).x);
                g.push(Gleichung::DiffX {
                    a,
                    b,
                    soll: s * pruefe_wert(wert)?,
                });
            }
            Bedingung::AbstandY { a, b, wert } => {
                let (a, b) = (punkt(a)?, punkt(b)?);
                let s = seite(m.punkt(v, b).y - m.punkt(v, a).y);
                g.push(Gleichung::DiffY {
                    a,
                    b,
                    soll: s * pruefe_wert(wert)?,
                });
            }
            Bedingung::Radius { kante: k, wert } => g.push(Gleichung::Radius {
                kante: bogen(k)?,
                soll: pruefe_wert(wert)?,
            }),
            Bedingung::Winkel { a, b, grad } => {
                let (a, b) = (linie(a)?, linie(b)?);
                if !(0.0..=180.0).contains(&grad) {
                    return Err(SkizzenFehler::UngueltigerWert);
                }
                let s = seite(winkel_zwischen(m.richtung(v, a), m.richtung(v, b)));
                g.push(Gleichung::Winkel {
                    a,
                    b,
                    soll: s * grad.to_radians(),
                });
            }
            Bedingung::Horizontal { kante: k } => {
                g.push(Gleichung::Horizontal { kante: linie(k)? })
            }
            Bedingung::Vertikal { kante: k } => g.push(Gleichung::Vertikal { kante: linie(k)? }),
            Bedingung::Parallel { a, b } => g.push(Gleichung::Parallel {
                a: linie(a)?,
                b: linie(b)?,
            }),
            Bedingung::Senkrecht { a, b } => g.push(Gleichung::Senkrecht {
                a: linie(a)?,
                b: linie(b)?,
            }),
            Bedingung::Gleich { a, b } => {
                let (a, b) = (kante(a)?, kante(b)?);
                if m.ist_linie(a) && m.ist_linie(b) {
                    g.push(Gleichung::GleichLang { a, b });
                } else if m.ist_bogen(a) && m.ist_bogen(b) {
                    g.push(Gleichung::GleichRadius { a, b });
                } else {
                    return Err(SkizzenFehler::KanteUnpassend);
                }
            }
            Bedingung::Tangential { a, b } => {
                let (a, b) = (kante(a)?, kante(b)?);
                let (ka, kb) = (m.kanten[a], m.kanten[b]);
                let gl = if ka.bis == kb.von {
                    Gleichung::Tangential {
                        a,
                        a_ende: true,
                        b,
                        b_ende: false,
                    }
                } else if kb.bis == ka.von {
                    Gleichung::Tangential {
                        a: b,
                        a_ende: true,
                        b: a,
                        b_ende: false,
                    }
                } else {
                    // Nur Kanten mit gemeinsamem Punkt können tangential sein.
                    return Err(SkizzenFehler::KanteUnpassend);
                };
                g.push(gl);
            }
            Bedingung::Fest { punkt: p, x, y } => {
                let p = punkt(p)?;
                if !(x.is_finite() && y.is_finite()) {
                    return Err(SkizzenFehler::UngueltigerWert);
                }
                g.push(Gleichung::FestX { punkt: p, x });
                g.push(Gleichung::FestY { punkt: p, y });
            }
        }
    }
    Ok(g)
}

/// Schreibt die gelösten Werte zurück in einen Pfad.
fn pfad_aus(m: &Modell, alt: &Pfad, v: &[f64]) -> Result<Pfad, SkizzenFehler> {
    let mut elemente = Vec::with_capacity(alt.elemente.len());
    for (i, e) in alt.elemente.iter().enumerate() {
        let ziel = m.punkt(v, i + 1);
        let neu = match (*e, m.kanten[i].art) {
            (PfadElement::Linie(_), _) => PfadElement::Linie(ziel),
            (
                PfadElement::Bogen { .. },
                Art::Bogen {
                    mitte,
                    uhrzeigersinn,
                },
            ) => {
                let von = m.punkt(v, i);
                let c = Modell::var(v, mitte);
                let r = (von.abstand(c) + ziel.abstand(c)) / 2.0;
                let gross = bogen_spanne(von, ziel, c, uhrzeigersinn) > PI;
                PfadElement::Bogen {
                    ziel,
                    radius: Mm::neu(r).map_err(|_| SkizzenFehler::Widerspruch)?,
                    uhrzeigersinn,
                    gross,
                }
            }
            (PfadElement::Kurve { .. }, Art::Kurve { k1, k2 }) => PfadElement::Kurve {
                k1: Modell::var(v, k1),
                k2: Modell::var(v, k2),
                ziel,
            },
            _ => return Err(SkizzenFehler::KanteUnpassend),
        };
        elemente.push(neu);
    }
    if elemente.iter().any(|e| !e.ziel().ist_endlich()) {
        return Err(SkizzenFehler::Widerspruch);
    }
    Ok(Pfad {
        start: alt.start,
        elemente,
    })
}

/// Löst die Skizze: verändert den Pfad so wenig wie möglich, bis alle
/// Bedingungen erfüllt sind.
pub fn loesen(
    pfad: &Pfad,
    geschlossen: bool,
    bedingungen: &[Bedingung],
) -> Result<Ergebnis, SkizzenFehler> {
    if bedingungen.len() > MAX_BEDINGUNGEN {
        return Err(SkizzenFehler::ZuGross);
    }
    let (m, mut v) = Modell::neu(pfad, geschlossen)?;
    let g = gleichungen(&m, &v, bedingungen)?;
    let system = System {
        anzahl: m.anzahl,
        variablen: g.iter().map(|gl| gl.variablen(&m)).collect(),
        wert: Box::new(|i, v| g[i].wert(&m, v)),
    };
    if !loeser::loese(&system, &mut v) {
        return Err(SkizzenFehler::Widerspruch);
    }
    let neu = pfad_aus(&m, pfad, &v)?;
    // Die Form muss sich noch bauen lassen (z. B. kein unmöglicher Bogen).
    if geschlossen {
        neu.kontur_mit_kanten()?;
    } else if !neu.elemente.is_empty() {
        neu.linienzug()?;
    }

    let j = system.jacobi(&mut v);
    let (rang, kern) = loeser::rang_und_kern(&j, m.anzahl);
    let frei = |k: usize| kern.iter().any(|x| x[k].abs() > 1e-6);
    let freie_punkte: Vec<bool> = m
        .punkt_var
        .iter()
        .map(|p| p.is_some_and(|i| frei(i) || frei(i + 1)))
        .collect();
    let freie_kanten = (0..m.kanten.len())
        .map(|k| {
            let mut vs = Vec::new();
            m.kanten_vars(k, &mut vs);
            vs.into_iter().any(frei)
        })
        .collect();
    Ok(Ergebnis {
        pfad: neu,
        freiheitsgrade: m.anzahl - rang,
        ueberbestimmt: rang < g.len(),
        freie_punkte,
        freie_kanten,
    })
}

/// Ein Maß zum Zeichnen: Maß- und Hilfslinien, Pfeile und Beschriftung,
/// in Koordinaten des Teils (mm).
#[derive(Debug, Clone, PartialEq)]
pub struct Masslinie {
    /// Index der Bedingung, zu der das Maß gehört; `None` bei Maßen einer
    /// Grundform (Rechteck, T-Pocket …).
    pub bedingung: Option<usize>,
    pub striche: Vec<(Punkt, Punkt)>,
    /// Pfeile: Spitze und Richtung, in die sie zeigt (Einheitsvektor).
    pub pfeile: Vec<(Punkt, Punkt)>,
    pub text: String,
    /// Mitte der Beschriftung.
    pub text_pos: Punkt,
    /// Drehung der Beschriftung (Bogenmaß, im Uhrzeigersinn auf dem
    /// Bildschirm); immer so, dass sie lesbar bleibt.
    pub text_winkel: f64,
}

/// Ein Symbol für eine geometrische Bedingung (waagerecht, parallel …).
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub bedingung: usize,
    pub pos: Punkt,
    pub text: &'static str,
}

/// Abstand der Maßlinie von der Kante (mm).
pub(crate) const MASS_ABSTAND: f64 = 8.0;
/// Weitere Maße in dieselbe Richtung stapeln sich in diesem Abstand.
const MASS_STAFFEL: f64 = 7.0;

/// Zahl wie auf einer Zeichnung: höchstens zwei Nachkommastellen, Komma.
pub fn masszahl(x: f64) -> String {
    let s = format!("{x:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.replace('.', ",")
}

/// Beschriftung lesbar halten: nie auf dem Kopf.
fn lesbar(w: f64) -> f64 {
    let mut w = w.rem_euclid(2.0 * PI);
    // Senkrecht steht die Schrift von unten nach oben zu lesen.
    if (PI / 2.0 - 1e-9..3.0 * PI / 2.0 - 1e-9).contains(&w) {
        w -= PI;
    }
    if w > PI {
        w -= 2.0 * PI;
    }
    w
}

/// Maß zwischen zwei Punkten entlang `richtung`, um `abstand` in Richtung
/// `normale` versetzt.
pub(crate) fn mass_zwischen(
    bedingung: Option<usize>,
    a: Punkt,
    b: Punkt,
    normale: Punkt,
    linie_a: Punkt,
    linie_b: Punkt,
    text: String,
) -> Masslinie {
    let ueber = normale.mal(2.0);
    let r = linie_b
        .minus(linie_a)
        .normiert()
        .unwrap_or(Punkt::neu(1.0, 0.0));
    let mitte = linie_a.plus(linie_b).mal(0.5);
    Masslinie {
        bedingung,
        striche: vec![
            (a, linie_a.plus(ueber)),
            (b, linie_b.plus(ueber)),
            (linie_a, linie_b),
        ],
        pfeile: vec![(linie_a, r.mal(-1.0)), (linie_b, r)],
        text,
        text_pos: mitte.plus(normale.mal(2.8)),
        text_winkel: lesbar(r.y.atan2(r.x)),
    }
}

/// Maßlinien und Symbole einer Skizze. Ungültige Bedingungen werden
/// übersprungen; die Oberfläche meldet sie über den Löser.
pub fn zeichnung(
    pfad: &Pfad,
    geschlossen: bool,
    bedingungen: &[Bedingung],
) -> (Vec<Masslinie>, Vec<Symbol>) {
    let Ok((m, v)) = Modell::neu(pfad, geschlossen) else {
        return (Vec::new(), Vec::new());
    };
    let np = m.punkt_var.len();
    let nk = m.kanten.len();
    let punkte: Vec<Punkt> = (0..np).map(|i| m.punkt(&v, i)).collect();
    // Grenzen und Schwerpunkt über die abgeflachte Form.
    let alle: Vec<Punkt> = if geschlossen {
        pfad.kontur_mit_kanten()
            .map(|(k, _)| k.punkte().to_vec())
            .unwrap_or_else(|_| punkte.clone())
    } else {
        pfad.linienzug()
            .map(|l| l.punkte)
            .unwrap_or_else(|_| punkte.clone())
    };
    let mut min = Punkt::neu(f64::INFINITY, f64::INFINITY);
    let mut max = Punkt::neu(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for q in &alle {
        min = Punkt::neu(min.x.min(q.x), min.y.min(q.y));
        max = Punkt::neu(max.x.max(q.x), max.y.max(q.y));
    }
    let mitte = min.plus(max).mal(0.5);
    let aussen = |q: Punkt, n: Punkt| {
        if q.minus(mitte).skalar(n) < 0.0 {
            n.mal(-1.0)
        } else {
            n
        }
    };
    let kante_mitte = |k: usize| -> (Punkt, Punkt) {
        let e = m.kanten[k];
        let (a, b) = (punkte[e.von], punkte[e.bis]);
        match e.art {
            Art::Bogen {
                mitte: c,
                uhrzeigersinn,
            } => {
                let c = Modell::var(&v, c);
                let r = a.abstand(c);
                let w0 = (a.y - c.y).atan2(a.x - c.x);
                let s = bogen_spanne(a, b, c, uhrzeigersinn);
                let w = w0 + if uhrzeigersinn { s / 2.0 } else { -s / 2.0 };
                let q = c.plus(Punkt::neu(w.cos(), w.sin()).mal(r));
                (q, q.minus(c).normiert().unwrap_or(Punkt::neu(0.0, -1.0)))
            }
            _ => {
                let d = b.minus(a);
                let n = Punkt::neu(-d.y, d.x)
                    .normiert()
                    .unwrap_or(Punkt::neu(0.0, -1.0));
                let q = a.plus(b).mal(0.5);
                (q, aussen(q, n))
            }
        }
    };

    let mut masse = Vec::new();
    let mut symbole = Vec::new();
    let (mut oben, mut links, mut unten, mut rechts) = (0usize, 0usize, 0usize, 0usize);
    for (i, bed) in bedingungen.iter().enumerate() {
        match *bed {
            Bedingung::Laenge { kante, wert } if kante < nk => {
                let e = m.kanten[kante];
                let (a, b) = (punkte[e.von], punkte[e.bis]);
                let (_, n) = kante_mitte(kante);
                let la = a.plus(n.mal(MASS_ABSTAND));
                let lb = b.plus(n.mal(MASS_ABSTAND));
                masse.push(mass_zwischen(
                    Some(i),
                    a,
                    b,
                    n,
                    la,
                    lb,
                    masszahl(wert.wert()),
                ));
            }
            Bedingung::Abstand { a, b, wert } if a < np && b < np => {
                let (pa, pb) = (punkte[a], punkte[b]);
                let d = pb.minus(pa);
                let n = Punkt::neu(-d.y, d.x)
                    .normiert()
                    .unwrap_or(Punkt::neu(0.0, -1.0));
                let n = aussen(pa.plus(pb).mal(0.5), n);
                let la = pa.plus(n.mal(MASS_ABSTAND));
                let lb = pb.plus(n.mal(MASS_ABSTAND));
                masse.push(mass_zwischen(
                    Some(i),
                    pa,
                    pb,
                    n,
                    la,
                    lb,
                    masszahl(wert.wert()),
                ));
            }
            Bedingung::AbstandX { a, b, wert } if a < np && b < np => {
                let (pa, pb) = (punkte[a], punkte[b]);
                // Über oder unter die Form, je nachdem, wo die Punkte liegen.
                let nach_oben = pa.y + pb.y <= 2.0 * mitte.y;
                let (y, n) = if nach_oben {
                    oben += 1;
                    (
                        min.y - MASS_ABSTAND - MASS_STAFFEL * (oben - 1) as f64,
                        Punkt::neu(0.0, -1.0),
                    )
                } else {
                    unten += 1;
                    (
                        max.y + MASS_ABSTAND + MASS_STAFFEL * (unten - 1) as f64,
                        Punkt::neu(0.0, 1.0),
                    )
                };
                let la = Punkt::neu(pa.x, y);
                let lb = Punkt::neu(pb.x, y);
                masse.push(mass_zwischen(
                    Some(i),
                    pa,
                    pb,
                    n,
                    la,
                    lb,
                    masszahl(wert.wert()),
                ));
            }
            Bedingung::AbstandY { a, b, wert } if a < np && b < np => {
                let (pa, pb) = (punkte[a], punkte[b]);
                let nach_links = pa.x + pb.x <= 2.0 * mitte.x;
                let (x, n) = if nach_links {
                    links += 1;
                    (
                        min.x - MASS_ABSTAND - MASS_STAFFEL * (links - 1) as f64,
                        Punkt::neu(-1.0, 0.0),
                    )
                } else {
                    rechts += 1;
                    (
                        max.x + MASS_ABSTAND + MASS_STAFFEL * (rechts - 1) as f64,
                        Punkt::neu(1.0, 0.0),
                    )
                };
                let la = Punkt::neu(x, pa.y);
                let lb = Punkt::neu(x, pb.y);
                masse.push(mass_zwischen(
                    Some(i),
                    pa,
                    pb,
                    n,
                    la,
                    lb,
                    masszahl(wert.wert()),
                ));
            }
            Bedingung::Radius { kante, wert } if kante < nk && m.ist_bogen(kante) => {
                let (q, n) = kante_mitte(kante);
                let Art::Bogen { mitte: c, .. } = m.kanten[kante].art else {
                    continue;
                };
                let c = Modell::var(&v, c);
                let ende = q.plus(n.mal(MASS_ABSTAND));
                masse.push(Masslinie {
                    bedingung: Some(i),
                    striche: vec![(c, ende)],
                    pfeile: vec![(q, n)],
                    text: format!("R {}", masszahl(wert.wert())),
                    text_pos: ende.plus(n.mal(3.0)),
                    text_winkel: 0.0,
                });
            }
            Bedingung::Winkel { a, b, grad } if a < nk && b < nk => {
                let (ka, kb) = (m.kanten[a], m.kanten[b]);
                // Am gemeinsamen Punkt, sonst in der Mitte der ersten Kante.
                let ecke = [ka.von, ka.bis]
                    .into_iter()
                    .find(|p| *p == kb.von || *p == kb.bis)
                    .map(|p| punkte[p])
                    .unwrap_or_else(|| kante_mitte(a).0);
                let r = 10.0;
                let ra = m.richtung(&v, a).normiert().unwrap_or(Punkt::neu(1.0, 0.0));
                let rb = m.richtung(&v, b).normiert().unwrap_or(Punkt::neu(0.0, 1.0));
                // Schenkel zeigen von der Ecke weg in die Form hinein.
                let weg = |d: Punkt, k: Kante| {
                    if punkte[k.bis].abstand(ecke) < 1e-9 {
                        d.mal(-1.0)
                    } else {
                        d
                    }
                };
                let (sa, sb) = (weg(ra, ka), weg(rb, kb));
                let w0 = sa.y.atan2(sa.x);
                let mut dw = winkel_zwischen(sa, sb);
                if dw.abs() < 1e-9 {
                    dw = 0.0;
                }
                let n = 12;
                let bogen: Vec<Punkt> = (0..=n)
                    .map(|j| {
                        let w = w0 + dw * j as f64 / n as f64;
                        ecke.plus(Punkt::neu(w.cos(), w.sin()).mal(r))
                    })
                    .collect();
                let wm = w0 + dw / 2.0;
                masse.push(Masslinie {
                    bedingung: Some(i),
                    striche: bogen.windows(2).map(|w| (w[0], w[1])).collect(),
                    pfeile: Vec::new(),
                    text: format!("{}°", masszahl(grad)),
                    text_pos: ecke.plus(Punkt::neu(wm.cos(), wm.sin()).mal(r + 4.0)),
                    text_winkel: 0.0,
                });
            }
            Bedingung::Horizontal { kante } if kante < nk => {
                symbole.push(symbol(i, &kante_mitte, kante, "H"));
            }
            Bedingung::Vertikal { kante } if kante < nk => {
                symbole.push(symbol(i, &kante_mitte, kante, "V"));
            }
            Bedingung::Parallel { a, b } if a < nk && b < nk => {
                symbole.push(symbol(i, &kante_mitte, a, "∥"));
                symbole.push(symbol(i, &kante_mitte, b, "∥"));
            }
            Bedingung::Senkrecht { a, b } if a < nk && b < nk => {
                symbole.push(symbol(i, &kante_mitte, a, "⊥"));
                symbole.push(symbol(i, &kante_mitte, b, "⊥"));
            }
            Bedingung::Gleich { a, b } if a < nk && b < nk => {
                symbole.push(symbol(i, &kante_mitte, a, "="));
                symbole.push(symbol(i, &kante_mitte, b, "="));
            }
            Bedingung::Tangential { a, b } if a < nk && b < nk => {
                let (ka, kb) = (m.kanten[a], m.kanten[b]);
                let p = if ka.bis == kb.von { ka.bis } else { kb.bis };
                let (_, n) = kante_mitte(a);
                symbole.push(Symbol {
                    bedingung: i,
                    pos: punkte[p].minus(n.mal(3.5)),
                    text: "T",
                });
            }
            Bedingung::Fest { punkt, .. } if punkt < np => {
                symbole.push(Symbol {
                    bedingung: i,
                    pos: punkte[punkt].plus(Punkt::neu(3.0, -3.0)),
                    text: "F",
                });
            }
            _ => {}
        }
    }
    (masse, symbole)
}

/// Maße einer Grundform: Breite oben, Höhe links, dazu Radius bzw. bei der
/// T-Pocket Flügelhöhe rechts und Stegbreite unten. Koordinaten wie die
/// Kontur der Form (linke obere Ecke bei 0, 0).
pub fn form_masse(f: &crate::form::Form) -> Vec<Masslinie> {
    use crate::form::Form;
    let breite_hoehe = |b: f64, h: f64| {
        vec![
            mass_zwischen(
                None,
                Punkt::neu(0.0, 0.0),
                Punkt::neu(b, 0.0),
                Punkt::neu(0.0, -1.0),
                Punkt::neu(0.0, -MASS_ABSTAND),
                Punkt::neu(b, -MASS_ABSTAND),
                masszahl(b),
            ),
            mass_zwischen(
                None,
                Punkt::neu(0.0, h),
                Punkt::neu(0.0, 0.0),
                Punkt::neu(-1.0, 0.0),
                Punkt::neu(-MASS_ABSTAND, h),
                Punkt::neu(-MASS_ABSTAND, 0.0),
                masszahl(h),
            ),
        ]
    };
    match *f {
        Form::Rechteck {
            breite,
            hoehe,
            eckenradius,
        } => {
            let (b, h, r) = (breite.wert(), hoehe.wert(), eckenradius.wert());
            let mut m = breite_hoehe(b, h);
            if r > 0.0 {
                let w = -PI / 4.0;
                let c = Punkt::neu(b - r, r);
                let q = c.plus(Punkt::neu(w.cos(), w.sin()).mal(r));
                let n = Punkt::neu(w.cos(), w.sin());
                let ende = q.plus(n.mal(MASS_ABSTAND));
                m.push(Masslinie {
                    bedingung: None,
                    striche: vec![(c, ende)],
                    pfeile: vec![(q, n)],
                    text: format!("R {}", masszahl(r)),
                    text_pos: ende.plus(n.mal(3.0)),
                    text_winkel: 0.0,
                });
            }
            m
        }
        Form::Ellipse { breite, hoehe } => breite_hoehe(breite.wert(), hoehe.wert()),
        Form::Kreis { durchmesser } => {
            let d = durchmesser.wert();
            let mut m = breite_hoehe(d, d);
            m.truncate(1);
            m[0].text = format!("Ø {}", masszahl(d));
            m
        }
        Form::TPocket {
            breite,
            hoehe,
            fluegel_ueberstand,
            fluegel_hoehe,
        } => {
            let (b, h, u, fh) = (
                breite.wert(),
                hoehe.wert(),
                fluegel_ueberstand.wert(),
                fluegel_hoehe.wert(),
            );
            let mut m = breite_hoehe(b, h);
            let x = b + MASS_ABSTAND;
            m.push(mass_zwischen(
                None,
                Punkt::neu(b, fh),
                Punkt::neu(b, 0.0),
                Punkt::neu(1.0, 0.0),
                Punkt::neu(x, fh),
                Punkt::neu(x, 0.0),
                masszahl(fh),
            ));
            let y = h + MASS_ABSTAND;
            m.push(mass_zwischen(
                None,
                Punkt::neu(u, h),
                Punkt::neu(b - u, h),
                Punkt::neu(0.0, 1.0),
                Punkt::neu(u, y),
                Punkt::neu(b - u, y),
                masszahl(b - 2.0 * u),
            ));
            m
        }
        Form::Pfad(_) => Vec::new(),
    }
}

/// Symbol in der Kantenmitte, leicht nach innen versetzt.
fn symbol(
    bedingung: usize,
    kante_mitte: &impl Fn(usize) -> (Punkt, Punkt),
    kante: usize,
    text: &'static str,
) -> Symbol {
    let (q, n) = kante_mitte(kante);
    Symbol {
        bedingung,
        pos: q.minus(n.mal(3.5)),
        text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass::TOLERANZ_MM;

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    fn p(x: f64, y: f64) -> Punkt {
        Punkt::neu(x, y)
    }

    /// Ein schief gezeichnetes Viereck, ungefähr 80 × 50.
    fn krummes_viereck() -> Pfad {
        Pfad {
            start: p(0.0, 0.0),
            elemente: vec![
                PfadElement::Linie(p(78.3, 1.7)),
                PfadElement::Linie(p(81.2, 48.9)),
                PfadElement::Linie(p(-1.4, 51.6)),
                PfadElement::Linie(p(0.0, 0.0)),
            ],
        }
    }

    fn rechteckbedingungen(b: f64, h: f64) -> Vec<Bedingung> {
        vec![
            Bedingung::Horizontal { kante: 0 },
            Bedingung::Vertikal { kante: 1 },
            Bedingung::Horizontal { kante: 2 },
            Bedingung::Vertikal { kante: 3 },
            Bedingung::Laenge {
                kante: 0,
                wert: mm(b),
            },
            Bedingung::Laenge {
                kante: 1,
                wert: mm(h),
            },
        ]
    }

    #[test]
    fn ein_frei_gezeichnetes_viereck_wird_mit_massen_ein_genaues_rechteck() {
        let e = loesen(&krummes_viereck(), true, &rechteckbedingungen(80.0, 50.0)).unwrap();
        let ecken: Vec<Punkt> = e.pfad.elemente.iter().map(|x| x.ziel()).collect();
        for (ist, soll) in ecken
            .iter()
            .zip([p(80.0, 0.0), p(80.0, 50.0), p(0.0, 50.0)])
        {
            assert!(ist.abstand(soll) < TOLERANZ_MM, "{ist:?} statt {soll:?}");
        }
        assert_eq!(e.freiheitsgrade, 0, "vollständig bestimmt");
        assert!(!e.ueberbestimmt);
        assert!(e.freie_punkte.iter().all(|f| !f));
    }

    #[test]
    fn ohne_hoehe_bleibt_genau_ein_freiheitsgrad_und_die_unteren_punkte_sind_frei() {
        let mut b = rechteckbedingungen(80.0, 50.0);
        b.pop();
        let e = loesen(&krummes_viereck(), true, &b).unwrap();
        assert_eq!(e.freiheitsgrade, 1);
        // Punkt 0 (Start) und 1 liegen fest, 2 und 3 können hoch und runter.
        assert_eq!(e.freie_punkte, vec![false, false, true, true, false]);
    }

    #[test]
    fn ein_mass_aendern_verschiebt_nur_was_sich_bewegen_muss() {
        let e = loesen(&krummes_viereck(), true, &rechteckbedingungen(80.0, 50.0)).unwrap();
        let breiter = loesen(&e.pfad, true, &rechteckbedingungen(100.0, 50.0)).unwrap();
        let z: Vec<Punkt> = breiter.pfad.elemente.iter().map(|x| x.ziel()).collect();
        assert!(z[0].abstand(p(100.0, 0.0)) < TOLERANZ_MM);
        assert!(z[1].abstand(p(100.0, 50.0)) < TOLERANZ_MM);
        // Die linke Seite bleibt, wo sie war.
        assert!(z[2].abstand(p(0.0, 50.0)) < TOLERANZ_MM);
    }

    #[test]
    fn widerspruechliche_masse_werden_abgewiesen() {
        let mut b = rechteckbedingungen(80.0, 50.0);
        // Diagonale kürzer als die Breite: unmöglich.
        b.push(Bedingung::Abstand {
            a: 0,
            b: 2,
            wert: mm(60.0),
        });
        assert_eq!(
            loesen(&krummes_viereck(), true, &b),
            Err(SkizzenFehler::Widerspruch)
        );
    }

    #[test]
    fn ein_doppeltes_mass_wird_als_ueberbestimmt_erkannt() {
        let mut b = rechteckbedingungen(80.0, 50.0);
        b.push(Bedingung::Laenge {
            kante: 2,
            wert: mm(80.0),
        });
        let e = loesen(&krummes_viereck(), true, &b).unwrap();
        assert!(e.ueberbestimmt);
        assert_eq!(e.freiheitsgrade, 0);
    }

    #[test]
    fn eine_klappe_mit_tangentialem_bogen_bekommt_genau_den_radius() {
        // Taschenklappe: oben gerade, Seiten senkrecht, unten ein Bogen,
        // der links und rechts tangential anschließt (Halbkreis).
        let pfad = Pfad {
            start: p(0.0, 0.0),
            elemente: vec![
                PfadElement::Linie(p(61.0, 0.5)),
                PfadElement::Linie(p(60.0, 30.0)),
                PfadElement::Bogen {
                    ziel: p(1.0, 29.0),
                    radius: mm(31.0),
                    uhrzeigersinn: true,
                    gross: false,
                },
                PfadElement::Linie(p(0.0, 0.0)),
            ],
        };
        let b = vec![
            Bedingung::Horizontal { kante: 0 },
            Bedingung::Vertikal { kante: 1 },
            Bedingung::Vertikal { kante: 3 },
            Bedingung::Tangential { a: 1, b: 2 },
            Bedingung::Tangential { a: 2, b: 3 },
            Bedingung::Laenge {
                kante: 0,
                wert: mm(60.0),
            },
            Bedingung::Laenge {
                kante: 3,
                wert: mm(25.0),
            },
        ];
        let e = loesen(&pfad, true, &b).unwrap();
        let PfadElement::Bogen { radius, ziel, .. } = e.pfad.elemente[2] else {
            panic!("Bogen erwartet");
        };
        assert!(radius.etwa_gleich(mm(30.0)), "{radius}");
        assert!(ziel.abstand(p(0.0, 25.0)) < TOLERANZ_MM);
        assert_eq!(e.freiheitsgrade, 0);
        // Tiefster Punkt der Klappe: 25 + 30 mm.
        let (_, bis) = e.pfad.kontur_mit_kanten().unwrap().0.grenzen();
        assert!((bis.y - 55.0).abs() < 0.02, "{}", bis.y);
    }

    #[test]
    fn ein_winkel_von_60_grad_wird_eingehalten() {
        // Dreieck: Grundlinie 50 mm, linke Seite 50 mm unter 60°: gleichseitig.
        let pfad = Pfad {
            start: p(0.0, 0.0),
            elemente: vec![
                PfadElement::Linie(p(52.0, 1.0)),
                PfadElement::Linie(p(24.0, -40.0)),
            ],
        };
        let b = vec![
            Bedingung::Horizontal { kante: 0 },
            Bedingung::Laenge {
                kante: 0,
                wert: mm(50.0),
            },
            Bedingung::Laenge {
                kante: 2,
                wert: mm(50.0),
            },
            Bedingung::Winkel {
                a: 2,
                b: 0,
                grad: 120.0,
            },
        ];
        let e = loesen(&pfad, true, &b).unwrap();
        let spitze = e.pfad.elemente[1].ziel();
        let h = 50.0 * 3f64.sqrt() / 2.0;
        assert!(spitze.abstand(p(25.0, -h)) < TOLERANZ_MM, "{spitze:?}");
        assert_eq!(e.freiheitsgrade, 0);
    }

    #[test]
    fn verweise_auf_fehlende_kanten_und_falsche_kantenarten_werden_abgewiesen() {
        let v = krummes_viereck();
        assert_eq!(
            loesen(&v, true, &[Bedingung::Horizontal { kante: 9 }]),
            Err(SkizzenFehler::Verweis)
        );
        assert_eq!(
            loesen(
                &v,
                true,
                &[Bedingung::Radius {
                    kante: 0,
                    wert: mm(5.0)
                }]
            ),
            Err(SkizzenFehler::KanteUnpassend)
        );
        assert_eq!(
            loesen(
                &v,
                true,
                &[Bedingung::Laenge {
                    kante: 0,
                    wert: mm(-5.0)
                }]
            ),
            Err(SkizzenFehler::UngueltigerWert)
        );
    }

    #[test]
    fn eine_offene_linie_laesst_sich_bemassen() {
        let pfad = Pfad {
            start: p(0.0, 0.0),
            elemente: vec![PfadElement::Linie(p(3.0, 70.0))],
        };
        let b = [
            Bedingung::Vertikal { kante: 0 },
            Bedingung::Laenge {
                kante: 0,
                wert: mm(65.45),
            },
        ];
        let e = loesen(&pfad, false, &b).unwrap();
        assert!(e.pfad.elemente[0].ziel().abstand(p(0.0, 65.45)) < TOLERANZ_MM);
        assert_eq!(e.freiheitsgrade, 0);
    }

    #[test]
    fn ein_geschlossener_pfad_ohne_rueckweg_hat_die_schlusslinie_als_kante() {
        let pfad = Pfad {
            start: p(0.0, 0.0),
            elemente: vec![
                PfadElement::Linie(p(40.0, 2.0)),
                PfadElement::Linie(p(41.0, 30.0)),
                PfadElement::Linie(p(1.0, 29.0)),
            ],
        };
        // Kante 3 ist die Schlusslinie von Punkt 3 zurück zum Start.
        let b = [
            Bedingung::Horizontal { kante: 0 },
            Bedingung::Vertikal { kante: 3 },
            Bedingung::Parallel { a: 0, b: 2 },
            Bedingung::Senkrecht { a: 0, b: 1 },
        ];
        let e = loesen(&pfad, true, &b).unwrap();
        let z: Vec<Punkt> = e.pfad.elemente.iter().map(|x| x.ziel()).collect();
        assert!(z[2].x.abs() < TOLERANZ_MM);
        assert!((z[1].x - z[0].x).abs() < TOLERANZ_MM);
        assert!((z[1].y - z[2].y).abs() < TOLERANZ_MM);
        // Breite und Höhe sind noch offen.
        assert_eq!(e.freiheitsgrade, 2);
    }

    #[test]
    fn masszahlen_stehen_mit_komma_und_ohne_unnoetige_nullen() {
        assert_eq!(masszahl(217.0), "217");
        assert_eq!(masszahl(3.85), "3,85");
        assert_eq!(masszahl(45.5), "45,5");
    }

    #[test]
    fn die_breite_steht_ueber_dem_teil_und_die_hoehe_links_davon() {
        let pfad = krummes_viereck();
        let b = [
            Bedingung::AbstandX {
                a: 0,
                b: 1,
                wert: mm(78.3),
            },
            Bedingung::AbstandY {
                a: 0,
                b: 3,
                wert: mm(51.6),
            },
            Bedingung::Horizontal { kante: 0 },
        ];
        let (masse, symbole) = zeichnung(&pfad, true, &b);
        assert_eq!(masse.len(), 2);
        assert!(masse[0].text_pos.y < 0.0, "Breite oberhalb");
        assert_eq!(masse[0].text, "78,3");
        assert!(masse[1].text_pos.x < 0.0, "Höhe links");
        // Die Höhe steht senkrecht, aber lesbar (nicht auf dem Kopf).
        assert!(masse[1].text_winkel.abs() <= PI / 2.0 + 1e-9);
        assert_eq!(symbole.len(), 1);
        assert_eq!(symbole[0].text, "H");
    }
}
