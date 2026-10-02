//! Kommandozeile für lepaca.
//!
//! Beispiel:
//! `lepaca pdf --form rechteck --breite 300 --hoehe 200 --radius 5
//!  --eisen diamond --lochabstand 3.85 --kantenabstand 3 --ausgabe teil.pdf`

use std::path::PathBuf;
use std::process::ExitCode;

use lepaca_core::eisen::Eisenart;
use lepaca_core::form::{Form, TPocketNaehte};
use lepaca_core::kachelung::{Einstellung as KachelEinstellung, Papier, kacheln};
use lepaca_core::mass::Mm;
use lepaca_core::stichlinie::{Eckausrichtung, Einstellung, stichreihe, stichreihe_an_kanten};

const HILFE: &str = "\
lepaca – Leather Pattern CAD

Aufruf:
  lepaca svg [Optionen]   Teil als ein SVG in Originalgröße
  lepaca pdf [Optionen]   Teil als PDF, auf Blätter gekachelt, mit Prüfseite

Optionen (alle Maße in Millimetern):
  --form rechteck|kreis|ellipse|tpocket
                                  Grundform (Standard: rechteck)
  --breite <mm>                   Breite (Rechteck, Ellipse)
  --hoehe <mm>                    Höhe (Rechteck, Ellipse)
  --radius <mm>                   Eckenradius (Rechteck, Standard: 0)
  --durchmesser <mm>              Durchmesser (Kreis)
  --fluegel <mm>                  Flügelüberstand je Seite (T-Pocket, Standard: 12.7)
  --fluegelhoehe <mm>             Flügelhöhe ab Öffnung (T-Pocket, Standard: 12)
  --eisen french|diamond|japan    Eisenart (Standard: diamond)
  --lochabstand <mm>              Zinkenabstand (Standard: 3.85)
  --kantenabstand <mm>            Abstand der Naht zur Kante (Standard: 3)
  --ecke ein|aus|mitte            Ausrichtung des Ecklochs (Standard: ein,
                                  wie die einlaufende Kante)
  --papier a4|a3|letter           Papierformat für PDF (Standard: a4)
  --rand <mm>                     Unbedruckter Blattrand für PDF (Standard: 10)
  --klebekante <mm>               Überlappung der Blätter für PDF (Standard: 10)
  --ausgabe <datei>               Zieldatei
  --ueberschreiben                Vorhandene Datei ersetzen
  --version                       Version anzeigen
";

/// Höchstlänge eines einzelnen Arguments, gegen Unsinn auf der Kommandozeile.
const MAX_ARG_LAENGE: usize = 4096;

#[derive(Debug, Default)]
struct Optionen {
    form: Option<String>,
    breite: Option<f64>,
    hoehe: Option<f64>,
    radius: Option<f64>,
    durchmesser: Option<f64>,
    fluegel: Option<f64>,
    fluegelhoehe: Option<f64>,
    eisen: Option<String>,
    lochabstand: Option<f64>,
    kantenabstand: Option<f64>,
    ecke: Option<String>,
    papier: Option<String>,
    rand: Option<f64>,
    klebekante: Option<f64>,
    ausgabe: Option<PathBuf>,
    ueberschreiben: bool,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match ausfuehren(&args) {
        Ok(meldung) => {
            println!("{meldung}");
            ExitCode::SUCCESS
        }
        Err(fehler) => {
            eprintln!("Fehler: {fehler}");
            ExitCode::FAILURE
        }
    }
}

fn ausfuehren(args: &[String]) -> Result<String, String> {
    if args.iter().any(|a| a.len() > MAX_ARG_LAENGE) {
        return Err("Ein Argument ist zu lang.".into());
    }
    match args.first().map(String::as_str) {
        Some("--version") => Ok(format!("lepaca {}", env!("CARGO_PKG_VERSION"))),
        Some("svg") => erzeugen(lies_optionen(&args[1..])?, Format::Svg),
        Some("pdf") => erzeugen(lies_optionen(&args[1..])?, Format::Pdf),
        _ => Ok(HILFE.into()),
    }
}

