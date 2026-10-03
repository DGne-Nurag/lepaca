//! Brücke zwischen Oberfläche (TypeScript) und Kern (Rust).
//!
//! Die Oberfläche schickt nur Zahlen und Auswahlwerte; alle Geometrie wird
//! hier im Kern berechnet und validiert. Dateien öffnet und schreibt nur
//! dieser Teil, die Oberfläche bekommt keinen Dateizugriff (ADR 0002).

use lepaca_core::eisen::Eisenart;
use lepaca_core::form::{Form, FormFehler, Pfad, PfadElement, TPocketNaehte};
use lepaca_core::geometrie::{KonturFehler, Punkt};
use lepaca_core::kachelung::{self, KachelFehler, Papier};
use lepaca_core::mass::Mm;
use lepaca_core::muster::{
    self, Montage, Musterteil, Naht, Nahtkanten, StichOderForm, Teil, TeilForm,
};
use lepaca_core::skizze::{self, Bedingung, SkizzenFehler};
use lepaca_core::stichlinie::{Eckausrichtung, Einstellung, StichFehler};
use lepaca_core::versatz::VersatzFehler;
use lepaca_io::pdf::MusterFehler;
use serde::{Deserialize, Serialize};
use tauri_plugin_dialog::DialogExt;

/// Höchstzahl Teile in einem Muster.
pub const MAX_TEILE: usize = muster::MAX_TEILE;
/// Höchstzahl Elemente (Linien, Bögen, Kurven) in einem Pfad.
pub const MAX_ELEMENTE: usize = 10_000;
/// Höchstlänge eines Teilnamens (Zeichen).
pub const MAX_NAME: usize = 80;

/// Ein Stück eines gezeichneten Pfads. Koordinaten relativ zum Teil (mm).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "art", rename_all = "lowercase", deny_unknown_fields)]
pub enum ElementDaten {
    Linie {
        ziel: [f64; 2],
    },
    Kurve {
        k1: [f64; 2],
        k2: [f64; 2],
        ziel: [f64; 2],
    },
    Bogen {
        ziel: [f64; 2],
        radius: f64,
        uhrzeigersinn: bool,
        gross: bool,
    },
}

/// Maß oder Bedingung einer Skizze; Zählweise siehe `lepaca_core::skizze`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "art", rename_all = "camelCase", deny_unknown_fields)]
pub enum BedingungDaten {
    Laenge { kante: usize, wert: f64 },
    Abstand { a: usize, b: usize, wert: f64 },
    AbstandX { a: usize, b: usize, wert: f64 },
    AbstandY { a: usize, b: usize, wert: f64 },
    Radius { kante: usize, wert: f64 },
    Winkel { a: usize, b: usize, grad: f64 },
    Horizontal { kante: usize },
    Vertikal { kante: usize },
    Parallel { a: usize, b: usize },
    Senkrecht { a: usize, b: usize },
    Gleich { a: usize, b: usize },
    Tangential { a: usize, b: usize },
    Fest { punkt: usize, x: f64, y: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "art", rename_all = "lowercase", deny_unknown_fields)]
pub enum FormDaten {
    Rechteck {
        breite: f64,
        hoehe: f64,
        eckenradius: f64,
    },
    Kreis {
        durchmesser: f64,
    },
    Ellipse {
        breite: f64,
        hoehe: f64,
    },
    /// Kartenfach in T-Form. Die Naht läuft nur an den gewählten Kanten.
    #[serde(rename_all = "camelCase")]
    Tpocket {
        breite: f64,
        hoehe: f64,
        fluegel_ueberstand: f64,
        fluegel_hoehe: f64,
        naht_fluegel: bool,
        naht_steg: bool,
        naht_oeffnung: bool,
    },
    /// Frei gezeichnet. Geschlossen ist es ein Teil, offen eine Linie
    /// (z. B. Aufnähnaht, Markierung).
    Pfad {
        start: [f64; 2],
        elemente: Vec<ElementDaten>,
        geschlossen: bool,
        /// Maße und Bedingungen der Skizze.
        #[serde(default)]
        bedingungen: Vec<BedingungDaten>,
        /// Je Kante, ob dort genäht wird; fehlt die Angabe, ringsum.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        naehte: Option<Vec<bool>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StichDaten {
    pub kantenabstand: f64,
    pub lochabstand: f64,
    pub eisen: String,
    pub ecke: String,
}

/// Lage im Zusammenbau und Nähschritt.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MontageDaten {
    pub x: f64,
    pub y: f64,
    pub schritt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeilDaten {
    #[serde(default)]
    pub name: Option<String>,
    pub form: FormDaten,
    /// Linke obere Ecke im Muster (mm).
    pub x: f64,
    pub y: f64,
    pub stiche: Option<StichDaten>,
    #[serde(default)]
    pub montage: Option<MontageDaten>,
}

/// Ein Maß zum Zeichnen (Koordinaten relativ zum Teil).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MassDaten {
    pub bedingung: Option<usize>,
    pub striche: Vec<[[f64; 2]; 2]>,
    pub pfeile: Vec<[[f64; 2]; 2]>,
    pub text: String,
    pub text_pos: [f64; 2],
    pub text_winkel: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SymbolDaten {
    pub bedingung: usize,
    pub pos: [f64; 2],
    pub text: String,
}

/// Fertig berechnete Geometrie zum Zeichnen.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Geometrie {
    pub kontur: Vec<[f64; 2]>,
    /// `false` bei einer offenen Linie: nicht füllen, nicht schließen.
    pub geschlossen: bool,
    pub linien: Vec<Linie>,
    pub loecher: Vec<Vec<[f64; 2]>>,
    pub ecklocher: usize,
    /// Tatsächliche Lochabstände je Abschnitt (mm).
    pub abstaende: Vec<f64>,
    /// Fehlercode, falls die Stichreihe nicht möglich ist; die Oberfläche
    /// übersetzt ihn in einen verständlichen Text.
    pub stichfehler: Option<String>,
    /// Maße der Skizze bzw. der Grundform.
    pub masse: Vec<MassDaten>,
    /// Symbole der geometrischen Bedingungen.
    pub symbole: Vec<SymbolDaten>,
}

