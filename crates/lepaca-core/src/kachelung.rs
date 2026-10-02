//! Große Teile auf mehrere Blätter verteilen (Kachelung mit Klebekanten).
//!
//! Jedes Blatt hat einen Druckrand, den viele Drucker nicht bedrucken können.
//! Benachbarte Blätter überlappen um die Klebekante: Auf dem linken bzw.
//! oberen Blatt ist dieser Streifen markiert, das rechte bzw. untere Blatt
//! wird an seiner Rahmenlinie abgeschnitten und darauf geklebt.

use crate::geometrie::Punkt;
use crate::mass::Mm;

/// Höchstzahl Blätter, gegen versehentliche Riesenausdrucke.
pub const MAX_BLAETTER: usize = 500;
/// Erlaubter Bereich für den Druckrand.
pub const RAND_MM: std::ops::RangeInclusive<f64> = 0.0..=30.0;
/// Erlaubter Bereich für die Klebekante.
pub const KLEBEKANTE_MM: std::ops::RangeInclusive<f64> = 0.0..=50.0;
/// Mindestens so viel muss pro Blatt neu hinzukommen.
pub const MIN_NUTZFLAECHE_MM: f64 = 20.0;

/// Papierformat im Hochformat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Papier {
    pub breite: f64,
    pub hoehe: f64,
}

