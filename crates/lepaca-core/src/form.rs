//! Grundformen eines Lederteils.
//!
//! Jede Form wird für Berechnungen in eine [`Kontur`] umgewandelt. Bögen und
//! Kurven werden dabei so fein aufgeteilt, dass die Abweichung höchstens
//! [`FLACH_TOLERANZ_MM`] beträgt.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use crate::geometrie::{Kontur, KonturFehler, MAX_PUNKTE, Punkt};
use crate::mass::Mm;

/// Größte Abweichung zwischen echter Kurve und Punktfolge.
pub const FLACH_TOLERANZ_MM: f64 = 0.01;

/// Kleinste und größte erlaubte Ausdehnung einer Form.
pub const MIN_GROESSE_MM: f64 = 1.0;
pub const MAX_GROESSE_MM: f64 = 5_000.0;

/// Ab diesem Knickwinkel gilt ein Übergang zwischen zwei Pfadstücken als Ecke.
pub const ECKEN_WINKEL_GRAD: f64 = 10.0;

/// Eine Grundform. Rechteck, Kreis und Ellipse liegen mit ihrer linken
/// oberen Ecke bei (0, 0).
#[derive(Debug, Clone, PartialEq)]
pub enum Form {
    /// Rechteck; mit `eckenradius` größer null abgerundet.
    Rechteck {
        breite: Mm,
        hoehe: Mm,
        eckenradius: Mm,
    },
    Kreis {
        durchmesser: Mm,
    },
    Ellipse {
        breite: Mm,
        hoehe: Mm,
    },
    /// Kartenfach in T-Form: oben der volle Querbalken mit den Flügeln,
    /// die in der Randnaht mitlaufen; darunter der schmalere Steg, der
    /// beim Staffeln unter dem nächsten Fach verschwindet.
    TPocket {
        /// Breite über die Flügel gemessen.
        breite: Mm,
        /// Höhe von der Öffnung bis zur Unterkante des Stegs.
        hoehe: Mm,
        /// Wie weit jeder Flügel seitlich über den Steg hinausragt.
        fluegel_ueberstand: Mm,
        /// Höhe der Flügel ab der Öffnung, meist gleich dem Versatz
        /// zum nächsten Fach.
        fluegel_hoehe: Mm,
    },
    /// Freie, geschlossene Form aus Linien und Kurven.
    Pfad(Pfad),
}

/// Nahtwahl für eine T-Pocket, in Begriffen des Teils statt Kantennummern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TPocketNaehte {
    /// Seitenkanten der Flügel; sie laufen in der Randnaht mit.
    pub fluegel: bool,
    /// Unterkante des Stegs, quer genäht.
    pub steg_unten: bool,
    /// Oberkante (Öffnung). Normalerweise nur gekantet, nicht genäht.
    pub oeffnung: bool,
}

impl Default for TPocketNaehte {
    fn default() -> Self {
        TPocketNaehte {
            fluegel: true,
            steg_unten: true,
            oeffnung: false,
        }
    }
}

impl TPocketNaehte {
    /// Kantenwahl passend zur Kontur von [`Form::TPocket`]: Kante `i`
    /// läuft von Punkt `i` zu Punkt `i + 1`.
    pub fn kanten(self) -> Vec<bool> {
        // 0 Öffnung, 1 Flügel rechts, 2 Flügel rechts unten, 3 Steg rechts,
        // 4 Steg unten, 5 Steg links, 6 Flügel links unten, 7 Flügel links.
        vec![
            self.oeffnung,
            self.fluegel,
            false,
            false,
            self.steg_unten,
            false,
            false,
            self.fluegel,
        ]
    }
}

/// Mindestbreite des Stegs, damit noch eine Karte hält und genäht werden kann.
pub const MIN_STEG_MM: f64 = 10.0;

/// Ein geschlossener Pfad. Nach dem letzten Element wird automatisch
/// zum Start zurückgezogen.
#[derive(Debug, Clone, PartialEq)]
pub struct Pfad {
    pub start: Punkt,
    pub elemente: Vec<PfadElement>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PfadElement {
    Linie(Punkt),
    /// Kubische Bézierkurve mit zwei Kontrollpunkten.
    Kurve {
        k1: Punkt,
        k2: Punkt,
        ziel: Punkt,
    },
}

/// Warum eine Form nicht gebaut werden kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFehler {
    /// Breite, Höhe oder Durchmesser liegt außerhalb von
    /// [`MIN_GROESSE_MM`]..=[`MAX_GROESSE_MM`].
    GroesseAusserhalb,
    /// Die Flügel einer T-Pocket sind so breit, dass vom Steg weniger als
    /// [`MIN_STEG_MM`] bleibt, oder so hoch wie das ganze Fach.
    FluegelZuGross,
    /// Der Eckenradius ist größer als die halbe kürzere Seite.
    EckenradiusZuGross,
    Kontur(KonturFehler),
}

