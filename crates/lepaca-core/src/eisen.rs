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
    /// Rund: runde Löcher, etwa vom Rundloch-Eisen, von der Ahle oder
    /// der Lochzange.
    Rund,
}

impl Eisenart {
    pub const ALLE: [Eisenart; 4] = [
        Eisenart::Franzoesisch,
        Eisenart::Diamant,
        Eisenart::Japanisch,
        Eisenart::Rund,
    ];

    /// Lochform für einen gegebenen Zinkenabstand.
    pub fn lochform(self, lochabstand: Mm) -> Lochform {
        let p = lochabstand.wert();
        if self == Eisenart::Rund {
            // Durchmesser relativ zum Abstand (bei 3,85 mm etwa 1,2 mm).
            let d = p * 0.30;
            return Lochform {
                laenge: d,
                breite: d,
                winkel_grad: 0.0,
                rund: true,
            };
        }
        // (Länge, Breite) relativ zum Abstand, Winkel zur Nahtrichtung.
        let (l, b, w) = match self {
            Eisenart::Franzoesisch => (0.60, 0.15, 35.0),
            Eisenart::Diamant => (0.55, 0.25, 45.0),
            Eisenart::Japanisch => (0.50, 0.10, 25.0),
            Eisenart::Rund => unreachable!("oben behandelt"),
        };
        Lochform {
            laenge: p * l,
            breite: p * b,
            winkel_grad: w,
            rund: false,
        }
    }
}

/// Raute, deren lange Achse um `winkel_grad` gegen die Nahtrichtung gedreht
/// ist, oder ein Kreis mit `laenge` als Durchmesser (`rund`). Werte in
/// Millimetern.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lochform {
    pub laenge: f64,
    pub breite: f64,
    pub winkel_grad: f64,
    pub rund: bool,
}

/// Ecken des Vielecks, das ein rundes Loch annähert. Bei 2 mm Durchmesser
/// weicht es höchstens r·(1 − cos(π/n)) ≈ 0,02 mm vom Kreis ab, bei den
/// üblichen gut 1 mm etwa 0,01 mm.
const RUNDLOCH_ECKEN: usize = 16;

impl Lochform {
    /// Die Eckpunkte des Lochs um die Lochmitte: vier bei der Raute,
    /// ein feines Vieleck beim runden Loch.
    pub fn umriss(&self, loch: &Stichloch) -> Vec<Punkt> {
        let m = loch.mitte;
        if self.rund {
            let r = self.laenge / 2.0;
            return (0..RUNDLOCH_ECKEN)
                .map(|i| {
                    let w = std::f64::consts::TAU * i as f64 / RUNDLOCH_ECKEN as f64;
                    Punkt::neu(m.x + r * w.cos(), m.y + r * w.sin())
                })
                .collect();
        }
        let achse = loch.richtung.gedreht(self.winkel_grad.to_radians());
        let quer = achse.gedreht(std::f64::consts::FRAC_PI_2);
        vec![
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

    #[test]
    fn rundes_loch_ist_ein_kreis_um_den_stichpunkt() {
        let loch = Stichloch {
            mitte: Punkt::neu(10.0, 10.0),
            richtung: Punkt::neu(1.0, 0.0),
            auf_ecke: false,
        };
        let f = Eisenart::Rund.lochform(Mm::neu(3.85).unwrap());
        assert!(f.rund);
        // Bei 3,85 mm Abstand gut 1,1 mm Durchmesser.
        assert!((f.laenge - 1.155).abs() < 1e-9);
        for p in f.umriss(&loch) {
            assert!((p.abstand(loch.mitte) - f.laenge / 2.0).abs() < 1e-9);
        }
    }

    #[test]
    fn rundes_loch_hat_in_jeder_richtung_dieselbe_groesse() {
        let f = Eisenart::Rund.lochform(Mm::neu(4.0).unwrap());
        assert!((f.laenge - f.breite).abs() < 1e-12);
        assert!((f.laenge_in_nahtrichtung() - f.laenge).abs() < 1e-9);
    }
}
