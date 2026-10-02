//! Stichlöcher entlang einer Linie im Kantenabstand.
//!
//! Regeln, wie sie beim Abstechen mit dem Pricking-Eisen üblich sind:
//! - Auf jeder echten Ecke sitzt genau ein Loch.
//! - Zwischen zwei Ecken werden die Löcher gleichmäßig verteilt. Der
//!   Lochabstand wird dafür minimal angepasst, statt am Ende einen
//!   krummen Rest zu lassen.
//! - Formen ohne Ecken (Kreis, abgerundetes Rechteck) werden ringsum
//!   gleichmäßig verteilt.

use crate::geometrie::{Kontur, Punkt};
use crate::mass::Mm;
use crate::versatz::{self, VersatzFehler};

/// Erlaubter Bereich für den Lochabstand (Zinkenabstand des Eisens).
pub const LOCHABSTAND_MM: std::ops::RangeInclusive<f64> = 1.0..=20.0;
/// Erlaubter Bereich für den Kantenabstand.
pub const KANTENABSTAND_MM: std::ops::RangeInclusive<f64> = 0.5..=50.0;
/// Höchstzahl Löcher pro Stichreihe.
pub const MAX_LOECHER: usize = 100_000;

/// Einstellungen für eine Stichreihe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Einstellung {
    /// Abstand der Stichlinie zur Schnittkante.
    pub kantenabstand: Mm,
    /// Gewünschter Abstand von Loch zu Loch.
    pub lochabstand: Mm,
    /// Wie das Loch auf einer Ecke ausgerichtet wird.
    pub ecke: Eckausrichtung,
}

/// Ausrichtung des Lochs auf einer Ecke.
///
/// Gängige Regel: Alle Löcher einer Seite haben denselben Schrägwinkel zur
/// Naht. Das Eckloch übernimmt deshalb standardmäßig die Ausrichtung der
/// Kante, die in die Ecke hineinläuft, und passt so zu seinen Nachbarn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Eckausrichtung {
    /// Wie die Kante, die in die Ecke hineinläuft.
    #[default]
    Einlaufend,
    /// Wie die Kante, die aus der Ecke hinausläuft.
    Auslaufend,
    /// Winkelhalbierende zwischen beiden Kanten.
    Mitte,
}

/// Ein Stichloch: Mittelpunkt und Richtung der Naht an dieser Stelle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stichloch {
    pub mitte: Punkt,
    /// Einheitsvektor in Nahtrichtung.
    pub richtung: Punkt,
    /// Sitzt das Loch auf einer Ecke?
    pub auf_ecke: bool,
}

/// Ein Abschnitt der Naht zwischen zwei Ecken (oder der ganze Umlauf).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Abschnitt {
    pub laenge: Mm,
    /// Anzahl Abstände in diesem Abschnitt (= Löcher ohne das Endloch).
    pub abstaende: usize,
    /// Tatsächlicher Lochabstand nach dem gleichmäßigen Verteilen.
    pub lochabstand: Mm,
}

/// Ein Stück Stichlinie. Geschlossen läuft es einmal ringsum, offen hat es
/// einen Anfang und ein Ende (Naht nur an einzelnen Kanten).
#[derive(Debug, Clone, PartialEq)]
pub struct Linienzug {
    pub punkte: Vec<Punkt>,
    pub geschlossen: bool,
}

/// Ergebnis: Stichlinie(n), Löcher und die Abschnitte für die Anzeige
/// („Abstand angepasst auf 3,92 mm").
#[derive(Debug, Clone, PartialEq)]
pub struct Stichreihe {
    pub linien: Vec<Linienzug>,
    pub loecher: Vec<Stichloch>,
    pub abschnitte: Vec<Abschnitt>,
}