/// Ein Stück Stichlinie zum Zeichnen.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Linie {
    pub punkte: Vec<[f64; 2]>,
    pub geschlossen: bool,
}

fn mm(x: f64, code: &str) -> Result<Mm, String> {
    Mm::neu(x).map_err(|_| code.to_string())
}

fn fehlercode_form(f: FormFehler) -> String {
    match f {
        FormFehler::GroesseAusserhalb => "form.groesse".into(),
        FormFehler::EckenradiusZuGross => "form.eckenradius".into(),
        FormFehler::FluegelZuGross => "form.fluegel".into(),
        FormFehler::BogenUnmoeglich => "form.bogen".into(),
        FormFehler::PfadLeer => "form.pfadLeer".into(),
        FormFehler::Kontur(KonturFehler::ZuVielePunkte) => "form.zuVielePunkte".into(),
        FormFehler::Kontur(_) => "form.ungueltig".into(),
    }
}

fn fehlercode_stich(f: StichFehler) -> String {
    match f {
        StichFehler::LochabstandAusserhalb => "stich.lochabstand".into(),
        StichFehler::KantenabstandAusserhalb => "stich.kantenabstand".into(),
        StichFehler::ZuVieleLoecher => "stich.zuVieleLoecher".into(),
        StichFehler::KantenUnpassend => "stich.kanten".into(),
        StichFehler::Versatz(VersatzFehler::AbstandZuGross) => "stich.abstandZuGross".into(),
        StichFehler::Versatz(VersatzFehler::Spitze) => "stich.spitze".into(),
    }
}

fn fehlercode(f: StichOderForm) -> String {
    match f {
        StichOderForm::Form(f) => fehlercode_form(f),
        StichOderForm::Stich(f) => fehlercode_stich(f),
    }
}

fn fehlercode_skizze(f: SkizzenFehler) -> String {
    match f {
        SkizzenFehler::ZuGross => "skizze.zuGross".into(),
        SkizzenFehler::Verweis => "skizze.verweis".into(),
        SkizzenFehler::KanteUnpassend => "skizze.kanteUnpassend".into(),
        SkizzenFehler::UngueltigerWert => "skizze.wert".into(),
        SkizzenFehler::Widerspruch => "skizze.widerspruch".into(),
        SkizzenFehler::Form(f) => fehlercode_form(f),
    }
}

fn eisenart(name: &str) -> Result<Eisenart, String> {
    match name {
        "french" => Ok(Eisenart::Franzoesisch),
        "diamond" => Ok(Eisenart::Diamant),
        "japan" => Ok(Eisenart::Japanisch),
        "round" => Ok(Eisenart::Rund),
        _ => Err("stich.eisen".into()),
    }
}

fn eisen_name(e: Eisenart) -> &'static str {
    match e {
        Eisenart::Franzoesisch => "french",
        Eisenart::Diamant => "diamond",
        Eisenart::Japanisch => "japan",
        Eisenart::Rund => "round",
    }
}

fn eckausrichtung(name: &str) -> Result<Eckausrichtung, String> {
    match name {
        "ein" => Ok(Eckausrichtung::Einlaufend),
        "aus" => Ok(Eckausrichtung::Auslaufend),
        "mitte" => Ok(Eckausrichtung::Mitte),
        _ => Err("stich.ecke".into()),
    }
}

fn ecke_name(e: Eckausrichtung) -> &'static str {
    match e {
        Eckausrichtung::Einlaufend => "ein",
        Eckausrichtung::Auslaufend => "aus",
        Eckausrichtung::Mitte => "mitte",
    }
}

fn punkt(p: [f64; 2]) -> Punkt {
    Punkt::neu(p[0], p[1])
}

fn feld(p: Punkt) -> [f64; 2] {
    [p.x, p.y]
}

