//! Uhrarmband als Generator: aus Stegbreite, Armumfang und Hornabstand
//! entsteht ein zweilagiges Standardband (Ober- und Futterleder) mit
//! Langteil, Kurzteil und Schlaufe.
//!
//! Aufbau: Beide Bandteile hängen am Federsteg der Uhr; die Dornschließe
//! sitzt an einem eigenen Federsteg am Ende des Kurzteils. Der Federsteg
//! läuft zwischen Ober- und Futterleder, deshalb sind die Stirnseiten
//! offen und nur die Längsseiten (und die Spitze) genäht.
//!
//! Längen nach der üblichen Faustregel: Um den Arm liegen Kurzteil, das
//! Langteil bis zum mittleren Loch und die Uhr selbst (Hornabstand). Weil
//! das Band nicht auf der Haut, sondern um sie herum liegt und die
//! Schließe Platz braucht, kommt eine Zugabe dazu. Mit 22 mm Steg, 185 mm
//! Armumfang und 48 mm Hornabstand ergibt das 75/120 mm, das gängige
//! Standardmaß.

use super::{Vorlage, VorlagenFehler, bestimmt, mm, p};
use crate::ausschnitt::Ausschnitt;
use crate::eisen::Eisenart;
use crate::form::{Form, Pfad, PfadElement};
use crate::geometrie::Punkt;
use crate::mass::Mm;
use crate::muster::{Montage, Musterteil, Naht, Nahtkanten, Teil, TeilForm};
use crate::skizze::Bedingung;
use crate::stichlinie::{Eckausrichtung, Einstellung};

/// Sinnvolle Bereiche der Eingaben (mm). Außerhalb gibt es kein
/// Uhrarmband, das man tragen könnte; die Grenzen schützen auch den Kern
/// vor unsinnigen Werten (Playbook 01).
pub const STEGBREITE_MM: (f64, f64) = (8.0, 36.0);
pub const ARMUMFANG_MM: (f64, f64) = (110.0, 280.0);
pub const HORNABSTAND_MM: (f64, f64) = (20.0, 70.0);

/// Welche Eingabe nicht passt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UhrarmbandFehler {
    Stegbreite,
    Armumfang,
    Hornabstand,
}

/// Eingaben und Werkstattmaße des Uhrarmbands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UhrarmbandMasse {
    /// Abstand zwischen den Hörnern der Uhr, gleich Bandbreite am Federsteg.
    pub stegbreite: Mm,
    /// Umfang des Handgelenks, gemessen direkt auf der Haut.
    pub armumfang: Mm,
    /// Länge der Uhr von Hornspitze zu Hornspitze (Lug-to-Lug).
    pub hornabstand: Mm,
    /// Zugabe für Banddicke und Schließe, siehe Moduldoku.
    pub zugabe: Mm,
    /// Um so viel wird das Band zur Schließe hin schmaler.
    pub verjuengung: Mm,
    /// Anteil des Kurzteils an dem Teil des Umfangs, den die Uhr nicht
    /// bedeckt; so sitzt die Schließe seitlich am Arm.
    pub anteil_kurzteil: f64,
    /// Anzahl der Dornlöcher; das mittlere passt zum Armumfang.
    pub dornloecher: usize,
    /// Abstand der Dornlöcher von Mitte zu Mitte.
    pub dornloch_abstand: Mm,
    pub dornloch_durchmesser: Mm,
    /// Länge vom letzten Dornloch bis zur Spitze.
    pub spitze: Mm,
    /// Breite der Schlaufe (das Lederband, nicht der Umfang).
    pub schlaufenbreite: Mm,
    /// Dicke des fertigen Bands (Ober- und Futterleder).
    pub banddicke: Mm,
    pub kantenabstand: Mm,
    pub lochabstand: Mm,
}

