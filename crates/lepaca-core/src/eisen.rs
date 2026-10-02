//! Eisenarten (Pricking-Eisen) und die Form ihrer Löcher.
//!
//! Die Maße sind **Startwerte** als Verhältnis zum Zinkenabstand. Sie müssen
//! noch an echten Eisen nachgemessen werden und sind deshalb bewusst an
//! einer Stelle gebündelt.

use crate::geometrie::Punkt;
use crate::mass::Mm;
use crate::stichlinie::Stichloch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eisenart {
    /// Französisch: schmale, deutlich schräge Zinken.
    Franzoesisch,
    /// Diamant: breitere Rautenform, schräg.
    Diamant,
    /// Japanisch: sehr schmale, flach angestellte Schlitze.
    Japanisch,
}

impl Eisenart {
    pub const ALLE: [Eisenart; 3] = [
        Eisenart::Franzoesisch,
        Eisenart::Diamant,
        Eisenart::Japanisch,
    ];

    /// Lochform für einen gegebenen Zinkenabstand.
    pub fn lochform(self, lochabstand: Mm) -> Lochform {
        // (Länge, Breite) relativ zum Abstand, Winkel zur Nahtrichtung.
        let (l, b, w) = match self {
            Eisenart::Franzoesisch => (0.60, 0.15, 35.0),
            Eisenart::Diamant => (0.55, 0.25, 45.0),
            Eisenart::Japanisch => (0.50, 0.10, 25.0),
        };
        let p = lochabstand.wert();
        Lochform {
            laenge: p * l,
            breite: p * b,
            winkel_grad: w,
        }
    }
}

/// Raute, deren lange Achse um `winkel_grad` gegen die Nahtrichtung gedreht
/// ist. Werte in Millimetern.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lochform {
    pub laenge: f64,
    pub breite: f64,
    pub winkel_grad: f64,
}

impl Lochform {
    /// Die vier Eckpunkte des Lochs um die Lochmitte.
    pub fn umriss(&self, loch: &Stichloch) -> [Punkt; 4] {
        let achse = loch.richtung.gedreht(self.winkel_grad.to_radians());
        let quer = achse.gedreht(std::f64::consts::FRAC_PI_2);
        let m = loch.mitte;
        [
            m.plus(achse.mal(self.laenge / 2.0)),
            m.plus(quer.mal(self.breite / 2.0)),
            m.minus(achse.mal(self.laenge / 2.0)),
            m.minus(quer.mal(self.breite / 2.0)),
        ]
    }

    /// Ausdehnung des Lochs in Nahtrichtung.
    pub fn laenge_in_nahtrichtung(&self) -> f64 {
        let w = self.winkel_grad.to_radians();
        self.laenge * w.cos().abs() + self.breite * w.sin().abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bei_allen_eisen_und_ueblichen_abstaenden_beruehren_sich_die_loecher_nie() {
        for art in Eisenart::ALLE {
            for p in [2.0, 2.7, 3.0, 3.38, 3.85, 4.0, 5.0, 6.0] {
                let f = art.lochform(Mm::neu(p).unwrap());
                assert!(f.laenge_in_nahtrichtung() < 0.8 * p, "{art:?} bei {p} mm");
            }
        }
    }

    #[test]
    fn das_loch_liegt_mittig_auf_dem_stichpunkt_und_ist_schraeg() {
        let loch = Stichloch {
            mitte: Punkt::neu(10.0, 10.0),
            richtung: Punkt::neu(1.0, 0.0),
            auf_ecke: false,
        };
        let f = Eisenart::Diamant.lochform(Mm::neu(4.0).unwrap());
        let u = f.umriss(&loch);
        let mitte = u
            .iter()
            .fold(Punkt::neu(0.0, 0.0), |s, p| s.plus(*p))
            .mal(0.25);
        assert!(mitte.abstand(loch.mitte) < 1e-9);
        // Lange Achse um 45° gedreht: Spitzen liegen diagonal.
        let achse = u[0].minus(u[2]);
        assert!((achse.x.abs() - achse.y.abs()).abs() < 1e-9);
        assert!((achse.laenge() - 2.2).abs() < 1e-9);
    }
}