fn pfad_aus(start: [f64; 2], elemente: &[ElementDaten]) -> Result<Pfad, String> {
    if elemente.len() > MAX_ELEMENTE {
        return Err("form.zuVielePunkte".into());
    }
    let elemente = elemente
        .iter()
        .map(|e| {
            Ok(match *e {
                ElementDaten::Linie { ziel } => PfadElement::Linie(punkt(ziel)),
                ElementDaten::Kurve { k1, k2, ziel } => PfadElement::Kurve {
                    k1: punkt(k1),
                    k2: punkt(k2),
                    ziel: punkt(ziel),
                },
                ElementDaten::Bogen {
                    ziel,
                    radius,
                    uhrzeigersinn,
                    gross,
                } => PfadElement::Bogen {
                    ziel: punkt(ziel),
                    radius: mm(radius, "form.bogen")?,
                    uhrzeigersinn,
                    gross,
                },
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Pfad {
        start: punkt(start),
        elemente,
    })
}

fn elemente_nach(p: &Pfad) -> Vec<ElementDaten> {
    p.elemente
        .iter()
        .map(|e| match *e {
            PfadElement::Linie(z) => ElementDaten::Linie { ziel: feld(z) },
            PfadElement::Kurve { k1, k2, ziel } => ElementDaten::Kurve {
                k1: feld(k1),
                k2: feld(k2),
                ziel: feld(ziel),
            },
            PfadElement::Bogen {
                ziel,
                radius,
                uhrzeigersinn,
                gross,
            } => ElementDaten::Bogen {
                ziel: feld(ziel),
                radius: radius.wert(),
                uhrzeigersinn,
                gross,
            },
        })
        .collect()
}

fn bedingung_aus(b: &BedingungDaten) -> Result<Bedingung, String> {
    let w = |x: f64| mm(x, "skizze.wert");
    Ok(match *b {
        BedingungDaten::Laenge { kante, wert } => Bedingung::Laenge {
            kante,
            wert: w(wert)?,
        },
        BedingungDaten::Abstand { a, b, wert } => Bedingung::Abstand {
            a,
            b,
            wert: w(wert)?,
        },
        BedingungDaten::AbstandX { a, b, wert } => Bedingung::AbstandX {
            a,
            b,
            wert: w(wert)?,
        },
        BedingungDaten::AbstandY { a, b, wert } => Bedingung::AbstandY {
            a,
            b,
            wert: w(wert)?,
        },
        BedingungDaten::Radius { kante, wert } => Bedingung::Radius {
            kante,
            wert: w(wert)?,
        },
        BedingungDaten::Winkel { a, b, grad } => Bedingung::Winkel { a, b, grad },
        BedingungDaten::Horizontal { kante } => Bedingung::Horizontal { kante },
        BedingungDaten::Vertikal { kante } => Bedingung::Vertikal { kante },
        BedingungDaten::Parallel { a, b } => Bedingung::Parallel { a, b },
        BedingungDaten::Senkrecht { a, b } => Bedingung::Senkrecht { a, b },
        BedingungDaten::Gleich { a, b } => Bedingung::Gleich { a, b },
        BedingungDaten::Tangential { a, b } => Bedingung::Tangential { a, b },
        BedingungDaten::Fest { punkt, x, y } => Bedingung::Fest { punkt, x, y },
    })
}

fn bedingung_nach(b: &Bedingung) -> BedingungDaten {
    match *b {
        Bedingung::Laenge { kante, wert } => BedingungDaten::Laenge {
            kante,
            wert: wert.wert(),
        },
        Bedingung::Abstand { a, b, wert } => BedingungDaten::Abstand {
            a,
            b,
            wert: wert.wert(),
        },
        Bedingung::AbstandX { a, b, wert } => BedingungDaten::AbstandX {
            a,
            b,
            wert: wert.wert(),
        },
        Bedingung::AbstandY { a, b, wert } => BedingungDaten::AbstandY {
            a,
            b,
            wert: wert.wert(),
        },
        Bedingung::Radius { kante, wert } => BedingungDaten::Radius {
            kante,
            wert: wert.wert(),
        },
        Bedingung::Winkel { a, b, grad } => BedingungDaten::Winkel { a, b, grad },
        Bedingung::Horizontal { kante } => BedingungDaten::Horizontal { kante },
        Bedingung::Vertikal { kante } => BedingungDaten::Vertikal { kante },
        Bedingung::Parallel { a, b } => BedingungDaten::Parallel { a, b },
        Bedingung::Senkrecht { a, b } => BedingungDaten::Senkrecht { a, b },
        Bedingung::Gleich { a, b } => BedingungDaten::Gleich { a, b },
        Bedingung::Tangential { a, b } => BedingungDaten::Tangential { a, b },
        Bedingung::Fest { punkt, x, y } => BedingungDaten::Fest { punkt, x, y },
    }
}

fn bedingungen_aus(b: &[BedingungDaten]) -> Result<Vec<Bedingung>, String> {
    if b.len() > skizze::MAX_BEDINGUNGEN {
        return Err("skizze.zuGross".into());
    }
    b.iter().map(bedingung_aus).collect()
}

/// Wandelt ein Teil aus der Oberfläche in ein Teil des Kerns.
fn teil_aus(t: &TeilDaten) -> Result<Musterteil, String> {
    if !(t.x.is_finite() && t.y.is_finite()) || t.x.abs() > 1e5 || t.y.abs() > 1e5 {
        return Err("form.position".into());
    }
    let mut kanten = Nahtkanten::Ringsum;
    let mut bedingungen = Vec::new();
    let form = match &t.form {
        &FormDaten::Rechteck {
            breite,
            hoehe,
            eckenradius,
        } => TeilForm::Geschlossen(Form::Rechteck {
            breite: mm(breite, "form.groesse")?,
            hoehe: mm(hoehe, "form.groesse")?,
            eckenradius: mm(eckenradius, "form.eckenradius")?,
        }),
        &FormDaten::Kreis { durchmesser } => TeilForm::Geschlossen(Form::Kreis {
            durchmesser: mm(durchmesser, "form.groesse")?,
        }),
        &FormDaten::Ellipse { breite, hoehe } => TeilForm::Geschlossen(Form::Ellipse {
            breite: mm(breite, "form.groesse")?,
            hoehe: mm(hoehe, "form.groesse")?,
        }),
        &FormDaten::Tpocket {
            breite,
            hoehe,
            fluegel_ueberstand,
            fluegel_hoehe,
            naht_fluegel,
            naht_steg,
            naht_oeffnung,
        } => {
            kanten = Nahtkanten::TPocket(TPocketNaehte {
                fluegel: naht_fluegel,
                steg_unten: naht_steg,
                oeffnung: naht_oeffnung,
            });
            TeilForm::Geschlossen(Form::TPocket {
                breite: mm(breite, "form.groesse")?,
                hoehe: mm(hoehe, "form.groesse")?,
                fluegel_ueberstand: mm(fluegel_ueberstand, "form.fluegel")?,
                fluegel_hoehe: mm(fluegel_hoehe, "form.fluegel")?,
            })
        }
        FormDaten::Pfad {
            start,
            elemente,
            geschlossen,
            bedingungen: b,
            naehte,
        } => {
            let p = pfad_aus(*start, elemente)?;
            bedingungen = bedingungen_aus(b)?;
            if let (Some(n), true) = (naehte, *geschlossen) {
                if n.len() > MAX_ELEMENTE + 1 {
                    return Err("stich.kanten".into());
                }
                kanten = Nahtkanten::Pfad(n.clone());
            }
            if *geschlossen {
                TeilForm::Geschlossen(Form::Pfad(p))
            } else {
                TeilForm::Offen(p)
            }
        }
    };
    let naht = match &t.stiche {
        Some(d) => Some(Naht {
            einstellung: Einstellung {
                kantenabstand: mm(d.kantenabstand, "stich.kantenabstand")?,
                lochabstand: mm(d.lochabstand, "stich.lochabstand")?,
                ecke: eckausrichtung(&d.ecke)?,
            },
            eisen: eisenart(&d.eisen)?,
            kanten,
        }),
        None => None,
    };
    let montage = match t.montage {
        Some(m) => {
            if !(m.x.is_finite() && m.y.is_finite()) || m.x.abs() > 1e5 || m.y.abs() > 1e5 {
                return Err("form.position".into());
            }
            Some(Montage {
                position: Punkt::neu(m.x, m.y),
                schritt: m.schritt.min(1000),
            })
        }
        None => None,
    };
    let name: String = t
        .name
        .as_deref()
        .unwrap_or_default()
        .chars()
        .take(MAX_NAME)
        .collect();
    Ok(Musterteil {
        name,
        teil: Teil {
            form,
            position: Punkt::neu(t.x, t.y),
            naht,
            montage,
        },
        bedingungen,
    })
}

/// Wandelt ein Teil des Kerns (z. B. aus einer Vorlage) für die Oberfläche.
fn teil_nach(m: &Musterteil) -> TeilDaten {
    let t = &m.teil;
    let mut tpocket_naehte = TPocketNaehte::default();
    let mut naehte = None;
    if let Some(n) = &t.naht {
        match &n.kanten {
            Nahtkanten::TPocket(k) => tpocket_naehte = *k,
            Nahtkanten::Pfad(k) => naehte = Some(k.clone()),
            Nahtkanten::Ringsum => {}
        }
    }
    let bedingungen: Vec<BedingungDaten> = m.bedingungen.iter().map(bedingung_nach).collect();
    let pfad = |p: &Pfad, geschlossen: bool, naehte: Option<Vec<bool>>| FormDaten::Pfad {
        start: feld(p.start),
        elemente: elemente_nach(p),
        geschlossen,
        bedingungen: bedingungen.clone(),
        naehte,
    };
    let form = match &t.form {
        TeilForm::Offen(p) => pfad(p, false, None),
        TeilForm::Geschlossen(f) => match *f {
            Form::Rechteck {
                breite,
                hoehe,
                eckenradius,
            } => FormDaten::Rechteck {
                breite: breite.wert(),
                hoehe: hoehe.wert(),
                eckenradius: eckenradius.wert(),
            },
            Form::Kreis { durchmesser } => FormDaten::Kreis {
                durchmesser: durchmesser.wert(),
            },
            Form::Ellipse { breite, hoehe } => FormDaten::Ellipse {
                breite: breite.wert(),
                hoehe: hoehe.wert(),
            },
            Form::TPocket {
                breite,
                hoehe,
                fluegel_ueberstand,
                fluegel_hoehe,
            } => FormDaten::Tpocket {
                breite: breite.wert(),
                hoehe: hoehe.wert(),
                fluegel_ueberstand: fluegel_ueberstand.wert(),
                fluegel_hoehe: fluegel_hoehe.wert(),
                naht_fluegel: tpocket_naehte.fluegel,
                naht_steg: tpocket_naehte.steg_unten,
                naht_oeffnung: tpocket_naehte.oeffnung,
            },
            Form::Pfad(ref p) => pfad(p, true, naehte),
        },
    };
    TeilDaten {
        name: Some(m.name.clone()),
        form,
        x: t.position.x,
        y: t.position.y,
        stiche: t.naht.as_ref().map(|n| StichDaten {
            kantenabstand: n.einstellung.kantenabstand.wert(),
            lochabstand: n.einstellung.lochabstand.wert(),
            eisen: eisen_name(n.eisen).into(),
            ecke: ecke_name(n.einstellung.ecke).into(),
        }),
        montage: t.montage.map(|m| MontageDaten {
            x: m.position.x,
            y: m.position.y,
            schritt: m.schritt,
        }),
    }
}

fn mass_daten(m: &skizze::Masslinie) -> MassDaten {
    MassDaten {
        bedingung: m.bedingung,
        striche: m
            .striche
            .iter()
            .map(|(a, b)| [feld(*a), feld(*b)])
            .collect(),
        pfeile: m.pfeile.iter().map(|(a, b)| [feld(*a), feld(*b)]).collect(),
        text: m.text.clone(),
        text_pos: feld(m.text_pos),
        text_winkel: m.text_winkel,
    }
}

/// Berechnet Schnittkante (oder Linie), Stichlinie, Löcher und Maße eines
/// Teils.
pub fn geometrie(daten: &TeilDaten) -> Result<Geometrie, String> {
    let mt = teil_aus(daten)?;
    let t = &mt.teil;
    let u = t.umriss().map_err(fehlercode)?;
    let mut g = Geometrie {
        kontur: u.punkte().iter().map(|p| feld(*p)).collect(),
        geschlossen: u.geschlossen(),
        linien: Vec::new(),
        loecher: Vec::new(),
        ecklocher: 0,
        abstaende: Vec::new(),
        stichfehler: None,
        masse: Vec::new(),
        symbole: Vec::new(),
    };
    // Maße relativ zum Teil: Skizzen rechnen ohnehin relativ zum
    // Bezugspunkt, Grundformen haben ihre linke obere Ecke dort.
    let (masse, symbole) = match &t.form {
        TeilForm::Geschlossen(Form::Pfad(p)) => skizze::zeichnung(p, true, &mt.bedingungen),
        TeilForm::Offen(p) => skizze::zeichnung(p, false, &mt.bedingungen),
        TeilForm::Geschlossen(f) => (skizze::form_masse(f), Vec::new()),
    };
    let d = t.position;
    let verschieben = |q: [f64; 2]| [q[0] + d.x, q[1] + d.y];
    g.masse = masse
        .iter()
        .map(|m| {
            let mut m = mass_daten(m);
            for s in &mut m.striche {
                *s = [verschieben(s[0]), verschieben(s[1])];
            }
            for p in &mut m.pfeile {
                p[0] = verschieben(p[0]);
            }
            m.text_pos = verschieben(m.text_pos);
            m
        })
        .collect();
    g.symbole = symbole
        .iter()
        .map(|s| SymbolDaten {
            bedingung: s.bedingung,
            pos: verschieben(feld(s.pos)),
            text: s.text.into(),
        })
        .collect();
    match t.stiche(&u) {
        Some(Ok((reihe, loch))) => {
            g.linien = reihe
                .linien
                .iter()
                .map(|l| Linie {
                    punkte: l.punkte.iter().map(|p| feld(*p)).collect(),
                    geschlossen: l.geschlossen,
                })
                .collect();
            g.ecklocher = reihe.loecher.iter().filter(|l| l.auf_ecke).count();
            g.loecher = reihe
                .loecher
                .iter()
                .map(|l| loch.umriss(l).iter().map(|p| feld(*p)).collect())
                .collect();
            g.abstaende = reihe
                .abschnitte
                .iter()
                .map(|a| a.lochabstand.wert())
                .collect();
        }
        Some(Err(f)) => g.stichfehler = Some(fehlercode_stich(f)),
        None => {}
    }
    Ok(g)
}

#[tauri::command]
fn teil_geometrie(teil: TeilDaten) -> Result<Geometrie, String> {
    geometrie(&teil)
}

/// Ergebnis des Skizzen-Lösers für die Oberfläche.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkizzenErgebnis {
    pub elemente: Vec<ElementDaten>,
    pub freiheitsgrade: usize,
    pub ueberbestimmt: bool,
    pub freie_punkte: Vec<bool>,
    pub freie_kanten: Vec<bool>,
}

/// Löst die Skizze eines gezeichneten Pfads mit seinen Bedingungen.
pub fn skizze(form: &FormDaten) -> Result<SkizzenErgebnis, String> {
    let FormDaten::Pfad {
        start,
        elemente,
        geschlossen,
        bedingungen,
        ..
    } = form
    else {
        return Err("skizze.keinPfad".into());
    };
    let p = pfad_aus(*start, elemente)?;
    let b = bedingungen_aus(bedingungen)?;
    let e = skizze::loesen(&p, *geschlossen, &b).map_err(fehlercode_skizze)?;
    Ok(SkizzenErgebnis {
        elemente: elemente_nach(&e.pfad),
        freiheitsgrade: e.freiheitsgrade,
        ueberbestimmt: e.ueberbestimmt,
        freie_punkte: e.freie_punkte,
        freie_kanten: e.freie_kanten,
    })
}

#[tauri::command]
fn skizze_loesen(form: FormDaten) -> Result<SkizzenErgebnis, String> {
    skizze(&form)
}

/// Übernommene Löcher eines Teils, relativ zu seiner Lage im Muster.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MontageErgebnis {
    pub loecher: Vec<Vec<[f64; 2]>>,
    pub kantennah: usize,
    pub zu_dicht: usize,
}

/// Für jedes Teil die Löcher aus Nähten anderer Teile, die beim Nähen
/// durch es hindurchgehen.
pub fn montage(teile: &[TeilDaten]) -> Result<Vec<MontageErgebnis>, String> {
    if teile.len() > MAX_TEILE {
        return Err("export.zuVieleTeile".into());
    }
    // Fehlerhafte Teile nehmen nicht teil, die anderen schon.
    let mt: Vec<Option<Musterteil>> = teile.iter().map(|t| teil_aus(t).ok()).collect();
    let roh: Vec<Teil> = mt
        .iter()
        .map(|m| match m {
            Some(m) => m.teil.clone(),
            None => Teil {
                form: TeilForm::Offen(Pfad {
                    start: Punkt::neu(0.0, 0.0),
                    elemente: Vec::new(),
                }),
                position: Punkt::neu(0.0, 0.0),
                naht: None,
                montage: None,
            },
        })
        .collect();
    let berechnet = muster::berechnen(&roh);
    let d = muster::durchstechen_berechnet(&roh, &berechnet);
    Ok(d.iter()
        .zip(&roh)
        .map(|(d, t)| MontageErgebnis {
            loecher: d
                .loecher
                .iter()
                .map(|l| {
                    l.form
                        .umriss(&l.loch)
                        .iter()
                        .map(|q| feld(q.minus(t.position)))
                        .collect()
                })
                .collect(),
            kantennah: d.kantennah,
            zu_dicht: d.zu_dicht,
        })
        .collect())
}

#[tauri::command]
fn montage_loecher(teile: Vec<TeilDaten>) -> Result<Vec<MontageErgebnis>, String> {
    montage(&teile)
}

/// Eine Vorlage für die Oberfläche: Teile und Nähanleitung.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VorlageDaten {
    pub teile: Vec<TeilDaten>,
    pub schritte: Vec<String>,
}