impl From<KonturFehler> for FormFehler {
    fn from(f: KonturFehler) -> Self {
        FormFehler::Kontur(f)
    }
}

impl Form {
    /// Wandelt die Form in eine Kontur um.
    pub fn kontur(&self) -> Result<Kontur, FormFehler> {
        match self {
            Form::Rechteck {
                breite,
                hoehe,
                eckenradius,
            } => rechteck(breite.wert(), hoehe.wert(), eckenradius.wert()),
            Form::Kreis { durchmesser } => {
                let d = durchmesser.wert();
                ellipse(d, d)
            }
            Form::Ellipse { breite, hoehe } => ellipse(breite.wert(), hoehe.wert()),
            Form::TPocket {
                breite,
                hoehe,
                fluegel_ueberstand,
                fluegel_hoehe,
            } => t_pocket(
                breite.wert(),
                hoehe.wert(),
                fluegel_ueberstand.wert(),
                fluegel_hoehe.wert(),
            ),
            Form::Pfad(p) => pfad(p),
        }
    }
}

fn pruefe_groesse(w: f64) -> Result<(), FormFehler> {
    if (MIN_GROESSE_MM..=MAX_GROESSE_MM).contains(&w) {
        Ok(())
    } else {
        Err(FormFehler::GroesseAusserhalb)
    }
}

/// Schrittwinkel, damit ein Bogen mit Radius `r` höchstens um die Toleranz
/// von der Sehne abweicht.
fn bogen_schritt(r: f64) -> f64 {
    if r <= FLACH_TOLERANZ_MM {
        FRAC_PI_2
    } else {
        (2.0 * (1.0 - FLACH_TOLERANZ_MM / r).acos()).min(FRAC_PI_2)
    }
}

fn rechteck(b: f64, h: f64, r: f64) -> Result<Kontur, FormFehler> {
    pruefe_groesse(b)?;
    pruefe_groesse(h)?;
    if !(0.0..=b.min(h) / 2.0).contains(&r) {
        return Err(FormFehler::EckenradiusZuGross);
    }
    if r < FLACH_TOLERANZ_MM {
        let p = vec![
            Punkt::neu(0.0, 0.0),
            Punkt::neu(b, 0.0),
            Punkt::neu(b, h),
            Punkt::neu(0.0, h),
        ];
        return Ok(Kontur::neu(p, vec![true; 4])?);
    }
    // Vier Viertelbögen, im Uhrzeigersinn auf dem Bildschirm.
    let mitten = [
        (Punkt::neu(b - r, r), -FRAC_PI_2),
        (Punkt::neu(b - r, h - r), 0.0),
        (Punkt::neu(r, h - r), FRAC_PI_2),
        (Punkt::neu(r, r), PI),
    ];
    let n = (FRAC_PI_2 / bogen_schritt(r)).ceil().max(1.0) as usize;
    let mut punkte = Vec::new();
    for (m, start) in mitten {
        for i in 0..=n {
            let w = start + FRAC_PI_2 * i as f64 / n as f64;
            punkte.push(m.plus(Punkt::neu(w.cos(), w.sin()).mal(r)));
        }
    }
    let ecke = vec![false; punkte.len()];
    Ok(Kontur::neu(punkte, ecke)?)
}

fn t_pocket(b: f64, h: f64, u: f64, fh: f64) -> Result<Kontur, FormFehler> {
    pruefe_groesse(b)?;
    pruefe_groesse(h)?;
    if u <= 0.0 || b - 2.0 * u < MIN_STEG_MM || fh <= 0.0 || fh >= h {
        return Err(FormFehler::FluegelZuGross);
    }
    // Im Uhrzeigersinn auf dem Bildschirm, beginnend oben links.
    let p = vec![
        Punkt::neu(0.0, 0.0),
        Punkt::neu(b, 0.0),
        Punkt::neu(b, fh),
        Punkt::neu(b - u, fh),
        Punkt::neu(b - u, h),
        Punkt::neu(u, h),
        Punkt::neu(u, fh),
        Punkt::neu(0.0, fh),
    ];
    Ok(Kontur::neu(p, vec![true; 8])?)
}

