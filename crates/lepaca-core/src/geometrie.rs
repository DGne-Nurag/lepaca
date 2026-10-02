//! Punkte und Konturen.
//!
//! Koordinaten sind Millimeter. Der Ursprung liegt oben links, die y-Achse
//! zeigt nach unten (wie bei SVG und auf dem Bildschirm).

use crate::mass::TOLERANZ_MM;

/// Höchstzahl Punkte pro Kontur. Schützt vor Ressourcenerschöpfung
/// (Playbook 01: jede Schleife über Eingabedaten hat eine Obergrenze).
pub const MAX_PUNKTE: usize = 20_000;

/// Ein Punkt (oder Richtungsvektor) in Millimetern.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Punkt {
    pub x: f64,
    pub y: f64,
}

impl Punkt {
    pub const fn neu(x: f64, y: f64) -> Self {
        Punkt { x, y }
    }

    pub fn plus(self, o: Punkt) -> Punkt {
        Punkt::neu(self.x + o.x, self.y + o.y)
    }

    pub fn minus(self, o: Punkt) -> Punkt {
        Punkt::neu(self.x - o.x, self.y - o.y)
    }

    pub fn mal(self, f: f64) -> Punkt {
        Punkt::neu(self.x * f, self.y * f)
    }

    pub fn skalar(self, o: Punkt) -> f64 {
        self.x * o.x + self.y * o.y
    }

    pub fn kreuz(self, o: Punkt) -> f64 {
        self.x * o.y - self.y * o.x
    }

    pub fn laenge(self) -> f64 {
        self.x.hypot(self.y)
    }

    pub fn abstand(self, o: Punkt) -> f64 {
        self.minus(o).laenge()
    }

    /// Einheitsvektor in dieselbe Richtung, `None` bei Länge null.
    pub fn normiert(self) -> Option<Punkt> {
        let l = self.laenge();
        (l > 1e-12).then(|| self.mal(1.0 / l))
    }

    /// Um `winkel` (Bogenmaß) gedreht.
    pub fn gedreht(self, winkel: f64) -> Punkt {
        let (s, c) = winkel.sin_cos();
        Punkt::neu(self.x * c - self.y * s, self.x * s + self.y * c)
    }

    pub fn ist_endlich(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// Fehler beim Anlegen einer Kontur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KonturFehler {
    ZuWenigPunkte,
    ZuVielePunkte,
    UngueltigeZahl,
    KeineFlaeche,
}

/// Eine geschlossene Umrisslinie als Punktfolge. Zu jedem Punkt ist vermerkt,
/// ob dort eine echte Ecke liegt (Knick in der Linie) oder die Linie glatt
/// weiterläuft (z. B. auf einem Bogen).
#[derive(Debug, Clone, PartialEq)]
pub struct Kontur {
    punkte: Vec<Punkt>,
    ecke: Vec<bool>,
}

impl Kontur {
    /// Legt eine Kontur an. Doppelte Punkte direkt hintereinander werden
    /// zusammengefasst, ungültige Eingaben abgewiesen.
    pub fn neu(punkte: Vec<Punkt>, ecke: Vec<bool>) -> Result<Self, KonturFehler> {
        if punkte.len() != ecke.len() {
            return Err(KonturFehler::ZuWenigPunkte);
        }
        if punkte.len() > MAX_PUNKTE {
            return Err(KonturFehler::ZuVielePunkte);
        }
        if punkte.iter().any(|p| !p.ist_endlich()) {
            return Err(KonturFehler::UngueltigeZahl);
        }
        let mut p2: Vec<Punkt> = Vec::with_capacity(punkte.len());
        let mut e2: Vec<bool> = Vec::with_capacity(ecke.len());
        for (p, e) in punkte.into_iter().zip(ecke) {
            match p2.last() {
                Some(letzter) if letzter.abstand(p) < 1e-9 => {
                    let i = e2.len() - 1;
                    e2[i] |= e;
                }
                _ => {
                    p2.push(p);
                    e2.push(e);
                }
            }
        }
        while p2.len() > 1 && p2[0].abstand(p2[p2.len() - 1]) < 1e-9 {
            p2.pop();
            let e = e2.pop().unwrap_or(false);
            e2[0] |= e;
        }
        if p2.len() < 3 {
            return Err(KonturFehler::ZuWenigPunkte);
        }
        let k = Kontur {
            punkte: p2,
            ecke: e2,
        };
        if k.flaeche_mit_vorzeichen().abs() < TOLERANZ_MM * TOLERANZ_MM {
            return Err(KonturFehler::KeineFlaeche);
        }
        Ok(k)
    }

    /// Dieselbe Kontur, um (dx, dy) verschoben.
    pub fn verschoben(&self, dx: f64, dy: f64) -> Kontur {
        let d = Punkt::neu(dx, dy);
        Kontur {
            punkte: self.punkte.iter().map(|p| p.plus(d)).collect(),
            ecke: self.ecke.clone(),
        }
    }

    pub fn punkte(&self) -> &[Punkt] {
        &self.punkte
    }

    pub fn ist_ecke(&self, i: usize) -> bool {
        self.ecke[i]
    }

    pub fn ecken(&self) -> &[bool] {
        &self.ecke
    }