pub fn vorlage_laden(name: &str) -> Result<VorlageDaten, String> {
    let v = match name {
        "bifold" => lepaca_core::vorlage::bifold().map_err(|_| "vorlage.fehler".to_string())?,
        _ => return Err("vorlage.unbekannt".into()),
    };
    Ok(VorlageDaten {
        teile: v.teile.iter().map(teil_nach).collect(),
        schritte: v.schritte,
    })
}

#[tauri::command]
fn vorlage(name: String) -> Result<VorlageDaten, String> {
    vorlage_laden(&name)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DruckDaten {
    pub papier: String,
    pub rand: f64,
    pub klebekante: f64,
}

/// Erzeugt das PDF eines Musters im Speicher.
pub fn pdf(teile: &[TeilDaten], druck: &DruckDaten) -> Result<(Vec<u8>, usize), String> {
    if teile.is_empty() {
        return Err("export.leer".into());
    }
    if teile.len() > MAX_TEILE {
        return Err("export.zuVieleTeile".into());
    }
    let papier = match druck.papier.as_str() {
        "a4" => Papier::A4,
        "a3" => Papier::A3,
        "letter" => Papier::LETTER,
        _ => return Err("export.papier".into()),
    };
    let mt = teile
        .iter()
        .map(teil_aus)
        .collect::<Result<Vec<_>, String>>()?;
    let einstellung = kachelung::Einstellung {
        papier,
        rand: mm(druck.rand, "export.rand")?,
        klebekante: mm(druck.klebekante, "export.klebekante")?,
    };
    lepaca_io::pdf::muster_pdf(&mt, einstellung).map_err(|f| match f {
        MusterFehler::Leer => "export.leer".to_string(),
        MusterFehler::ZuVieleTeile => "export.zuVieleTeile".to_string(),
        MusterFehler::Teil(_, f) => fehlercode(f),
        MusterFehler::Kachel(KachelFehler::RandAusserhalb) => "export.rand".to_string(),
        MusterFehler::Kachel(KachelFehler::KlebekanteAusserhalb) => "export.klebekante".to_string(),
        MusterFehler::Kachel(KachelFehler::PapierZuKlein) => "export.papierZuKlein".to_string(),
        MusterFehler::Kachel(KachelFehler::ZuVieleBlaetter) => "export.zuVieleBlaetter".to_string(),
        MusterFehler::Kachel(KachelFehler::UngueltigerBereich) => "export.leer".to_string(),
    })
}

/// Ergebnis des Exports für die Statusanzeige.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportErgebnis {
    pub abgebrochen: bool,
    pub blaetter: usize,
    pub datei: String,
}

