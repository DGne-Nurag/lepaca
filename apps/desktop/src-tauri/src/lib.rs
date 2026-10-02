//! Brücke zwischen Oberfläche (TypeScript) und Kern (Rust).
//!
//! Die Oberfläche schickt nur Zahlen und Auswahlwerte; alle Geometrie wird
//! hier im Kern berechnet und validiert. Dateien öffnet und schreibt nur
//! dieser Teil, die Oberfläche bekommt keinen Dateizugriff (ADR 0002).

use lepaca_core::eisen::Eisenart;
use lepaca_core::form::{Form, FormFehler, OffeneLinie, Pfad, PfadElement, TPocketNaehte};
use lepaca_core::geometrie::{Kontur, KonturFehler, Punkt};
use lepaca_core::kachelung::{self, KachelFehler, Papier};
use lepaca_core::mass::Mm;
use lepaca_core::stichlinie::{self, Eckausrichtung, StichFehler, Stichreihe};
use lepaca_core::versatz::VersatzFehler;
use lepaca_io::pdf::{Druckteil, muster_gekachelt};
use serde::{Deserialize, Serialize};
use tauri_plugin_dialog::DialogExt;

/// Höchstzahl Teile in einem Muster.
pub const MAX_TEILE: usize = 1000;
/// Höchstzahl Elemente (Linien, Bögen, Kurven) in einem Pfad.
pub const MAX_ELEMENTE: usize = 10_000;

/// Ein Stück eines gezeichneten Pfads. Koordinaten relativ zum Teil (mm).
#[derive(Debug, Clone, Deserialize)]
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
    /// Frei gezeichnet. Geschlossen ist es ein Teil, offen eine Linie
    /// (z. B. Aufnähnaht, Markierung).
    Pfad {
        start: [f64; 2],
        elemente: Vec<ElementDaten>,
        geschlossen: bool,
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

fn eisenart(name: &str) -> Result<Eisenart, String> {
    match name {
        "french" => Ok(Eisenart::Franzoesisch),
        "diamond" => Ok(Eisenart::Diamant),
        "japan" => Ok(Eisenart::Japanisch),
        "round" => Ok(Eisenart::Rund),
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

/// Was ein Teil zeichnet: eine geschlossene Schnittkante (mit den Kanten,
/// die genäht werden; `None` heißt ringsum) oder eine offene Linie.
enum Umriss {
    Teil {
        kontur: Kontur,
        kanten: Option<Vec<bool>>,
    },
    Linie(OffeneLinie),
}

impl Umriss {
    fn punkte(&self) -> &[Punkt] {
        match self {
            Umriss::Teil { kontur, .. } => kontur.punkte(),
            Umriss::Linie(l) => &l.punkte,
        }
    }

    fn geschlossen(&self) -> bool {
        matches!(self, Umriss::Teil { .. })
    }
}

fn punkt(p: [f64; 2]) -> Punkt {
    Punkt::neu(p[0], p[1])
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

/// Umriss des Teils an seiner Position im Muster.
fn umriss(teil: &TeilDaten) -> Result<Umriss, String> {
    if !(teil.x.is_finite() && teil.y.is_finite()) || teil.x.abs() > 1e5 || teil.y.abs() > 1e5 {
        return Err("form.position".into());
    }
    let mut kanten = None;
    let form = match &teil.form {
        &FormDaten::Rechteck {
            breite,
            hoehe,
            eckenradius,
        } => Form::Rechteck {
            breite: mm(breite, "form.groesse")?,
            hoehe: mm(hoehe, "form.groesse")?,
            eckenradius: mm(eckenradius, "form.eckenradius")?,
        },
        &FormDaten::Kreis { durchmesser } => Form::Kreis {
            durchmesser: mm(durchmesser, "form.groesse")?,
        },
        &FormDaten::Ellipse { breite, hoehe } => Form::Ellipse {
            breite: mm(breite, "form.groesse")?,
            hoehe: mm(hoehe, "form.groesse")?,
        },
        &FormDaten::Tpocket {
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
        FormDaten::Pfad {
            start,
            elemente,
            geschlossen,
        } => {
            let p = pfad_aus(*start, elemente)?;
            if !geschlossen {
                let mut l = p.linienzug().map_err(fehlercode_form)?;
                let d = Punkt::neu(teil.x, teil.y);
                for q in &mut l.punkte {
                    *q = q.plus(d);
                }
                return Ok(Umriss::Linie(l));
            }
            Form::Pfad(p)
        }
    };
    let kontur = form
        .kontur()
        .map_err(fehlercode_form)?
        .verschoben(teil.x, teil.y);
    Ok(Umriss::Teil { kontur, kanten })
}

type Stiche = (Stichreihe, lepaca_core::eisen::Lochform);

fn stiche(u: &Umriss, d: &StichDaten) -> Result<Stiche, String> {
    let e = stichlinie::Einstellung {
        kantenabstand: mm(d.kantenabstand, "stich.kantenabstand")?,
        lochabstand: mm(d.lochabstand, "stich.lochabstand")?,
        ecke: eckausrichtung(&d.ecke)?,
    };
    let eisen = eisenart(&d.eisen)?;
    let reihe = match u {
        Umriss::Teil {
            kontur,
            kanten: Some(kanten),
        } => stichlinie::stichreihe_an_kanten(kontur, kanten, e),
        Umriss::Teil {
            kontur,
            kanten: None,
        } => stichlinie::stichreihe(kontur, e),
        Umriss::Linie(l) => stichlinie::stichreihe_entlang(&l.punkte, &l.ecke, e),
    }
    .map_err(fehlercode_stich)?;
    Ok((reihe, eisen.lochform(e.lochabstand)))
}

/// Berechnet Schnittkante (oder Linie), Stichlinie und Löcher eines Teils.
pub fn geometrie(teil: &TeilDaten) -> Result<Geometrie, String> {
    let u = umriss(teil)?;
    let mut g = Geometrie {
        kontur: u.punkte().iter().map(|p| [p.x, p.y]).collect(),
        geschlossen: u.geschlossen(),
        linien: Vec::new(),
        loecher: Vec::new(),
        ecklocher: 0,
        abstaende: Vec::new(),
        stichfehler: None,
    };
    if let Some(d) = &teil.stiche {
        match stiche(&u, d) {
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
                    .map(|l| loch.umriss(l).iter().map(|p| [p.x, p.y]).collect())
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
    let mut umrisse = Vec::with_capacity(teile.len());
    let mut alle_stiche = Vec::with_capacity(teile.len());
    for t in teile {
        let u = umriss(t)?;
        let s = match &t.stiche {
            Some(d) => Some(stiche(&u, d)?),
            None => None,
        };
        umrisse.push(u);
        alle_stiche.push(s);
    }
    let mut von = Punkt::neu(f64::INFINITY, f64::INFINITY);
    let mut bis = Punkt::neu(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for q in umrisse.iter().flat_map(|u| u.punkte()) {
        von = Punkt::neu(von.x.min(q.x), von.y.min(q.y));
        bis = Punkt::neu(bis.x.max(q.x), bis.y.max(q.y));
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
    let druckteile: Vec<Druckteil<'_>> = umrisse
        .iter()
        .zip(&alle_stiche)
        .map(|(u, s)| Druckteil {
            umriss: u.punkte(),
            geschlossen: u.geschlossen(),
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
    #[cfg(target_os = "linux")]
    wayland::umgebung_absichern();
    let ergebnis = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![teil_geometrie, pdf_exportieren])
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