fn zahl(name: &str, wert: Option<&String>) -> Result<f64, String> {
    let wert = wert.ok_or_else(|| format!("Nach {name} fehlt ein Wert."))?;
    // Komma als Dezimaltrenner zulassen, wie im Deutschen üblich.
    let x: f64 = wert
        .replace(',', ".")
        .parse()
        .map_err(|_| format!("{name}: „{wert}“ ist keine Zahl."))?;
    if x.is_finite() {
        Ok(x)
    } else {
        Err(format!("{name}: „{wert}“ ist keine gültige Zahl."))
    }
}

fn lies_optionen(args: &[String]) -> Result<Optionen, String> {
    let mut o = Optionen::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--form" => o.form = it.next().cloned(),
            "--breite" => o.breite = Some(zahl(a, it.next())?),
            "--hoehe" => o.hoehe = Some(zahl(a, it.next())?),
            "--radius" => o.radius = Some(zahl(a, it.next())?),
            "--durchmesser" => o.durchmesser = Some(zahl(a, it.next())?),
            "--fluegel" => o.fluegel = Some(zahl(a, it.next())?),
            "--fluegelhoehe" => o.fluegelhoehe = Some(zahl(a, it.next())?),
            "--eisen" => o.eisen = it.next().cloned(),
            "--lochabstand" => o.lochabstand = Some(zahl(a, it.next())?),
            "--kantenabstand" => o.kantenabstand = Some(zahl(a, it.next())?),
            "--ecke" => o.ecke = it.next().cloned(),
            "--papier" => o.papier = it.next().cloned(),
            "--rand" => o.rand = Some(zahl(a, it.next())?),
            "--klebekante" => o.klebekante = Some(zahl(a, it.next())?),
            "--ausgabe" => o.ausgabe = it.next().map(PathBuf::from),
            "--ueberschreiben" => o.ueberschreiben = true,
            _ => return Err(format!("Unbekannte Option „{a}“. Hilfe: lepaca --hilfe")),
        }
    }
    Ok(o)
}

