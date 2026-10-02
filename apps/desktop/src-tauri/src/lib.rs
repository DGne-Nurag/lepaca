//! Brücke zwischen Oberfläche (TypeScript) und Kern (Rust).
//!
//! Die Oberfläche schickt nur Zahlen und Auswahlwerte; alle Geometrie wird
//! hier im Kern berechnet und validiert. Dateien öffnet und schreibt nur
//! dieser Teil, die Oberfläche bekommt keinen Dateizugriff (ADR 0002).

use lepaca_core::eisen::Eisenart;
use lepaca_core::form::{Form, FormFehler, TPocketNaehte};
use lepaca_core::geometrie::{Kontur, KonturFehler};
use lepaca_core::kachelung::{self, KachelFehler, Papier};
use lepaca_core::mass::Mm;
use lepaca_core::stichlinie::{self, Eckausrichtung, StichFehler, Stichreihe};
use lepaca_core::versatz::VersatzFehler;
use lepaca_io::pdf::{Druckteil, muster_gekachelt};
use serde::{Deserialize, Serialize};
use tauri_plugin_dialog::DialogExt;

/// Höchstzahl Teile in einem Muster.
pub const MAX_TEILE: usize = 1000;

#[derive(Debug, Clone, Deserialize)]
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StichDaten {
    pub kantenabstand: f64,
    pub lochabstand: f64,
    pub eisen: String,
    pub ecke: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeilDaten {
    pub form: FormDaten,
    /// Linke obere Ecke im Muster (mm).
    pub x: f64,
    pub y: f64,
    pub stiche: Option<StichDaten>,
}

/// Fertig berechnete Geometrie zum Zeichnen.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Geometrie {
    pub kontur: Vec<[f64; 2]>,
    pub linien: Vec<Linie>,
    pub loecher: Vec<[[f64; 2]; 4]>,
    pub ecklocher: usize,
    /// Tatsächliche Lochabstände je Abschnitt (mm).
    pub abstaende: Vec<f64>,
    /// Fehlercode, falls die Stichreihe nicht möglich ist; die Oberfläche
    /// übersetzt ihn in einen verständlichen Text.
    pub stichfehler: Option<String>,
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

fn eisenart(name: &str) -> Result<Eisenart, String> {
    match name {
        "french" => Ok(Eisenart::Franzoesisch),
        "diamond" => Ok(Eisenart::Diamant),
        "japan" => Ok(Eisenart::Japanisch),
        _ => Err("stich.eisen".into()),
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

/// Kontur des Teils an seiner Position im Muster, dazu die Kanten mit
/// Naht (`None`: ringsum).
fn kontur(teil: &TeilDaten) -> Result<(Kontur, Option<Vec<bool>>), String> {
    if !(teil.x.is_finite() && teil.y.is_finite()) || teil.x.abs() > 1e5 || teil.y.abs() > 1e5 {
        return Err("form.position".into());
    }
    let mut kanten = None;
    let form = match teil.form {
        FormDaten::Rechteck {
            breite,
            hoehe,
            eckenradius,
        } => Form::Rechteck {
            breite: mm(breite, "form.groesse")?,
            hoehe: mm(hoehe, "form.groesse")?,
            eckenradius: mm(eckenradius, "form.eckenradius")?,
        },
        FormDaten::Kreis { durchmesser } => Form::Kreis {
            durchmesser: mm(durchmesser, "form.groesse")?,
        },
        FormDaten::Ellipse { breite, hoehe } => Form::Ellipse {
            breite: mm(breite, "form.groesse")?,
            hoehe: mm(hoehe, "form.groesse")?,
        },
        FormDaten::Tpocket {
            breite,
            hoehe,
            fluegel_ueberstand,
            fluegel_hoehe,
            naht_fluegel,
            naht_steg,
            naht_oeffnung,
        } => {
            kanten = Some(
                TPocketNaehte {
                    fluegel: naht_fluegel,
                    steg_unten: naht_steg,
                    oeffnung: naht_oeffnung,
                }
                .kanten(),
            );
            Form::TPocket {
                breite: mm(breite, "form.groesse")?,
                hoehe: mm(hoehe, "form.groesse")?,
                fluegel_ueberstand: mm(fluegel_ueberstand, "form.fluegel")?,
                fluegel_hoehe: mm(fluegel_hoehe, "form.fluegel")?,
            }
        }
    };
    let k = form
        .kontur()
        .map_err(fehlercode_form)?
        .verschoben(teil.x, teil.y);
    Ok((k, kanten))
}

type Stiche = (Stichreihe, lepaca_core::eisen::Lochform);

fn stiche(k: &Kontur, kanten: Option<&[bool]>, d: &StichDaten) -> Result<Stiche, String> {
    let e = stichlinie::Einstellung {
        kantenabstand: mm(d.kantenabstand, "stich.kantenabstand")?,
        lochabstand: mm(d.lochabstand, "stich.lochabstand")?,
        ecke: eckausrichtung(&d.ecke)?,
    };
    let eisen = eisenart(&d.eisen)?;
    let reihe = match kanten {
        Some(kanten) => stichlinie::stichreihe_an_kanten(k, kanten, e),
        None => stichlinie::stichreihe(k, e),
    }
    .map_err(fehlercode_stich)?;
    Ok((reihe, eisen.lochform(e.lochabstand)))
}

fn punkte(k: &Kontur) -> Vec<[f64; 2]> {
    k.punkte().iter().map(|p| [p.x, p.y]).collect()
}

/// Berechnet Schnittkante, Stichlinie und Löcher eines Teils.
pub fn geometrie(teil: &TeilDaten) -> Result<Geometrie, String> {
    let (k, kanten) = kontur(teil)?;
    let mut g = Geometrie {
        kontur: punkte(&k),
        linien: Vec::new(),
        loecher: Vec::new(),
        ecklocher: 0,
        abstaende: Vec::new(),
        stichfehler: None,
    };
    if let Some(d) = &teil.stiche {
        match stiche(&k, kanten.as_deref(), d) {
            Ok((reihe, loch)) => {
                g.linien = reihe
                    .linien
                    .iter()
                    .map(|l| Linie {
                        punkte: l.punkte.iter().map(|p| [p.x, p.y]).collect(),
                        geschlossen: l.geschlossen,
                    })
                    .collect();
                g.ecklocher = reihe.loecher.iter().filter(|l| l.auf_ecke).count();
                g.loecher = reihe
                    .loecher
                    .iter()
                    .map(|l| loch.umriss(l).map(|p| [p.x, p.y]))
                    .collect();
                g.abstaende = reihe
                    .abschnitte
                    .iter()
                    .map(|a| a.lochabstand.wert())
                    .collect();
            }
            Err(code) => g.stichfehler = Some(code),
        }
    }
    Ok(g)
}

#[tauri::command]
fn teil_geometrie(teil: TeilDaten) -> Result<Geometrie, String> {
    geometrie(&teil)
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
    let mut konturen = Vec::with_capacity(teile.len());
    let mut alle_stiche = Vec::with_capacity(teile.len());
    for t in teile {
        let (k, kanten) = kontur(t)?;
        let s = match &t.stiche {
            Some(d) => Some(stiche(&k, kanten.as_deref(), d)?),
            None => None,
        };
        konturen.push(k);
        alle_stiche.push(s);
    }
    let mut von = lepaca_core::geometrie::Punkt::neu(f64::INFINITY, f64::INFINITY);
    let mut bis = lepaca_core::geometrie::Punkt::neu(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for k in &konturen {
        let (a, b) = k.grenzen();
        von = lepaca_core::geometrie::Punkt::neu(von.x.min(a.x), von.y.min(a.y));
        bis = lepaca_core::geometrie::Punkt::neu(bis.x.max(b.x), bis.y.max(b.y));
    }
    let einstellung = kachelung::Einstellung {
        papier,
        rand: mm(druck.rand, "export.rand")?,
        klebekante: mm(druck.klebekante, "export.klebekante")?,
    };
    let plan = kachelung::kacheln(von, bis, einstellung).map_err(|f| match f {
        KachelFehler::RandAusserhalb => "export.rand".to_string(),
        KachelFehler::KlebekanteAusserhalb => "export.klebekante".to_string(),
        KachelFehler::PapierZuKlein => "export.papierZuKlein".to_string(),
        KachelFehler::ZuVieleBlaetter => "export.zuVieleBlaetter".to_string(),
        KachelFehler::UngueltigerBereich => "export.leer".to_string(),
    })?;
    let druckteile: Vec<Druckteil<'_>> = konturen
        .iter()
        .zip(&alle_stiche)
        .map(|(k, s)| Druckteil {
            kontur: k,
            stiche: s.as_ref().map(|(r, l)| (r, l)),
        })
        .collect();
    Ok((muster_gekachelt(&druckteile, &plan), plan.kacheln.len()))
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
    let ergebnis = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![teil_geometrie, pdf_exportieren])
        .run(tauri::generate_context!());
    if let Err(e) = ergebnis {
        eprintln!("lepaca konnte nicht starten: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rechteck(x: f64, y: f64) -> TeilDaten {
        TeilDaten {
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
}