impl UhrarmbandMasse {
    /// Standardband für die drei Eingaben, alle anderen Maße üblich.
    pub fn neu(stegbreite: Mm, armumfang: Mm, hornabstand: Mm) -> Self {
        UhrarmbandMasse {
            stegbreite,
            armumfang,
            hornabstand,
            zugabe: mm(22.0),
            // 22 → 20 mm, 18 → 16 mm: die übliche Verjüngung.
            verjuengung: mm(2.0),
            anteil_kurzteil: 0.55,
            dornloecher: 7,
            dornloch_abstand: mm(7.0),
            dornloch_durchmesser: mm(3.0),
            spitze: mm(15.0),
            schlaufenbreite: mm(6.0),
            banddicke: mm(3.5),
            kantenabstand: mm(3.0),
            lochabstand: mm(3.0),
        }
    }

    fn pruefen(&self) -> Result<(), UhrarmbandFehler> {
        let im = |x: Mm, (von, bis): (f64, f64)| (von..=bis).contains(&x.wert());
        if !im(self.stegbreite, STEGBREITE_MM) {
            return Err(UhrarmbandFehler::Stegbreite);
        }
        if !im(self.armumfang, ARMUMFANG_MM) {
            return Err(UhrarmbandFehler::Armumfang);
        }
        if !im(self.hornabstand, HORNABSTAND_MM) {
            return Err(UhrarmbandFehler::Hornabstand);
        }
        // Die Uhr darf nicht den halben Arm bedecken, sonst bleibt für die
        // Bandteile nichts übrig.
        if self.hornabstand.wert() > self.armumfang.wert() / 2.0 {
            return Err(UhrarmbandFehler::Hornabstand);
        }
        Ok(())
    }
}

/// Längen, die aus den Eingaben folgen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UhrarmbandAbgeleitet {
    /// Breite an der Schließe.
    pub schliessenbreite: Mm,
    /// Kurzteil vom Federsteg der Uhr bis zum Federsteg der Schließe.
    pub kurzteil: Mm,
    /// Langteil vom Federsteg bis zur Spitze.
    pub langteil: Mm,
    /// Vom Federsteg bis zur Mitte des mittleren Dornlochs.
    pub mittleres_loch: Mm,
    /// Gesamtlänge der Schlaufe (wird zum Ring geschlossen).
    pub schlaufe: Mm,
}

pub fn uhrarmband_abgeleitet(m: &UhrarmbandMasse) -> Result<UhrarmbandAbgeleitet, VorlagenFehler> {
    m.pruefen().map_err(VorlagenFehler::Uhrarmband)?;
    let schliessenbreite = m.stegbreite.wert() - m.verjuengung.wert();
    let frei = m.armumfang.wert() - m.hornabstand.wert();
    // Auf ganze Millimeter, wie man es anzeichnet.
    let kurzteil = (m.anteil_kurzteil * frei).round();
    let mittleres_loch = (frei + m.zugabe.wert() - kurzteil).round();
    let halbe = (m.dornloecher.saturating_sub(1) / 2) as f64;
    let langteil = mittleres_loch + halbe * m.dornloch_abstand.wert() + m.spitze.wert();
    // Die Schlaufe umschließt beide Bandenden übereinander.
    let schlaufe = 2.0 * schliessenbreite + 4.0 * m.banddicke.wert() + 2.0;
    Ok(UhrarmbandAbgeleitet {
        schliessenbreite: mm(schliessenbreite),
        kurzteil: mm(kurzteil),
        langteil: mm(langteil),
        mittleres_loch: mm(mittleres_loch),
        schlaufe: mm(schlaufe),
    })
}

fn naht(m: &UhrarmbandMasse, kanten: Vec<bool>) -> Naht {
    Naht {
        einstellung: Einstellung {
            kantenabstand: m.kantenabstand,
            lochabstand: m.lochabstand,
            ecke: Eckausrichtung::Einlaufend,
        },
        eisen: Eisenart::Diamant,
        kanten: Nahtkanten::Pfad(kanten),
    }
}