impl Papier {
    pub const A4: Papier = Papier {
        breite: 210.0,
        hoehe: 297.0,
    };
    pub const A3: Papier = Papier {
        breite: 297.0,
        hoehe: 420.0,
    };
    pub const LETTER: Papier = Papier {
        breite: 215.9,
        hoehe: 279.4,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Einstellung {
    pub papier: Papier,
    /// Unbedruckter Rand auf jeder Seite des Blatts.
    pub rand: Mm,
    /// Überlappung benachbarter Blätter zum Kleben.
    pub klebekante: Mm,
}

impl Default for Einstellung {
    fn default() -> Self {
        Einstellung {
            papier: Papier::A4,
            rand: Mm::neu(10.0).unwrap_or_else(|_| unreachable!()),
            klebekante: Mm::neu(10.0).unwrap_or_else(|_| unreachable!()),
        }
    }
}

/// Ein Blatt des Ausdrucks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kachel {
    /// Zeile (0 = oben) und Spalte (0 = links).
    pub zeile: usize,
    pub spalte: usize,
    /// Ausschnitt des Musters, der auf diesem Blatt gedruckt wird
    /// (Musterkoordinaten in mm).
    pub von: Punkt,
    pub bis: Punkt,
    /// Rechts bzw. unten folgt ein weiteres Blatt, das hier aufgeklebt wird.
    pub klebekante_rechts: bool,
    pub klebekante_unten: bool,
}

impl Kachel {
    /// Bezeichnung wie auf einem Stadtplan: Zeile als Buchstabe, Spalte als
    /// Zahl, z. B. „B3".
    pub fn name(&self) -> String {
        name(self.zeile, self.spalte)
    }
}

/// Bezeichnung für Zeile und Spalte (A1, B3, …; nach Z folgt AA).
pub fn name(zeile: usize, spalte: usize) -> String {
    let mut buchstaben = String::new();
    let mut z = zeile + 1;
    while z > 0 {
        let r = (z - 1) % 26;
        buchstaben.insert(0, char::from(b'A' + r as u8));
        z = (z - 1) / 26;
    }
    format!("{buchstaben}{}", spalte + 1)
}

/// Der fertige Plan: alle Blätter und das gewählte Seitenformat.
#[derive(Debug, Clone, PartialEq)]
pub struct Kachelplan {
    pub kacheln: Vec<Kachel>,
    pub zeilen: usize,
    pub spalten: usize,
    /// Blattgröße in der gewählten Ausrichtung (Breite, Höhe).
    pub blatt_breite: f64,
    pub blatt_hoehe: f64,
    pub hochformat: bool,
    pub rand: f64,
    pub klebekante: f64,
}

impl Kachelplan {
    /// Bedruckbare Fläche eines Blatts (Breite, Höhe).
    pub fn nutzflaeche(&self) -> (f64, f64) {
        (
            self.blatt_breite - 2.0 * self.rand,
            self.blatt_hoehe - 2.0 * self.rand,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KachelFehler {
    RandAusserhalb,
    KlebekanteAusserhalb,
    /// Rand und Klebekante lassen auf dem Papier keinen Platz mehr.
    PapierZuKlein,
    ZuVieleBlaetter,
    UngueltigerBereich,
}

/// Verteilt den Bereich `von`..`bis` auf Blätter. Hoch- oder Querformat wird
/// so gewählt, dass möglichst wenige Blätter nötig sind (bei Gleichstand
/// Hochformat).
pub fn kacheln(von: Punkt, bis: Punkt, e: Einstellung) -> Result<Kachelplan, KachelFehler> {
    let rand = e.rand.wert();
    let kleben = e.klebekante.wert();
    if !RAND_MM.contains(&rand) {
        return Err(KachelFehler::RandAusserhalb);
    }
    if !KLEBEKANTE_MM.contains(&kleben) {
        return Err(KachelFehler::KlebekanteAusserhalb);
    }
    let breite = bis.x - von.x;
    let hoehe = bis.y - von.y;
    if !(breite.is_finite() && hoehe.is_finite() && breite > 0.0 && hoehe > 0.0) {
        return Err(KachelFehler::UngueltigerBereich);
    }

    let hoch = plan(breite, hoehe, e.papier.breite, e.papier.hoehe, rand, kleben);
    let quer = plan(breite, hoehe, e.papier.hoehe, e.papier.breite, rand, kleben);
    let (hochformat, (spalten, zeilen)) = match (hoch, quer) {
        (Some(h), Some(q)) if q.0 * q.1 < h.0 * h.1 => (false, q),
        (Some(h), _) => (true, h),
        (None, Some(q)) => (false, q),
        (None, None) => return Err(KachelFehler::PapierZuKlein),
    };
    if spalten.saturating_mul(zeilen) > MAX_BLAETTER {
        return Err(KachelFehler::ZuVieleBlaetter);
    }
    let (blatt_breite, blatt_hoehe) = if hochformat {
        (e.papier.breite, e.papier.hoehe)
    } else {
        (e.papier.hoehe, e.papier.breite)
    };
    let nutz_b = blatt_breite - 2.0 * rand;
    let nutz_h = blatt_hoehe - 2.0 * rand;

    let mut kacheln = Vec::with_capacity(spalten * zeilen);
    for zeile in 0..zeilen {
        for spalte in 0..spalten {
            let x = von.x + spalte as f64 * (nutz_b - kleben);
            let y = von.y + zeile as f64 * (nutz_h - kleben);
            kacheln.push(Kachel {
                zeile,
                spalte,
                von: Punkt::neu(x, y),
                bis: Punkt::neu(x + nutz_b, y + nutz_h),
                klebekante_rechts: spalte + 1 < spalten,
                klebekante_unten: zeile + 1 < zeilen,
            });
        }
    }
    Ok(Kachelplan {
        kacheln,
        zeilen,
        spalten,
        blatt_breite,
        blatt_hoehe,
        hochformat,
        rand,
        klebekante: kleben,
    })
}

/// Anzahl (Spalten, Zeilen) für eine Blattausrichtung, `None` wenn das
/// Papier für Rand und Klebekante zu klein ist.
fn plan(b: f64, h: f64, pb: f64, ph: f64, rand: f64, kleben: f64) -> Option<(usize, usize)> {
    let schritt_b = pb - 2.0 * rand - kleben;
    let schritt_h = ph - 2.0 * rand - kleben;
    if schritt_b < MIN_NUTZFLAECHE_MM || schritt_h < MIN_NUTZFLAECHE_MM {
        return None;
    }
    let anzahl = |laenge: f64, schritt: f64| {
        // Erstes Blatt deckt schritt + kleben ab, jedes weitere schritt.
        let rest = (laenge - kleben).max(0.0);
        ((rest / schritt).ceil() as usize).max(1)
    };
    Some((anzahl(b, schritt_b), anzahl(h, schritt_h)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    #[test]
    fn ein_kleines_teil_passt_auf_ein_einziges_a4_blatt() {
        let p = kacheln(
            Punkt::neu(0.0, 0.0),
            Punkt::neu(100.0, 70.0),
            Einstellung::default(),
        )
        .unwrap();
        assert_eq!(p.kacheln.len(), 1);
        assert!(p.hochformat);
        assert!(!p.kacheln[0].klebekante_rechts && !p.kacheln[0].klebekante_unten);
    }

    #[test]
    fn ein_breites_teil_wird_quer_gedruckt_wenn_das_blaetter_spart() {
        // 270 × 150 mm: quer passt es mit 10 mm Rand auf ein Blatt (277 × 190).
        let p = kacheln(
            Punkt::neu(0.0, 0.0),
            Punkt::neu(270.0, 150.0),
            Einstellung::default(),
        )
        .unwrap();
        assert!(!p.hochformat);
        assert_eq!(p.kacheln.len(), 1);
    }

    #[test]
    fn eine_grosse_tasche_wird_auf_mehrere_blaetter_mit_klebekanten_verteilt() {
        // 400 × 300 mm auf A4 mit 10 mm Rand und 10 mm Klebekante:
        // hoch wären es 3 × 2 Blätter, quer nur 2 × 2.
        let p = kacheln(
            Punkt::neu(0.0, 0.0),
            Punkt::neu(400.0, 300.0),
            Einstellung::default(),
        )
        .unwrap();
        assert!(!p.hochformat);
        assert_eq!(p.kacheln.len(), 4);
        let rechts = p.kacheln.iter().filter(|k| k.klebekante_rechts).count();
        let unten = p.kacheln.iter().filter(|k| k.klebekante_unten).count();
        assert_eq!(rechts, p.zeilen * (p.spalten - 1));
        assert_eq!(unten, p.spalten * (p.zeilen - 1));
    }

    #[test]
    fn benachbarte_blaetter_ueberlappen_genau_um_die_klebekante() {
        let p = kacheln(
            Punkt::neu(0.0, 0.0),
            Punkt::neu(400.0, 300.0),
            Einstellung::default(),
        )
        .unwrap();
        let a1 = p.kacheln[0];
        let a2 = p.kacheln[1];
        assert!((a1.bis.x - a2.von.x - 10.0).abs() < 1e-9);
        let b1 = p.kacheln[p.spalten];
        assert!((a1.bis.y - b1.von.y - 10.0).abs() < 1e-9);
    }

    #[test]
    fn jeder_punkt_des_teils_landet_auf_mindestens_einem_blatt() {
        for (b, h) in [
            (50.0, 50.0),
            (400.0, 300.0),
            (1200.0, 450.0),
            (189.9, 277.0),
        ] {
            let p = kacheln(
                Punkt::neu(-5.0, 3.0),
                Punkt::neu(b - 5.0, h + 3.0),
                Einstellung::default(),
            )
            .unwrap();
            for i in 0..=40 {
                for j in 0..=40 {
                    let q = Punkt::neu(-5.0 + b * i as f64 / 40.0, 3.0 + h * j as f64 / 40.0);
                    assert!(
                        p.kacheln.iter().any(|k| q.x >= k.von.x
                            && q.x <= k.bis.x
                            && q.y >= k.von.y
                            && q.y <= k.bis.y),
                        "{q:?} fehlt bei {b} × {h}"
                    );
                }
            }
        }
    }

    #[test]
    fn blaetter_heissen_wie_planquadrate() {
        assert_eq!(name(0, 0), "A1");
        assert_eq!(name(1, 2), "B3");
        assert_eq!(name(26, 0), "AA1");
    }

    #[test]
    fn unsinnige_einstellungen_werden_abgewiesen() {
        let von = Punkt::neu(0.0, 0.0);
        let bis = Punkt::neu(100.0, 100.0);
        let mit = |rand: f64, kleben: f64| Einstellung {
            rand: mm(rand),
            klebekante: mm(kleben),
            ..Einstellung::default()
        };
        assert_eq!(
            kacheln(von, bis, mit(40.0, 10.0)),
            Err(KachelFehler::RandAusserhalb)
        );
        assert_eq!(
            kacheln(von, bis, mit(10.0, 60.0)),
            Err(KachelFehler::KlebekanteAusserhalb)
        );
        let winzig = Einstellung {
            papier: Papier {
                breite: 60.0,
                hoehe: 60.0,
            },
            ..mit(10.0, 25.0)
        };
        assert_eq!(kacheln(von, bis, winzig), Err(KachelFehler::PapierZuKlein));
        assert_eq!(
            kacheln(von, Punkt::neu(5000.0, 5000.0), Einstellung::default()),
            Err(KachelFehler::ZuVieleBlaetter)
        );
    }
}