fn ellipse(b: f64, h: f64) -> Result<Kontur, FormFehler> {
    pruefe_groesse(b)?;
    pruefe_groesse(h)?;
    let (a, c) = (b / 2.0, h / 2.0);
    let n = (TAU / bogen_schritt(a.max(c))).ceil().max(8.0) as usize;
    let punkte: Vec<Punkt> = (0..n)
        .map(|i| {
            let w = TAU * i as f64 / n as f64;
            Punkt::neu(a + a * w.cos(), c + c * w.sin())
        })
        .collect();
    let ecke = vec![false; punkte.len()];
    Ok(Kontur::neu(punkte, ecke)?)
}

fn ist_knick(ein: Option<Punkt>, aus: Option<Punkt>) -> bool {
    match (ein.and_then(Punkt::normiert), aus.and_then(Punkt::normiert)) {
        (Some(a), Some(b)) => a.skalar(b) < ECKEN_WINKEL_GRAD.to_radians().cos(),
        _ => false,
    }
}

fn pfad(p: &Pfad) -> Result<Kontur, FormFehler> {
    let mut punkte = vec![p.start];
    let mut ecke = vec![false];
    let mut letzte_tangente: Option<Punkt> = None;
    let mut erste_tangente: Option<Punkt> = None;

    for el in &p.elemente {
        let von = punkte[punkte.len() - 1];
        let (t_start, t_ende, neue) = match *el {
            PfadElement::Linie(ziel) => (ziel.minus(von), ziel.minus(von), vec![ziel]),
            PfadElement::Kurve { k1, k2, ziel } => {
                let t_start = if k1.abstand(von) > 1e-9 { k1 } else { k2 }.minus(von);
                let t_ende = ziel.minus(if ziel.abstand(k2) > 1e-9 { k2 } else { k1 });
                (t_start, t_ende, kurve_flach(von, k1, k2, ziel))
            }
        };
        if erste_tangente.is_none() {
            erste_tangente = Some(t_start);
        }
        let j = ecke.len() - 1;
        ecke[j] = ist_knick(letzte_tangente, Some(t_start));
        for q in neue {
            if punkte.len() >= MAX_PUNKTE {
                return Err(KonturFehler::ZuVielePunkte.into());
            }
            punkte.push(q);
            ecke.push(false);
        }
        letzte_tangente = Some(t_ende);
    }

    // Schließen: entweder endet der Pfad schon am Start, oder es kommt eine
    // gerade Schlusslinie dazu.
    let letzter = punkte[punkte.len() - 1];
    if letzter.abstand(p.start) < 1e-9 {
        punkte.pop();
        ecke.pop();
    } else {
        let t_schluss = p.start.minus(letzter);
        let j = ecke.len() - 1;
        ecke[j] = ist_knick(letzte_tangente, Some(t_schluss));
        letzte_tangente = Some(t_schluss);
    }
    ecke[0] = ist_knick(letzte_tangente, erste_tangente);

    let kontur = Kontur::neu(punkte, ecke)?;
    let (min, max) = kontur.grenzen();
    pruefe_groesse((max.x - min.x).max(max.y - min.y))?;
    Ok(kontur)
}