/// Fragt per Systemdialog nach dem Ziel und speichert das PDF atomar.
/// Der Pfad kommt nur aus dem Dialog, nie von der Oberfläche.
#[tauri::command]
async fn pdf_exportieren(
    app: tauri::AppHandle,
    teile: Vec<TeilDaten>,
    druck: DruckDaten,
) -> Result<ExportErgebnis, String> {
    let (inhalt, blaetter) = pdf(&teile, &druck)?;
    let ziel = app
        .dialog()
        .file()
        .add_filter("PDF", &["pdf"])
        .set_file_name("muster.pdf")
        .blocking_save_file();
    let Some(ziel) = ziel else {
        return Ok(ExportErgebnis {
            abgebrochen: true,
            blaetter,
            datei: String::new(),
        });
    };
    let pfad = ziel.into_path().map_err(|_| "export.pfad".to_string())?;
    // Der Systemdialog hat bereits nach dem Überschreiben gefragt.
    lepaca_io::datei::atomar_schreiben(&pfad, &inhalt, true)
        .map_err(|_| "export.schreiben".to_string())?;
    let datei = pfad
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(ExportErgebnis {
        abgebrochen: false,
        blaetter,
        datei,
    })
}

/// Startet die Anwendung.
pub fn starten() {
    #[cfg(target_os = "linux")]
    wayland::umgebung_absichern();
    let ergebnis = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            teil_geometrie,
            skizze_loesen,
            montage_loecher,
            vorlage,
            pdf_exportieren
        ])
        .run(tauri::generate_context!());
    if let Err(e) = ergebnis {
        eprintln!("lepaca konnte nicht starten: {e}");
        std::process::exit(1);
    }
}