/// Warum keine Stichreihe entstehen kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StichFehler {
    LochabstandAusserhalb,
    KantenabstandAusserhalb,
    ZuVieleLoecher,
    /// Die Kantenauswahl passt nicht zur Form, oder die Stichlinie hat
    /// durch den Kantenabstand Kanten verloren und lässt sich den
    /// gewählten Kanten nicht mehr zuordnen.
    KantenUnpassend,
    Versatz(VersatzFehler),
}

impl From<VersatzFehler> for StichFehler {
    fn from(f: VersatzFehler) -> Self {
        StichFehler::Versatz(f)
    }
}

fn pruefe(e: Einstellung) -> Result<(), StichFehler> {
    if !LOCHABSTAND_MM.contains(&e.lochabstand.wert()) {
        return Err(StichFehler::LochabstandAusserhalb);
    }
    if !KANTENABSTAND_MM.contains(&e.kantenabstand.wert()) {
        return Err(StichFehler::KantenabstandAusserhalb);
    }
    Ok(())
}

/// Setzt die Stichlöcher ringsum für ein Teil mit der Schnittkante `kontur`.
pub fn stichreihe(kontur: &Kontur, e: Einstellung) -> Result<Stichreihe, StichFehler> {
    pruefe(e)?;
    let linie = versatz::nach_innen(kontur, e.kantenabstand)?;
    let n = linie.len();
    let ecken: Vec<usize> = (0..n).filter(|&i| linie.ist_ecke(i)).collect();
    // Ohne Ecken: ein einziger Abschnitt ringsum, beginnend bei Punkt 0.
    let starts = if ecken.is_empty() { vec![0] } else { ecken };

    let mut r = Stichreihe {
        linien: vec![Linienzug {
            punkte: linie.punkte().to_vec(),
            geschlossen: true,
        }],
        loecher: Vec::new(),
        abschnitte: Vec::new(),
    };
    for (k, &von) in starts.iter().enumerate() {
        let bis = starts[(k + 1) % starts.len()];
        let schritte = if bis > von { bis - von } else { bis + n - von };
        let start = Stichloch {
            mitte: linie.punkt(von),
            richtung: richtung_an_punkt(&linie, von, e.ecke),
            auf_ecke: linie.ist_ecke(von),
        };
        abschnitt_legen(&mut r, |i| linie.punkt(von + i), schritte, start, e)?;
    }
    Ok(r)
}

/// Setzt Stichlöcher nur an den gewählten Kanten. `kanten[i]` steht für die
/// Kante von Punkt `i` zu Punkt `i + 1` der Kontur. Aufeinanderfolgende
/// gewählte Kanten bilden eine durchgehende Naht; jede Naht beginnt und
/// endet mit einem Loch. Sind alle Kanten gewählt, ist das dasselbe wie
/// [`stichreihe`].
pub fn stichreihe_an_kanten(
    kontur: &Kontur,
    kanten: &[bool],
    e: Einstellung,
) -> Result<Stichreihe, StichFehler> {
    pruefe(e)?;
    let n = kontur.len();
    if kanten.len() != n {
        return Err(StichFehler::KantenUnpassend);
    }
    if kanten.iter().all(|&k| k) {
        return stichreihe(kontur, e);
    }
    let linie = versatz::nach_innen(kontur, e.kantenabstand)?;
    // Ohne gleiche Punktzahl lässt sich Kante i nicht mehr Kante i der
    // Stichlinie zuordnen.
    if linie.len() != n {
        return Err(StichFehler::KantenUnpassend);
    }

    let mut r = Stichreihe {
        linien: Vec::new(),
        loecher: Vec::new(),
        abschnitte: Vec::new(),
    };
    // Jede Naht beginnt an einer gewählten Kante, deren Vorgängerin nicht
    // gewählt ist.
    for start in (0..n).filter(|&i| kanten[i] && !kanten[(i + n - 1) % n]) {
        let laenge = (0..n).take_while(|&k| kanten[(start + k) % n]).count();
        let punkte: Vec<Punkt> = (0..=laenge).map(|k| linie.punkt(start + k)).collect();
        // Innerhalb der Naht an den Ecken teilen, wie ringsum.
        let mut teil_start = 0;
        for k in 1..=laenge {
            if k < laenge && !linie.ist_ecke(start + k) {
                continue;
            }
            let von = teil_start;
            let richtung = if von == 0 {
                punkte[1]
                    .minus(punkte[0])
                    .normiert()
                    .unwrap_or(Punkt::neu(1.0, 0.0))
            } else {
                richtung_an_punkt(&linie, start + von, e.ecke)
            };
            let anfang = Stichloch {
                mitte: punkte[von],
                richtung,
                auf_ecke: von > 0 || linie.ist_ecke(start),
            };
            abschnitt_legen(&mut r, |i| punkte[von + i], k - von, anfang, e)?;
            teil_start = k;
        }
        // Endloch der offenen Naht.
        let ende = punkte[laenge];
        r.loecher.push(Stichloch {
            mitte: ende,
            richtung: ende
                .minus(punkte[laenge - 1])
                .normiert()
                .unwrap_or(Punkt::neu(1.0, 0.0)),
            auf_ecke: linie.ist_ecke(start + laenge),
        });
        r.linien.push(Linienzug {
            punkte,
            geschlossen: false,
        });
    }
    Ok(r)
}