fn mm(name: &str, x: Option<f64>) -> Result<Mm, String> {
    let x = x.ok_or_else(|| format!("Bitte --{name} angeben."))?;
    Mm::neu(x).map_err(|e| format!("--{name}: {e}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Svg,
    Pdf,
}

fn erzeugen(o: Optionen, format: Format) -> Result<String, String> {
    let form = match o.form.as_deref().unwrap_or("rechteck") {
        "rechteck" => Form::Rechteck {
            breite: mm("breite", o.breite)?,
            hoehe: mm("hoehe", o.hoehe)?,
            eckenradius: mm("radius", Some(o.radius.unwrap_or(0.0)))?,
        },
        "kreis" => Form::Kreis {
            durchmesser: mm("durchmesser", o.durchmesser)?,
        },
        "ellipse" => Form::Ellipse {
            breite: mm("breite", o.breite)?,
            hoehe: mm("hoehe", o.hoehe)?,
        },
        "tpocket" => Form::TPocket {
            breite: mm("breite", o.breite)?,
            hoehe: mm("hoehe", o.hoehe)?,
            fluegel_ueberstand: mm("fluegel", Some(o.fluegel.unwrap_or(12.7)))?,
            fluegel_hoehe: mm("fluegelhoehe", Some(o.fluegelhoehe.unwrap_or(12.0)))?,
        },
        f => {
            return Err(format!(
                "Unbekannte Form „{f}“. Möglich: rechteck, kreis, ellipse, tpocket."
            ));
        }
    };
    let eisen = match o.eisen.as_deref().unwrap_or("diamond") {
        "french" | "franzoesisch" => Eisenart::Franzoesisch,
        "diamond" | "diamant" => Eisenart::Diamant,
        "japan" | "japanisch" => Eisenart::Japanisch,
        e => {
            return Err(format!(
                "Unbekannte Eisenart „{e}“. Möglich: french, diamond, japan."
            ));
        }
    };
    let ecke = match o.ecke.as_deref().unwrap_or("ein") {
        "ein" => Eckausrichtung::Einlaufend,
        "aus" => Eckausrichtung::Auslaufend,
        "mitte" => Eckausrichtung::Mitte,
        e => {
            return Err(format!(
                "Unbekannte Eckausrichtung „{e}“. Möglich: ein, aus, mitte."
            ));
        }
    };
    let einstellung = Einstellung {
        kantenabstand: mm("kantenabstand", Some(o.kantenabstand.unwrap_or(3.0)))?,
        lochabstand: mm("lochabstand", Some(o.lochabstand.unwrap_or(3.85)))?,
        ecke,
    };
    let ausgabe = o.ausgabe.ok_or("Bitte --ausgabe <datei.svg> angeben.")?;

    let kontur = form
        .kontur()
        .map_err(|e| format!("Die Form ist ungültig: {e:?}"))?;
    // T-Pockets werden nur an Flügeln und Steg genäht, nicht an der Öffnung.
    let reihe = if matches!(form, Form::TPocket { .. }) {
        stichreihe_an_kanten(&kontur, &TPocketNaehte::default().kanten(), einstellung)
    } else {
        stichreihe(&kontur, einstellung)
    }
    .map_err(|e| format!("Keine Stichreihe möglich: {e:?}"))?;
    let lochform = eisen.lochform(einstellung.lochabstand);
    let mut blaetter = String::new();
    let inhalt = match format {
        Format::Svg => lepaca_io::svg::teil(&kontur, &reihe, &lochform).into_bytes(),
        Format::Pdf => {
            let papier = match o.papier.as_deref().unwrap_or("a4") {
                "a4" => Papier::A4,
                "a3" => Papier::A3,
                "letter" => Papier::LETTER,
                p => {
                    return Err(format!(
                        "Unbekanntes Papierformat „{p}“. Möglich: a4, a3, letter."
                    ));
                }
            };
            let kachel_einstellung = KachelEinstellung {
                papier,
                rand: mm("rand", Some(o.rand.unwrap_or(10.0)))?,
                klebekante: mm("klebekante", Some(o.klebekante.unwrap_or(10.0)))?,
            };
            let (von, bis) = kontur.grenzen();
            let plan = kacheln(von, bis, kachel_einstellung)
                .map_err(|e| format!("Keine Aufteilung auf Blätter möglich: {e:?}"))?;
            blaetter = format!(
                "\n{} ({} × {}, {}) plus Prüfseite",
                if plan.kacheln.len() == 1 {
                    "1 Blatt".to_string()
                } else {
                    format!("{} Blätter", plan.kacheln.len())
                },
                plan.zeilen,
                plan.spalten,
                if plan.hochformat { "hoch" } else { "quer" }
            );
            lepaca_io::pdf::teil_gekachelt(&kontur, &reihe, &lochform, &plan)
        }
    };
    lepaca_io::datei::atomar_schreiben(&ausgabe, &inhalt, o.ueberschreiben).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            format!(
                "{} gibt es schon. Mit --ueberschreiben ersetzen.",
                ausgabe.display()
            )
        } else {
            format!("{} konnte nicht gespeichert werden: {e}", ausgabe.display())
        }
    })?;

    let mut meldung = format!(
        "{} Stichlöcher, gespeichert in {}{blaetter}",
        reihe.loecher.len(),
        ausgabe.display()
    );
    let soll = einstellung.lochabstand.wert();
    let abweichend: Vec<String> = reihe
        .abschnitte
        .iter()
        // Nur melden, was man in der Anzeige (zwei Nachkommastellen) sieht.
        .filter(|a| (a.lochabstand.wert() - einstellung.lochabstand.wert()).abs() >= 0.005)
        .map(|a| format!("{:.2}", a.lochabstand.wert()).replace('.', ","))
        .collect();
    if !abweichend.is_empty() {
        meldung.push_str(&format!(
            "\nLochabstand für gleichmäßige Verteilung angepasst (Soll {}): {} mm",
            format!("{soll:.2}").replace('.', ","),
            abweichend.join(" / ")
        ));
    }
    Ok(meldung)
}