/// Umgehung für WebKitGTK unter Wayland.
///
/// Mit dem DMA-BUF-Renderer von WebKitGTK bricht die App auf manchen
/// Wayland-Systemen sofort mit „Error 71 (Protokollfehler) dispatching to
/// Wayland display“ ab. Abhilfe ist die Umgebungsvariable
/// `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Sie muss gesetzt sein, bevor GTK
/// startet; `std::env::set_var` ist in Edition 2024 aber `unsafe` und damit
/// bei uns verboten. Deshalb startet sich die App einmal selbst neu (per
/// `exec`, ohne zweiten Prozess) mit der ergänzten Umgebung.
///
/// Wer die Variable selbst setzt (auch auf `0`), behält seinen Wert.
#[cfg(target_os = "linux")]
mod wayland {
    use std::ffi::OsString;

    const DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    const SCHON_NEU_GESTARTET: &str = "LEPACA_UMGEBUNG_GESETZT";

    /// Welche Variablen für den Neustart ergänzt werden müssen; leer heißt:
    /// kein Neustart nötig.
    pub(crate) fn fehlende_variablen(
        lesen: impl Fn(&str) -> Option<OsString>,
    ) -> Vec<(&'static str, &'static str)> {
        let wayland = lesen("WAYLAND_DISPLAY").is_some_and(|w| !w.is_empty())
            || lesen("XDG_SESSION_TYPE").is_some_and(|t| t == "wayland");
        if !wayland || lesen(SCHON_NEU_GESTARTET).is_some() || lesen(DMABUF).is_some() {
            return Vec::new();
        }
        vec![(DMABUF, "1"), (SCHON_NEU_GESTARTET, "1")]
    }