/// Kurzteil: Trapez von der Schließe (oben, Schließenbreite) zum
/// Federsteg der Uhr (unten, Stegbreite). So herum liegt es auch am Arm,
/// wenn die Uhr unter ihm sitzt.
fn kurzteil_form(w: f64, b: f64, l: f64) -> Result<(Pfad, Vec<Bedingung>), VorlagenFehler> {
    let rand = (w - b) / 2.0;
    let skizze = Pfad {
        start: p(rand, 0.0),
        elemente: vec![
            PfadElement::Linie(p(rand + b + 0.4, 0.3)),
            PfadElement::Linie(p(w + 0.6, l - 0.5)),
            PfadElement::Linie(p(-0.4, l + 0.4)),
        ],
    };
    // Punkte: 0 oben links, 1 oben rechts, 2 unten rechts, 3 unten links;
    // Kante 0 Schließe, Kante 2 Federsteg, Kante 3 die Schlusslinie.
    let bed = vec![
        Bedingung::Horizontal { kante: 0 },
        Bedingung::Laenge {
            kante: 0,
            wert: mm(b),
        },
        Bedingung::Horizontal { kante: 2 },
        Bedingung::Laenge {
            kante: 2,
            wert: mm(w),
        },
        Bedingung::Gleich { a: 1, b: 3 },
        Bedingung::AbstandY {
            a: 1,
            b: 2,
            wert: mm(l),
        },
    ];
    let pfad = bestimmt(&skizze, true, &bed)?;
    Ok((pfad, bed))
}

/// Langteil: vom Federsteg zur Spitze schmaler werdend, die Spitze ein
/// Halbkreis, in den die Seiten ohne Knick übergehen. `l` ist die
/// Gesamtlänge bis zum Scheitel der Spitze.
fn langteil_form(w: f64, b: f64, l: f64) -> Result<(Pfad, Vec<Bedingung>), VorlagenFehler> {
    let r = b / 2.0;
    // Wo die rechte Seite in den Spitzenbogen übergeht: Tangente von der
    // Ecke am Federsteg an den Kreis der Spitze.
    let ecke = p(w, 0.0);
    let mitte = p(w / 2.0, l - r);
    let v = mitte.minus(ecke);
    let d = v.laenge();
    let t = (d * d - r * r).sqrt();
    // Drehung von der Richtung zur Kreismitte nach außen (zur rechten Seite).
    let winkel = (r / d).asin();
    let richtung = v.mal(1.0 / d).gedreht(-winkel);
    let beruehrung = ecke.plus(richtung.mal(t));
    let skizze = Pfad {
        start: p(0.0, 0.0),
        elemente: vec![
            PfadElement::Linie(p(w + 0.3, 0.4)),
            PfadElement::Linie(p(beruehrung.x + 0.5, beruehrung.y - 0.6)),
            PfadElement::Bogen {
                ziel: p(w - beruehrung.x - 0.4, beruehrung.y + 0.3),
                radius: mm(r + 1.0),
                uhrzeigersinn: true,
                gross: false,
            },
        ],
    };
    // Punkte: 0 oben links, 1 oben rechts, 2 Übergang rechts, 3 Übergang
    // links; Kante 2 ist der Bogen, Kante 3 die linke Seite (Schlusslinie).
    let bed = vec![
        Bedingung::Horizontal { kante: 0 },
        Bedingung::Laenge {
            kante: 0,
            wert: mm(w),
        },
        Bedingung::Tangential { a: 1, b: 2 },
        Bedingung::Tangential { a: 2, b: 3 },
        Bedingung::Radius {
            kante: 2,
            wert: mm(r),
        },
        Bedingung::Gleich { a: 1, b: 3 },
        Bedingung::AbstandY {
            a: 1,
            b: 2,
            wert: mm(beruehrung.y),
        },
    ];
    let pfad = bestimmt(&skizze, true, &bed)?;
    Ok((pfad, bed))
}