/// Verteilt die Löcher auf einem Abschnitt mit `schritte` Kanten, dessen
/// Punkte `punkt(0..=schritte)` liefert. Setzt das Anfangsloch und alle
/// Zwischenlöcher; das Endloch gehört zum nächsten Abschnitt.
fn abschnitt_legen(
    r: &mut Stichreihe,
    punkt: impl Fn(usize) -> Punkt,
    schritte: usize,
    anfang: Stichloch,
    e: Einstellung,
) -> Result<(), StichFehler> {
    let p = e.lochabstand.wert();
    let laenge: f64 = (0..schritte).map(|i| punkt(i).abstand(punkt(i + 1))).sum();
    let abstaende = ((laenge / p).round() as usize).max(1);
    if r.loecher.len() + abstaende > MAX_LOECHER {
        return Err(StichFehler::ZuVieleLoecher);
    }
    let ist = laenge / abstaende as f64;
    r.abschnitte.push(Abschnitt {
        laenge: Mm::neu(laenge).map_err(|_| StichFehler::ZuVieleLoecher)?,
        abstaende,
        lochabstand: Mm::neu(ist).map_err(|_| StichFehler::ZuVieleLoecher)?,
    });

    // Entlang des Abschnitts laufen und bei jedem Vielfachen von `ist`
    // ein Loch setzen.
    r.loecher.push(anfang);
    let mut gelaufen = 0.0;
    let mut naechstes = 1;
    for i in 0..schritte {
        let a = punkt(i);
        let b = punkt(i + 1);
        let kante = a.abstand(b);
        while naechstes < abstaende && (naechstes as f64) * ist <= gelaufen + kante {
            let t = ((naechstes as f64) * ist - gelaufen) / kante;
            let richtung = b.minus(a).normiert().unwrap_or(Punkt::neu(1.0, 0.0));
            r.loecher.push(Stichloch {
                mitte: a.plus(b.minus(a).mal(t)),
                richtung,
                auf_ecke: false,
            });
            naechstes += 1;
        }
        gelaufen += kante;
    }
    Ok(())
}