/// Teilt eine kubische Bézierkurve gleichmäßig auf. Die Schrittzahl folgt aus
/// der Fehlerschranke für lineare Interpolation: Fehler ≤ ¾·max|Δ²P| / n².
fn kurve_flach(p0: Punkt, p1: Punkt, p2: Punkt, p3: Punkt) -> Vec<Punkt> {
    let dd1 = p0.minus(p1.mal(2.0)).plus(p2).laenge();
    let dd2 = p1.minus(p2.mal(2.0)).plus(p3).laenge();
    let dd = dd1.max(dd2);
    let n = (0.75 * dd / FLACH_TOLERANZ_MM)
        .sqrt()
        .ceil()
        .clamp(1.0, 1000.0) as usize;
    (1..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            p0.mal(u * u * u)
                .plus(p1.mal(3.0 * u * u * t))
                .plus(p2.mal(3.0 * u * t * t))
                .plus(p3.mal(t * t * t))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass::TOLERANZ_MM;

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    #[test]
    fn ein_rechteck_80_x_50_hat_vier_ecken_und_260_mm_umfang() {
        let k = Form::Rechteck {
            breite: mm(80.0),
            hoehe: mm(50.0),
            eckenradius: mm(0.0),
        }
        .kontur()
        .unwrap();
        assert_eq!(k.len(), 4);
        assert!(k.ecken().iter().all(|e| *e));
        assert!((k.umfang() - 260.0).abs() < TOLERANZ_MM);
    }

    #[test]
    fn abgerundete_ecken_verkuerzen_den_umfang_um_den_erwarteten_betrag() {
        // Jede abgerundete Ecke spart 2r gerade Strecke und fügt einen
        // Viertelkreis (πr/2) hinzu.
        let r = 5.0;
        let k = Form::Rechteck {
            breite: mm(80.0),
            hoehe: mm(50.0),
            eckenradius: mm(r),
        }
        .kontur()
        .unwrap();
        let erwartet = 260.0 - 8.0 * r + 2.0 * PI * r;
        assert!((k.umfang() - erwartet).abs() < 0.05, "{}", k.umfang());
        assert!(k.ecken().iter().all(|e| !*e), "abgerundet hat keine Knicke");
    }

    #[test]
    fn ein_kreis_mit_100_mm_durchmesser_hat_den_richtigen_umfang() {
        let k = Form::Kreis {
            durchmesser: mm(100.0),
        }
        .kontur()
        .unwrap();
        assert!((k.umfang() - PI * 100.0).abs() < 0.05);
        let (min, max) = k.grenzen();
        assert!((max.x - min.x - 100.0).abs() < 0.01);
    }

    #[test]
    fn ein_zu_grosser_eckenradius_wird_abgewiesen() {
        let f = Form::Rechteck {
            breite: mm(20.0),
            hoehe: mm(10.0),
            eckenradius: mm(6.0),
        };
        assert_eq!(f.kontur(), Err(FormFehler::EckenradiusZuGross));
    }

    #[test]
    fn formen_unter_1_mm_oder_ueber_5_m_werden_abgewiesen() {
        assert_eq!(
            Form::Kreis {
                durchmesser: mm(0.5)
            }
            .kontur(),
            Err(FormFehler::GroesseAusserhalb)
        );
        assert_eq!(
            Form::Ellipse {
                breite: mm(6000.0),
                hoehe: mm(10.0)
            }
            .kontur(),
            Err(FormFehler::GroesseAusserhalb)
        );
    }

    #[test]
    fn ein_pfad_aus_linien_und_einer_kurve_erkennt_nur_echte_knicke() {
        // Unterseite gerade, oben ein glatter Bogen (wie eine Taschenklappe).
        let pfad = Pfad {
            start: Punkt::neu(0.0, 40.0),
            elemente: vec![
                PfadElement::Linie(Punkt::neu(0.0, 20.0)),
                PfadElement::Kurve {
                    k1: Punkt::neu(0.0, -5.0),
                    k2: Punkt::neu(60.0, -5.0),
                    ziel: Punkt::neu(60.0, 20.0),
                },
                PfadElement::Linie(Punkt::neu(60.0, 40.0)),
            ],
        };
        let k = Form::Pfad(pfad).kontur().unwrap();
        let knicke = k.ecken().iter().filter(|e| **e).count();
        // Ecken unten links und unten rechts; die Übergänge Linie → Kurve
        // sind tangential und zählen nicht.
        assert_eq!(knicke, 2);
        assert!(k.ist_ecke(0));
    }

    fn t_pocket(b: f64, h: f64, u: f64, fh: f64) -> Result<Kontur, FormFehler> {
        Form::TPocket {
            breite: mm(b),
            hoehe: mm(h),
            fluegel_ueberstand: mm(u),
            fluegel_hoehe: mm(fh),
        }
        .kontur()
    }

    #[test]
    fn eine_t_pocket_hat_acht_ecken_und_einen_schmaleren_steg() {
        // Springfield-Maß: 101,6 × 44,5 mm, Flügel 12,7 mm, 12 mm hoch.
        let k = t_pocket(101.6, 44.5, 12.7, 12.0).unwrap();
        assert_eq!(k.len(), 8);
        assert!(k.ecken().iter().all(|&e| e));
        let (von, bis) = k.grenzen();
        assert!((bis.x - von.x - 101.6).abs() < 1e-9);
        assert!((bis.y - von.y - 44.5).abs() < 1e-9);
        // Fläche: Querbalken plus Steg darunter.
        let soll = 101.6 * 12.0 + (101.6 - 2.0 * 12.7) * (44.5 - 12.0);
        assert!((k.flaeche_mit_vorzeichen().abs() - soll).abs() < 1e-6);
    }

    #[test]
    fn zu_breite_oder_zu_hohe_fluegel_werden_abgewiesen() {
        // Steg bliebe nur 6 mm breit.
        assert_eq!(
            t_pocket(80.0, 40.0, 37.0, 10.0),
            Err(FormFehler::FluegelZuGross)
        );
        // Flügel so hoch wie das ganze Fach: das wäre ein Rechteck.
        assert_eq!(
            t_pocket(80.0, 40.0, 10.0, 40.0),
            Err(FormFehler::FluegelZuGross)
        );
    }
}