    pub(crate) fn umgebung_absichern() {
        use std::os::unix::process::CommandExt;

        let fehlend = fehlende_variablen(|k| std::env::var_os(k));
        if fehlend.is_empty() {
            return;
        }
        let Ok(programm) = std::env::current_exe() else {
            return;
        };
        let fehler = std::process::Command::new(programm)
            .args(std::env::args_os().skip(1))
            .envs(fehlend)
            .exec();
        // `exec` kehrt nur bei einem Fehler zurück; dann ohne Umgehung weiter.
        eprintln!("lepaca: Neustart mit {DMABUF}=1 fehlgeschlagen: {fehler}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rechteck(x: f64, y: f64) -> TeilDaten {
        TeilDaten {
            name: None,
            form: FormDaten::Rechteck {
                breite: 80.0,
                hoehe: 50.0,
                eckenradius: 0.0,
            },
            x,
            y,
            stiche: Some(StichDaten {
                kantenabstand: 3.0,
                lochabstand: 4.0,
                eisen: "diamond".into(),
                ecke: "ein".into(),
            }),
            montage: None,
        }
    }

    #[test]
    fn ein_teil_wird_an_seiner_position_berechnet() {
        let g = geometrie(&rechteck(100.0, 20.0)).unwrap();
        assert_eq!(g.kontur[0], [100.0, 20.0]);
        assert_eq!(g.ecklocher, 4);
        assert_eq!(g.loecher.len(), 60);
        assert!(g.stichfehler.is_none());
    }

    #[test]
    fn mit_rundem_eisen_werden_die_loecher_rund() {
        let mut t = rechteck(0.0, 0.0);
        if let Some(n) = t.stiche.as_mut() {
            n.eisen = "round".into();
        }
        let g = geometrie(&t).unwrap();
        assert!(!g.loecher.is_empty());
        // Runde Löcher sind feine Vielecke, keine Rauten mit vier Ecken.
        assert!(g.loecher.iter().all(|l| l.len() > 4));
    }

    #[test]
    fn ein_zu_grosser_kantenabstand_kommt_als_fehlercode_statt_absturz() {
        let mut t = rechteck(0.0, 0.0);
        if let Some(s) = t.stiche.as_mut() {
            s.kantenabstand = 30.0;
        }
        let g = geometrie(&t).unwrap();
        assert_eq!(g.stichfehler.as_deref(), Some("stich.abstandZuGross"));
        assert!(g.loecher.is_empty());
        assert!(
            !g.kontur.is_empty(),
            "die Schnittkante wird trotzdem gezeigt"
        );
    }

    #[test]
    fn unbekannte_felder_aus_der_oberflaeche_werden_abgewiesen() {
        let json =
            r#"{"form":{"art":"kreis","durchmesser":50},"x":0,"y":0,"stiche":null,"skript":"x"}"#;
        assert!(serde_json::from_str::<TeilDaten>(json).is_err());
        let gut = r#"{"form":{"art":"kreis","durchmesser":50},"x":0,"y":0,"stiche":null}"#;
        assert!(serde_json::from_str::<TeilDaten>(gut).is_ok());
    }

    #[test]
    fn eine_t_pocket_aus_der_oberflaeche_hat_offene_naehte_ohne_oeffnung() {
        let json = r#"{"form":{"art":"tpocket","breite":101.6,"hoehe":44.5,
            "fluegelUeberstand":12.7,"fluegelHoehe":12,
            "nahtFluegel":true,"nahtSteg":true,"nahtOeffnung":false},
            "x":0,"y":0,"stiche":{"kantenabstand":3,"lochabstand":3,"eisen":"diamond","ecke":"ein"}}"#;
        let t: TeilDaten = serde_json::from_str(json).unwrap();
        let g = geometrie(&t).unwrap();
        assert_eq!(g.stichfehler, None);
        assert_eq!(g.linien.len(), 3);
        assert!(g.linien.iter().all(|l| !l.geschlossen));
        assert_eq!(g.kontur.len(), 8);
    }

    #[test]
    fn ein_offener_pfad_mit_bogen_wird_eine_genaehte_linie() {
        let json = r#"{"form":{"art":"pfad","start":[0,0],"geschlossen":false,
            "elemente":[{"art":"linie","ziel":[30,0]},
                        {"art":"bogen","ziel":[30,40],"radius":20,"uhrzeigersinn":true,"gross":false}]},
            "x":10,"y":5,"stiche":{"kantenabstand":3,"lochabstand":3,"eisen":"diamond","ecke":"ein"}}"#;
        let t: TeilDaten = serde_json::from_str(json).unwrap();
        let g = geometrie(&t).unwrap();
        assert!(!g.geschlossen);
        assert_eq!(g.stichfehler, None);
        assert_eq!(g.kontur[0], [10.0, 5.0]);
        assert_eq!(g.linien.len(), 1);
        assert!(!g.linien[0].geschlossen);
    }

    #[test]
    fn ein_bogen_mit_zu_kleinem_radius_kommt_als_fehlercode() {
        let json = r#"{"form":{"art":"pfad","start":[0,0],"geschlossen":true,
            "elemente":[{"art":"bogen","ziel":[50,0],"radius":10,"uhrzeigersinn":true,"gross":false},
                        {"art":"linie","ziel":[0,20]}]},
            "x":0,"y":0,"stiche":null}"#;
        let t: TeilDaten = serde_json::from_str(json).unwrap();
        assert_eq!(geometrie(&t), Err("form.bogen".to_string()));
    }

    #[test]
    fn ein_muster_aus_zwei_teilen_wird_ein_pdf() {
        let druck = DruckDaten {
            papier: "a4".into(),
            rand: 10.0,
            klebekante: 10.0,
        };
        let (pdf, blaetter) = pdf(&[rechteck(0.0, 0.0), rechteck(300.0, 0.0)], &druck).unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert_eq!(blaetter, 2);
        assert_eq!(super::pdf(&[], &druck).unwrap_err(), "export.leer");
    }

    #[test]
    fn die_bifold_vorlage_kommt_vollstaendig_in_der_oberflaeche_an() {
        let v = vorlage_laden("bifold").unwrap();
        assert_eq!(v.teile.len(), 10);
        assert!(!v.schritte.is_empty());
        // Hin und zurück über JSON, wie in der App.
        let json = serde_json::to_string(&v.teile).unwrap();
        let zurueck: Vec<TeilDaten> = serde_json::from_str(&json).unwrap();
        // Bis aufs letzte Bit der Gleitkommazahlen muss es nicht stimmen.
        assert_eq!(zurueck.len(), v.teile.len());
        for (a, b) in zurueck.iter().zip(&v.teile) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.montage.map(|m| m.schritt), b.montage.map(|m| m.schritt));
        }
        // Jedes Teil lässt sich berechnen, die Skizzen sind bestimmt.
        for t in &zurueck {
            let g = geometrie(t).unwrap();
            assert!(g.stichfehler.is_none());
            if let FormDaten::Pfad { .. } = t.form {
                assert_eq!(skizze(&t.form).unwrap().freiheitsgrade, 0);
            }
        }
        // Löcher aus dem Zusammenbau passen ohne Warnung.
        let m = montage(&zurueck).unwrap();
        assert!(m.iter().all(|x| x.kantennah == 0 && x.zu_dicht == 0));
        let futter = zurueck
            .iter()
            .position(|t| t.name.as_deref() == Some("Futter"))
            .unwrap();
        assert!(m[futter].loecher.len() > 100);
        let (pdf, _) = pdf(
            &zurueck,
            &DruckDaten {
                papier: "a4".into(),
                rand: 10.0,
                klebekante: 10.0,
            },
        )
        .unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn ein_mass_aus_der_oberflaeche_macht_das_viereck_genau() {
        let json = r#"{"art":"pfad","start":[0,0],"geschlossen":true,
            "elemente":[{"art":"linie","ziel":[79,2]},{"art":"linie","ziel":[81,49]},
                        {"art":"linie","ziel":[-1,51]},{"art":"linie","ziel":[0,0]}],
            "bedingungen":[{"art":"horizontal","kante":0},{"art":"vertikal","kante":1},
                           {"art":"horizontal","kante":2},{"art":"vertikal","kante":3},
                           {"art":"laenge","kante":0,"wert":80},{"art":"abstandY","a":0,"b":3,"wert":50}]}"#;
        let f: FormDaten = serde_json::from_str(json).unwrap();
        let e = skizze(&f).unwrap();
        assert_eq!(e.freiheitsgrade, 0);
        let ElementDaten::Linie { ziel } = e.elemente[1] else {
            panic!("Linie erwartet");
        };
        assert!((ziel[0] - 80.0).abs() < 1e-6 && (ziel[1] - 50.0).abs() < 1e-6);
    }

    #[test]
    fn eine_widerspruechliche_bemassung_kommt_als_fehlercode() {
        let json = r#"{"art":"pfad","start":[0,0],"geschlossen":false,
            "elemente":[{"art":"linie","ziel":[30,1]}],
            "bedingungen":[{"art":"horizontal","kante":0},{"art":"abstandY","a":0,"b":1,"wert":5}]}"#;
        let f: FormDaten = serde_json::from_str(json).unwrap();
        assert_eq!(skizze(&f), Err("skizze.widerspruch".to_string()));
    }

    #[cfg(target_os = "linux")]
    mod wayland_start {
        use crate::wayland::fehlende_variablen;
        use std::ffi::OsString;

        fn umgebung(paare: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
            let paare: Vec<(String, String)> = paare
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            move |k| {
                paare
                    .iter()
                    .find(|(kk, _)| kk == k)
                    .map(|(_, v)| OsString::from(v))
            }
        }

        #[test]
        fn unter_wayland_wird_der_dmabuf_renderer_abgeschaltet() {
            let v = fehlende_variablen(umgebung(&[("WAYLAND_DISPLAY", "wayland-0")]));
            assert!(v.contains(&("WEBKIT_DISABLE_DMABUF_RENDERER", "1")));
        }

        #[test]
        fn unter_x11_bleibt_alles_wie_es_ist() {
            let v = fehlende_variablen(umgebung(&[("DISPLAY", ":0"), ("XDG_SESSION_TYPE", "x11")]));
            assert!(v.is_empty());
        }

        #[test]
        fn eigene_einstellung_des_nutzers_wird_respektiert() {
            let v = fehlende_variablen(umgebung(&[
                ("WAYLAND_DISPLAY", "wayland-0"),
                ("WEBKIT_DISABLE_DMABUF_RENDERER", "0"),
            ]));
            assert!(v.is_empty());
        }

        #[test]
        fn nach_dem_neustart_startet_die_app_nicht_endlos_neu() {
            let v = fehlende_variablen(umgebung(&[
                ("XDG_SESSION_TYPE", "wayland"),
                ("LEPACA_UMGEBUNG_GESETZT", "1"),
            ]));
            assert!(v.is_empty());
        }
    }
}