    pub fn len(&self) -> usize {
        self.punkte.len()
    }

    pub fn is_empty(&self) -> bool {
        self.punkte.is_empty()
    }

    /// Punkt mit Index modulo Länge (die Kontur ist geschlossen).
    pub fn punkt(&self, i: usize) -> Punkt {
        self.punkte[i % self.punkte.len()]
    }

    /// Fläche nach der Gaußschen Trapezformel; das Vorzeichen zeigt die
    /// Umlaufrichtung.
    pub fn flaeche_mit_vorzeichen(&self) -> f64 {
        let n = self.punkte.len();
        (0..n)
            .map(|i| self.punkte[i].kreuz(self.punkte[(i + 1) % n]))
            .sum::<f64>()
            / 2.0
    }

    /// Umfang in Millimetern.
    pub fn umfang(&self) -> f64 {
        let n = self.punkte.len();
        (0..n)
            .map(|i| self.punkt(i).abstand(self.punkt(i + 1)))
            .sum()
    }

    /// Kleinstes umschließendes Rechteck: (min, max).
    pub fn grenzen(&self) -> (Punkt, Punkt) {
        let mut min = Punkt::neu(f64::INFINITY, f64::INFINITY);
        let mut max = Punkt::neu(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in &self.punkte {
            min = Punkt::neu(min.x.min(p.x), min.y.min(p.y));
            max = Punkt::neu(max.x.max(p.x), max.y.max(p.y));
        }
        (min, max)
    }

    /// Prüft, ob sich zwei nicht benachbarte Kanten schneiden.
    pub fn schneidet_sich_selbst(&self) -> bool {
        let n = self.punkte.len();
        for i in 0..n {
            let (a, b) = (self.punkt(i), self.punkt(i + 1));
            for j in (i + 2)..n {
                if i == 0 && j == n - 1 {
                    continue;
                }
                let (c, d) = (self.punkt(j), self.punkt(j + 1));
                if strecken_schneiden(a, b, c, d) {
                    return true;
                }
            }
        }
        false
    }
}

fn strecken_schneiden(a: Punkt, b: Punkt, c: Punkt, d: Punkt) -> bool {
    let d1 = b.minus(a).kreuz(c.minus(a));
    let d2 = b.minus(a).kreuz(d.minus(a));
    let d3 = d.minus(c).kreuz(a.minus(c));
    let d4 = d.minus(c).kreuz(b.minus(c));
    (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadrat() -> Kontur {
        let p = vec![
            Punkt::neu(0.0, 0.0),
            Punkt::neu(10.0, 0.0),
            Punkt::neu(10.0, 10.0),
            Punkt::neu(0.0, 10.0),
        ];
        Kontur::neu(p, vec![true; 4]).unwrap()
    }

    #[test]
    fn ein_quadrat_mit_10_mm_kante_hat_40_mm_umfang_und_100_mm2_flaeche() {
        let q = quadrat();
        assert!((q.umfang() - 40.0).abs() < TOLERANZ_MM);
        assert!((q.flaeche_mit_vorzeichen().abs() - 100.0).abs() < TOLERANZ_MM);
        assert!(!q.schneidet_sich_selbst());
    }

    #[test]
    fn verschieben_aendert_weder_form_noch_ecken() {
        let q = quadrat();
        let v = q.verschoben(25.0, -5.0);
        assert!((v.umfang() - q.umfang()).abs() < TOLERANZ_MM);
        assert_eq!(v.ecken(), q.ecken());
        assert_eq!(v.grenzen().0, Punkt::neu(25.0, -5.0));
    }

    #[test]
    fn eine_verdrehte_acht_schneidet_sich_selbst() {
        let p = vec![
            Punkt::neu(0.0, 0.0),
            Punkt::neu(10.0, 10.0),
            Punkt::neu(10.0, 0.0),
            Punkt::neu(0.0, 5.0),
        ];
        assert!(
            Kontur::neu(p, vec![true; 4])
                .unwrap()
                .schneidet_sich_selbst()
        );
    }

    #[test]
    fn ungueltige_zahlen_und_leere_formen_werden_abgewiesen() {
        let nan = vec![Punkt::neu(f64::NAN, 0.0); 3];
        assert_eq!(
            Kontur::neu(nan, vec![false; 3]),
            Err(KonturFehler::UngueltigeZahl)
        );
        let linie = vec![
            Punkt::neu(0.0, 0.0),
            Punkt::neu(5.0, 0.0),
            Punkt::neu(10.0, 0.0),
        ];
        assert_eq!(
            Kontur::neu(linie, vec![false; 3]),
            Err(KonturFehler::KeineFlaeche)
        );
    }

    #[test]
    fn doppelte_punkte_und_ein_wiederholter_startpunkt_werden_zusammengefasst() {
        let p = vec![
            Punkt::neu(0.0, 0.0),
            Punkt::neu(10.0, 0.0),
            Punkt::neu(10.0, 0.0),
            Punkt::neu(10.0, 10.0),
            Punkt::neu(0.0, 0.0),
        ];
        let k = Kontur::neu(p, vec![false; 5]).unwrap();
        assert_eq!(k.len(), 3);
    }
}