/// Nahtrichtung an einem Punkt der Linie. Auf einer Ecke entscheidet die
/// [`Eckausrichtung`]; auf glatten Stellen ist es das Mittel aus ein- und
/// auslaufender Kante.
fn richtung_an_punkt(linie: &Kontur, i: usize, ecke: Eckausrichtung) -> Punkt {
    let n = linie.len();
    let p = linie.punkt(i);
    let ein = p.minus(linie.punkt(i + n - 1)).normiert();
    let aus = linie.punkt(i + 1).minus(p).normiert();
    let ausrichtung = if linie.ist_ecke(i) {
        ecke
    } else {
        Eckausrichtung::Mitte
    };
    match (ein, aus) {
        (Some(a), Some(_)) if ausrichtung == Eckausrichtung::Einlaufend => a,
        (Some(_), Some(b)) if ausrichtung == Eckausrichtung::Auslaufend => b,
        (Some(a), Some(b)) => a.plus(b).normiert().unwrap_or(b),
        (_, Some(b)) => b,
        (Some(a), _) => a,
        _ => Punkt::neu(1.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::Form;
    use crate::mass::TOLERANZ_MM;

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    fn einstellung(kante: f64, loch: f64) -> Einstellung {
        Einstellung {
            kantenabstand: mm(kante),
            lochabstand: mm(loch),
            ecke: Eckausrichtung::default(),
        }
    }

    fn rechteck(b: f64, h: f64, r: f64) -> Kontur {
        Form::Rechteck {
            breite: mm(b),
            hoehe: mm(h),
            eckenradius: mm(r),
        }
        .kontur()
        .unwrap()
    }

    #[test]
    fn auf_jeder_ecke_eines_rechtecks_sitzt_ein_loch() {
        let r = stichreihe(&rechteck(80.0, 50.0, 0.0), einstellung(3.0, 4.0)).unwrap();
        let ecken: Vec<Punkt> = r
            .loecher
            .iter()
            .filter(|l| l.auf_ecke)
            .map(|l| l.mitte)
            .collect();
        assert_eq!(ecken.len(), 4);
        for soll in [(3.0, 3.0), (77.0, 3.0), (77.0, 47.0), (3.0, 47.0)] {
            let soll = Punkt::neu(soll.0, soll.1);
            assert!(
                ecken.iter().any(|p| p.abstand(soll) < TOLERANZ_MM),
                "{soll:?}"
            );
        }
    }

    #[test]
    fn passt_der_abstand_genau_bleibt_er_unveraendert() {
        // Stichlinie 74 × 44 mm; 4 mm passt nicht genau, 2 mm schon.
        let r = stichreihe(&rechteck(80.0, 50.0, 0.0), einstellung(3.0, 2.0)).unwrap();
        for a in &r.abschnitte {
            assert!(a.lochabstand.etwa_gleich(mm(2.0)));
        }
        // 2 × (37 + 22) Abstände = 118 Löcher.
        assert_eq!(r.loecher.len(), 118);
    }

    #[test]
    fn passt_er_nicht_genau_wird_er_gleichmaessig_verteilt() {
        let r = stichreihe(&rechteck(80.0, 50.0, 0.0), einstellung(3.0, 3.85)).unwrap();
        // Lange Seite 74 mm: 74 / 3,85 = 19,2 → 19 Abstände à 3,895 mm.
        let lang = r.abschnitte[0];
        assert_eq!(lang.abstaende, 19);
        assert!(lang.lochabstand.etwa_gleich(mm(74.0 / 19.0)));
        // Nachbarlöcher liegen auf dem ganzen Abschnitt gleich weit auseinander.
        for paar in r.loecher[..20].windows(2) {
            let d = paar[0].mitte.abstand(paar[1].mitte);
            assert!((d - 74.0 / 19.0).abs() < TOLERANZ_MM, "{d}");
        }
    }

    #[test]
    fn alle_loecher_liegen_genau_auf_der_stichlinie_im_kantenabstand() {
        let r = stichreihe(&rechteck(60.0, 40.0, 0.0), einstellung(4.0, 3.0)).unwrap();
        for l in &r.loecher {
            let x_kante = l.mitte.x.min(60.0 - l.mitte.x);
            let y_kante = l.mitte.y.min(40.0 - l.mitte.y);
            assert!((x_kante.min(y_kante) - 4.0).abs() < TOLERANZ_MM);
        }
    }

    #[test]
    fn ein_kreis_wird_ringsum_gleichmaessig_gelocht() {
        let k = Form::Kreis {
            durchmesser: mm(50.0),
        }
        .kontur()
        .unwrap();
        let r = stichreihe(&k, einstellung(3.0, 4.0)).unwrap();
        assert_eq!(r.abschnitte.len(), 1);
        assert!(r.loecher.iter().all(|l| !l.auf_ecke));
        let n = r.loecher.len();
        // Umfang der Stichlinie ≈ π · 44 = 138,2 mm → 35 Löcher.
        assert_eq!(n, 35);
        for i in 0..n {
            let d = r.loecher[i].mitte.abstand(r.loecher[(i + 1) % n].mitte);
            assert!((d - 3.95).abs() < 0.05, "{d}");
        }
    }

    #[test]
    fn die_nahtrichtung_folgt_der_kante() {
        let r = stichreihe(&rechteck(80.0, 50.0, 0.0), einstellung(3.0, 4.0)).unwrap();
        let oben = r
            .loecher
            .iter()
            .find(|l| !l.auf_ecke && (l.mitte.y - 3.0).abs() < 1e-6);
        let richtung = oben.unwrap().richtung;
        assert!(richtung.x.abs() > 0.999 && richtung.y.abs() < 1e-6);
    }

    #[test]
    fn das_eckloch_hat_standardmaessig_die_richtung_der_einlaufenden_kante() {
        let k = rechteck(80.0, 50.0, 0.0);
        let ecklochrichtung = |ecke| {
            let e = Einstellung {
                ecke,
                ..einstellung(3.0, 4.0)
            };
            let r = stichreihe(&k, e).unwrap();
            // Erstes Loch ist die Ecke oben links (3, 3).
            assert!(r.loecher[0].auf_ecke);
            assert!(r.loecher[0].mitte.abstand(Punkt::neu(3.0, 3.0)) < TOLERANZ_MM);
            r.loecher[0].richtung
        };
        let ein = ecklochrichtung(Eckausrichtung::Einlaufend);
        let aus = ecklochrichtung(Eckausrichtung::Auslaufend);
        let mitte = ecklochrichtung(Eckausrichtung::Mitte);
        // Die Naht kommt von unten (linke Seite) und läuft nach rechts weiter.
        assert!(ein.abstand(Punkt::neu(0.0, -1.0)) < 1e-9, "{ein:?}");
        assert!(aus.abstand(Punkt::neu(1.0, 0.0)) < 1e-9, "{aus:?}");
        let h = std::f64::consts::FRAC_1_SQRT_2;
        assert!(mitte.abstand(Punkt::neu(h, -h)) < 1e-9, "{mitte:?}");
    }

    #[test]
    fn unsinnige_einstellungen_werden_abgewiesen() {
        let k = rechteck(80.0, 50.0, 0.0);
        assert_eq!(
            stichreihe(&k, einstellung(3.0, 0.2)),
            Err(StichFehler::LochabstandAusserhalb)
        );
        assert_eq!(
            stichreihe(&k, einstellung(0.0, 4.0)),
            Err(StichFehler::KantenabstandAusserhalb)
        );
        assert_eq!(
            stichreihe(&k, einstellung(30.0, 4.0)),
            Err(StichFehler::Versatz(VersatzFehler::AbstandZuGross))
        );
    }

    #[test]
    fn viele_kombinationen_ergeben_nie_loecher_ausserhalb_des_teils() {
        // Ersatz für einen Eigenschaftstest: breites Raster an Formen und
        // Einstellungen, ohne zusätzliche Abhängigkeit.
        for (b, h, r) in [(30.0, 20.0, 0.0), (120.0, 35.0, 6.0), (200.0, 90.0, 20.0)] {
            let k = rechteck(b, h, r);
            for kante in [1.0, 2.5, 3.0, 4.0, 6.0] {
                for loch in [2.0, 2.7, 3.0, 3.38, 3.85, 4.0, 5.0, 6.0] {
                    let s = stichreihe(&k, einstellung(kante, loch)).unwrap();
                    for l in &s.loecher {
                        assert!(
                            l.mitte.x >= kante - TOLERANZ_MM
                                && l.mitte.x <= b - kante + TOLERANZ_MM
                        );
                        assert!(
                            l.mitte.y >= kante - TOLERANZ_MM
                                && l.mitte.y <= h - kante + TOLERANZ_MM
                        );
                    }
                    for a in &s.abschnitte {
                        // Nie mehr als die Hälfte eines Abstands daneben.
                        assert!(
                            (a.lochabstand.wert() - loch).abs() <= loch / 2.0 + TOLERANZ_MM
                                || a.abstaende == 1
                        );
                    }
                }
            }
        }
    }

    fn t_pocket() -> Kontur {
        Form::TPocket {
            breite: mm(101.6),
            hoehe: mm(44.5),
            fluegel_ueberstand: mm(12.7),
            fluegel_hoehe: mm(12.0),
        }
        .kontur()
        .unwrap()
    }

    #[test]
    fn eine_t_pocket_wird_an_fluegeln_und_steg_genaeht_aber_nicht_an_der_oeffnung() {
        let kanten = crate::form::TPocketNaehte::default().kanten();
        let r = stichreihe_an_kanten(&t_pocket(), &kanten, einstellung(3.0, 3.0)).unwrap();
        // Drei getrennte Nähte: Flügel rechts, Steg unten, Flügel links.
        assert_eq!(r.linien.len(), 3);
        assert!(r.linien.iter().all(|l| !l.geschlossen));
        for l in &r.loecher {
            let p = l.mitte;
            let am_fluegel = ((p.x - 3.0).abs() < TOLERANZ_MM || (p.x - 98.6).abs() < TOLERANZ_MM)
                && p.y >= 3.0 - TOLERANZ_MM
                && p.y <= 9.0 + TOLERANZ_MM;
            let am_steg = (p.y - 41.5).abs() < TOLERANZ_MM
                && p.x >= 15.7 - TOLERANZ_MM
                && p.x <= 85.9 + TOLERANZ_MM;
            assert!(am_fluegel || am_steg, "Loch an falscher Stelle: {p:?}");
        }
    }

    #[test]
    fn jede_offene_naht_beginnt_und_endet_mit_einem_loch() {
        let kanten = crate::form::TPocketNaehte::default().kanten();
        let r = stichreihe_an_kanten(&t_pocket(), &kanten, einstellung(3.0, 2.7)).unwrap();
        for l in &r.linien {
            for ende in [l.punkte[0], l.punkte[l.punkte.len() - 1]] {
                assert!(
                    r.loecher
                        .iter()
                        .any(|h| h.mitte.abstand(ende) < TOLERANZ_MM),
                    "kein Loch am Nahtende {ende:?}"
                );
            }
        }
        // Steg unten: 70,2 mm / 2,7 = 26 Abstände → 27 Löcher; Flügel je
        // 6 mm / 2,7 → 2 Abstände → 3 Löcher.
        assert_eq!(r.loecher.len(), 27 + 3 + 3);
    }

    #[test]
    fn naht_an_allen_kanten_ist_dieselbe_wie_ringsum() {
        let k = rechteck(80.0, 50.0, 0.0);
        let e = einstellung(3.0, 3.85);
        assert_eq!(stichreihe_an_kanten(&k, &[true; 4], e), stichreihe(&k, e));
    }

    #[test]
    fn eine_kantenwahl_die_nicht_zur_form_passt_wird_abgewiesen() {
        let k = rechteck(80.0, 50.0, 0.0);
        assert_eq!(
            stichreihe_an_kanten(&k, &[true, false], einstellung(3.0, 3.0)),
            Err(StichFehler::KantenUnpassend)
        );
    }
}