/// Dornlöcher auf der Mittellinie des Langteils, das mittlere bei
/// `mitte_y`.
fn dornloecher(m: &UhrarmbandMasse, a: &UhrarmbandAbgeleitet) -> Vec<Ausschnitt> {
    let n = m.dornloecher;
    let r = m.dornloch_durchmesser.wert() / 2.0;
    let x = m.stegbreite.wert() / 2.0;
    let mitte = (n.saturating_sub(1) / 2) as f64;
    (0..n)
        .map(|i| {
            let y = a.mittleres_loch.wert() + (i as f64 - mitte) * m.dornloch_abstand.wert();
            Ausschnitt {
                form: Form::Kreis {
                    durchmesser: m.dornloch_durchmesser,
                },
                // Kreise liegen mit der linken oberen Ecke am Bezugspunkt.
                position: p(x - r, y - r),
            }
        })
        .collect()
}

/// Uhrarmband aus drei Eingaben, alle anderen Maße Standard.
pub fn uhrarmband(
    stegbreite: Mm,
    armumfang: Mm,
    hornabstand: Mm,
) -> Result<Vorlage, VorlagenFehler> {
    uhrarmband_mit(&UhrarmbandMasse::neu(stegbreite, armumfang, hornabstand))
}

pub fn uhrarmband_mit(m: &UhrarmbandMasse) -> Result<Vorlage, VorlagenFehler> {
    let a = uhrarmband_abgeleitet(m)?;
    let (w, b) = (m.stegbreite.wert(), a.schliessenbreite.wert());
    let (lang_pfad, lang_bed) = langteil_form(w, b, a.langteil.wert())?;
    let (kurz_pfad, kurz_bed) = kurzteil_form(w, b, a.kurzteil.wert())?;
    // Langteil: genäht an beiden Seiten und um die Spitze, offen am
    // Federsteg. Kurzteil: nur die Seiten, beide Stirnseiten offen.
    let lang_naht = vec![false, true, true, true];
    let kurz_naht = vec![false, true, false, true];
    let loecher = dornloecher(m, &a);

    // Im Muster nebeneinander, im Zusammenbau Langteil und Kurzteil wie
    // an der Uhr: das Kurzteil oben, die Uhr dazwischen, das Langteil unten.
    let zwischenraum = 30.0;
    let x_kurz = w + zwischenraum;
    let x_schlaufe = 2.0 * (w + zwischenraum);
    let uhr = m.hornabstand.wert();
    let montage_kurz = p(0.0, 0.0);
    let montage_lang = p(0.0, a.kurzteil.wert() + uhr);

    let teil = |name: &str,
                pfad: &Pfad,
                bed: &[Bedingung],
                position: Punkt,
                montage: Montage,
                naht_kanten: Option<Vec<bool>>,
                ausschnitte: Vec<Ausschnitt>| Musterteil {
        name: name.into(),
        teil: Teil {
            form: TeilForm::Geschlossen(Form::Pfad(pfad.clone())),
            position,
            naht: naht_kanten.map(|k| naht(m, k)),
            montage: Some(montage),
            ausschnitte,
        },
        bedingungen: bed.to_vec(),
        abstandsmasse: Vec::new(),
    };

    // Das Oberleder trägt die Naht, das Futter bekommt dieselben Löcher
    // beim Durchstechen im Zusammenbau.
    let unten = a.langteil.wert() + zwischenraum;
    let teile = vec![
        teil(
            "Langteil Oberleder",
            &lang_pfad,
            &lang_bed,
            p(0.0, 0.0),
            Montage {
                position: montage_lang,
                schritt: 1,
            },
            Some(lang_naht),
            loecher.clone(),
        ),
        teil(
            "Langteil Futter",
            &lang_pfad,
            &lang_bed,
            p(0.0, unten),
            Montage {
                position: montage_lang,
                schritt: 0,
            },
            None,
            loecher,
        ),
        teil(
            "Kurzteil Oberleder",
            &kurz_pfad,
            &kurz_bed,
            p(x_kurz, 0.0),
            Montage {
                position: montage_kurz,
                schritt: 1,
            },
            Some(kurz_naht),
            Vec::new(),
        ),
        teil(
            "Kurzteil Futter",
            &kurz_pfad,
            &kurz_bed,
            p(x_kurz, unten),
            Montage {
                position: montage_kurz,
                schritt: 0,
            },
            None,
            Vec::new(),
        ),
        Musterteil {
            name: "Schlaufe".into(),
            teil: Teil {
                form: TeilForm::Geschlossen(Form::Rechteck {
                    breite: a.schlaufe,
                    hoehe: m.schlaufenbreite,
                    eckenradius: Mm::NULL,
                }),
                position: p(x_schlaufe, 0.0),
                naht: None,
                // Liegt im Zusammenbau quer über dem Kurzteil, kurz hinter
                // der Schließe, wo sie das Ende des Langteils hält.
                montage: Some(Montage {
                    position: p(w / 2.0 - a.schlaufe.wert() / 2.0, 12.0),
                    schritt: 2,
                }),
                ausschnitte: Vec::new(),
            },
            bedingungen: Vec::new(),
            abstandsmasse: Vec::new(),
        },
    ];

    let schritte = vec![
        "Alle Teile ausschneiden, Dornlöcher in Ober- und Futterleder des \
         Langteils stanzen. Futter an den Kanten etwas dünner schärfen, wenn \
         das Band zu dick wird."
            .into(),
        "Schritt 1: Oberleder und Futter jeweils links auf links verkleben, \
         an den Federstegen (Uhr und Schließe) aber die letzten 10 mm \
         unverklebt lassen: Dort schiebt man den Federsteg zwischen die \
         Lagen. Dann die Seiten nähen, beim Langteil um die Spitze herum."
            .into(),
        format!(
            "Schritt 2: Schlaufe ({} mm lang) um beide Bandenden legen, auf \
             Stoß nähen oder überlappend kleben, und auf das Kurzteil schieben.",
            a.schlaufe.wert()
        ),
        "Schritt 3: Kanten schleifen, färben und polieren. Schließe mit \
         Federsteg am Kurzteil einsetzen, beide Teile an der Uhr einhängen."
            .into(),
    ];
    Ok(Vorlage { teile, schritte })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::Form;
    use crate::mass::TOLERANZ_MM;
    use crate::muster::{self, Umriss};
    use crate::skizze;

    fn masse(steg: f64, arm: f64, horn: f64) -> UhrarmbandMasse {
        UhrarmbandMasse::neu(mm(steg), mm(arm), mm(horn))
    }

    fn standard() -> Vorlage {
        uhrarmband(mm(22.0), mm(185.0), mm(48.0)).unwrap()
    }

    fn teil<'a>(v: &'a Vorlage, name: &str) -> &'a Musterteil {
        v.teile.iter().find(|t| t.name == name).unwrap()
    }

    fn grenzen(t: &Musterteil) -> (Punkt, Punkt) {
        let Umriss::Teil { kontur, .. } = t.teil.umriss().unwrap() else {
            panic!("Teil erwartet");
        };
        kontur.grenzen()
    }

    #[test]
    fn standardband_22_mm_bei_185_mm_armumfang_ist_75_zu_120() {
        let a = uhrarmband_abgeleitet(&masse(22.0, 185.0, 48.0)).unwrap();
        assert!(a.kurzteil.etwa_gleich(mm(75.0)), "{}", a.kurzteil);
        assert!(a.langteil.etwa_gleich(mm(120.0)), "{}", a.langteil);
        assert!(a.schliessenbreite.etwa_gleich(mm(20.0)));
    }

    #[test]
    fn das_mittlere_dornloch_schliesst_das_band_beim_armumfang_plus_zugabe() {
        for (steg, arm, horn) in [
            (22.0, 185.0, 48.0),
            (18.0, 160.0, 40.0),
            (24.0, 210.0, 52.0),
        ] {
            let m = masse(steg, arm, horn);
            let a = uhrarmband_abgeleitet(&m).unwrap();
            let rund = a.kurzteil.wert() + a.mittleres_loch.wert() + horn;
            // Auf ganze Millimeter gerundet: höchstens 1 mm daneben.
            assert!((rund - arm - m.zugabe.wert()).abs() <= 1.0, "{rund}");
        }
    }

    #[test]
    fn das_band_ist_am_federsteg_so_breit_wie_der_steg_und_wird_zur_schliesse_schmaler() {
        let v = standard();
        for name in ["Langteil Oberleder", "Kurzteil Oberleder"] {
            let (von, bis) = grenzen(teil(&v, name));
            assert!((bis.x - von.x - 22.0).abs() < TOLERANZ_MM, "{name}");
        }
        // Länge exakt aus dem Spitzenbogen: tiefster Punkt = Mitte + Radius.
        // (Die Kontur nähert den Bogen mit Strecken an und liegt knapp
        // darüber.)
        let TeilForm::Geschlossen(Form::Pfad(lang)) = &teil(&v, "Langteil Oberleder").teil.form
        else {
            panic!("Pfad erwartet");
        };
        let (
            PfadElement::Linie(rechts),
            PfadElement::Bogen {
                ziel: links,
                radius,
                ..
            },
        ) = (lang.elemente[1], lang.elemente[2])
        else {
            panic!("Linie und Bogen erwartet");
        };
        let halbe_sehne = rechts.abstand(links) / 2.0;
        let r = radius.wert();
        let mitte_y = (rechts.y + links.y) / 2.0 - (r * r - halbe_sehne * halbe_sehne).sqrt();
        assert!((mitte_y + r - 120.0).abs() < TOLERANZ_MM, "{}", mitte_y + r);
        let (von, bis) = grenzen(teil(&v, "Kurzteil Oberleder"));
        assert!((bis.y - von.y - 75.0).abs() < TOLERANZ_MM);
        // Das Kurzteil ist an der Schließe (oben) 20 mm und am Federsteg
        // der Uhr (unten) 22 mm breit.
        let TeilForm::Geschlossen(Form::Pfad(pfad)) = &teil(&v, "Kurzteil Oberleder").teil.form
        else {
            panic!("Pfad erwartet");
        };
        let ecken: Vec<Punkt> = std::iter::once(pfad.start)
            .chain(pfad.elemente.iter().map(|e| e.ziel()))
            .collect();
        assert!((ecken[0].abstand(ecken[1]) - 20.0).abs() < TOLERANZ_MM);
        assert!((ecken[2].abstand(ecken[3]) - 22.0).abs() < TOLERANZ_MM);
        assert!(ecken[2].y > ecken[1].y);
    }

    #[test]
    fn die_spitze_ist_ein_halbkreis_ohne_knick() {
        let v = standard();
        let t = teil(&v, "Langteil Oberleder");
        let TeilForm::Geschlossen(Form::Pfad(pfad)) = &t.teil.form else {
            panic!("Pfad erwartet");
        };
        let PfadElement::Bogen { radius, .. } = pfad.elemente[2] else {
            panic!("Bogen erwartet");
        };
        assert!(radius.etwa_gleich(mm(10.0)), "{radius}");
        assert!(
            t.bedingungen
                .contains(&Bedingung::Tangential { a: 1, b: 2 })
        );
        assert!(
            t.bedingungen
                .contains(&Bedingung::Tangential { a: 2, b: 3 })
        );
    }

    #[test]
    fn alle_skizzen_sind_vollstaendig_bestimmt() {
        for (steg, arm, horn) in [
            (22.0, 185.0, 48.0),
            (12.0, 140.0, 30.0),
            (30.0, 250.0, 60.0),
        ] {
            let v = uhrarmband_mit(&masse(steg, arm, horn)).unwrap();
            for t in &v.teile {
                if let TeilForm::Geschlossen(Form::Pfad(pfad)) = &t.teil.form {
                    let e = skizze::loesen(pfad, true, &t.bedingungen).unwrap();
                    assert_eq!(e.freiheitsgrade, 0, "{steg}/{arm}: {}", t.name);
                }
            }
        }
    }

    #[test]
    fn die_dornloecher_liegen_auf_der_mittellinie_im_gleichen_abstand() {
        let v = standard();
        let t = teil(&v, "Langteil Oberleder");
        assert_eq!(t.teil.ausschnitte.len(), 7);
        let mitten: Vec<Punkt> = t
            .teil
            .ausschnitte
            .iter()
            .map(|a| crate::ausschnitt::mitte(a).unwrap())
            .collect();
        for q in &mitten {
            assert!((q.x - 11.0).abs() < TOLERANZ_MM);
        }
        for paar in mitten.windows(2) {
            assert!((paar[1].y - paar[0].y - 7.0).abs() < TOLERANZ_MM);
        }
        // Das mittlere bei 84 mm, wie abgeleitet.
        assert!((mitten[3].y - 84.0).abs() < TOLERANZ_MM);
    }

    #[test]
    fn ober_und_futterleder_haben_dieselben_dornloecher() {
        let v = standard();
        assert_eq!(
            teil(&v, "Langteil Oberleder").teil.ausschnitte,
            teil(&v, "Langteil Futter").teil.ausschnitte
        );
    }

    #[test]
    fn am_federsteg_wird_nicht_genaeht_damit_er_zwischen_die_lagen_passt() {
        let v = standard();
        for (name, offen) in [
            ("Langteil Oberleder", vec![0]),
            ("Kurzteil Oberleder", vec![0, 2]),
        ] {
            let Some(Naht {
                kanten: Nahtkanten::Pfad(k),
                ..
            }) = &teil(&v, name).teil.naht
            else {
                panic!("Naht erwartet");
            };
            for (i, genaeht) in k.iter().enumerate() {
                assert_eq!(*genaeht, !offen.contains(&i), "{name}, Kante {i}");
            }
        }
    }

    #[test]
    fn die_naht_geht_durch_beide_lagen_und_kein_loch_liegt_zu_dicht() {
        let v = standard();
        let teile: Vec<Teil> = v.teile.iter().map(|t| t.teil.clone()).collect();
        let b = muster::berechnen(&teile);
        let d = muster::durchstechen_berechnet(&teile, &b);
        for (t, d) in v.teile.iter().zip(&d) {
            assert_eq!(d.zu_dicht, 0, "{}: Löcher zu dicht", t.name);
            assert_eq!(d.kantennah, 0, "{}: Loch an der Kante", t.name);
        }
        // Das Futter bekommt alle Nahtlöcher des Oberleders.
        let ober = b[0]
            .as_ref()
            .unwrap()
            .stiche
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .0
            .loecher
            .len();
        assert!(ober > 50, "{ober}");
        assert_eq!(d[1].loecher.len(), ober);
    }

    #[test]
    fn die_schlaufe_umschliesst_beide_bandenden() {
        let m = masse(22.0, 185.0, 48.0);
        let a = uhrarmband_abgeleitet(&m).unwrap();
        // Zwei Lagen Band an der Schließe übereinander: Umfang 2 · (20 + 2 · 3,5).
        let umfang = 2.0 * (20.0 + 2.0 * 3.5);
        assert!(a.schlaufe.wert() > umfang);
        assert!(a.schlaufe.wert() < umfang + 5.0);
    }

    #[test]
    fn unsinnige_eingaben_werden_abgelehnt() {
        let fehler = |steg, arm, horn| uhrarmband(mm(steg), mm(arm), mm(horn)).unwrap_err();
        assert_eq!(
            fehler(4.0, 185.0, 48.0),
            VorlagenFehler::Uhrarmband(UhrarmbandFehler::Stegbreite)
        );
        assert_eq!(
            fehler(22.0, 30.0, 48.0),
            VorlagenFehler::Uhrarmband(UhrarmbandFehler::Armumfang)
        );
        assert_eq!(
            fehler(22.0, 185.0, 5.0),
            VorlagenFehler::Uhrarmband(UhrarmbandFehler::Hornabstand)
        );
        assert_eq!(
            fehler(22.0, 120.0, 65.0),
            VorlagenFehler::Uhrarmband(UhrarmbandFehler::Hornabstand)
        );
        assert_eq!(
            fehler(f64::NAN, 185.0, 48.0),
            VorlagenFehler::Uhrarmband(UhrarmbandFehler::Stegbreite)
        );
    }
}
