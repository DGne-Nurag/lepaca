//! Kommandozeile für lepaca.
//!
//! Beispiel:
//! `lepaca svg --form rechteck --breite 80 --hoehe 50 --radius 5
//!  --eisen diamant --lochabstand 3.85 --kantenabstand 3 --ausgabe teil.svg`

use std::path::PathBuf;
use std::process::ExitCode;

use lepaca_core::eisen::Eisenart;
use lepaca_core::form::Form;
use lepaca_core::mass::Mm;
use lepaca_core::stichlinie::{Eckausrichtung, Einstellung, stichreihe};

const HILFE: &str = "\
lepaca – Leather Pattern CAD

Aufruf:
  lepaca svg [Optionen]

Optionen (alle Maße in Millimetern):
  --form rechteck|kreis|ellipse   Grundform (Standard: rechteck)
  --breite <mm>                   Breite (Rechteck, Ellipse)
  --hoehe <mm>                    Höhe (Rechteck, Ellipse)
  --radius <mm>                   Eckenradius (Rechteck, Standard: 0)
  --durchmesser <mm>              Durchmesser (Kreis)
  --eisen french|diamond|japan    Eisenart (Standard: diamond)
  --lochabstand <mm>              Zinkenabstand (Standard: 3.85)
  --kantenabstand <mm>            Abstand der Naht zur Kante (Standard: 3)
  --ecke ein|aus|mitte            Ausrichtung des Ecklochs (Standard: ein,
                                  wie die einlaufende Kante)
  --ausgabe <datei.svg>           Zieldatei
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
    eisen: Option<String>,
    lochabstand: Option<f64>,
    kantenabstand: Option<f64>,
    ecke: Option<String>,
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
        Some("svg") => svg(lies_optionen(&args[1..])?),
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
            "--eisen" => o.eisen = it.next().cloned(),
            "--lochabstand" => o.lochabstand = Some(zahl(a, it.next())?),
            "--kantenabstand" => o.kantenabstand = Some(zahl(a, it.next())?),
            "--ecke" => o.ecke = it.next().cloned(),
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

fn svg(o: Optionen) -> Result<String, String> {
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
        f => {
            return Err(format!(
                "Unbekannte Form „{f}“. Möglich: rechteck, kreis, ellipse."
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
    let reihe =
        stichreihe(&kontur, einstellung).map_err(|e| format!("Keine Stichreihe möglich: {e:?}"))?;
    let svg = lepaca_io::svg::teil(&kontur, &reihe, &eisen.lochform(einstellung.lochabstand));
    lepaca_io::datei::atomar_schreiben(&ausgabe, svg.as_bytes(), o.ueberschreiben).map_err(
        |e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "{} gibt es schon. Mit --ueberschreiben ersetzen.",
                    ausgabe.display()
                )
            } else {
                format!("{} konnte nicht gespeichert werden: {e}", ausgabe.display())
            }
        },
    )?;

    let mut meldung = format!(
        "{} Stichlöcher, gespeichert in {}",
        reihe.loecher.len(),
        ausgabe.display()
    );
    let soll = einstellung.lochabstand.wert();
    let abweichend: Vec<String> = reihe
        .abschnitte
        .iter()
        .filter(|a| !a.lochabstand.etwa_gleich(einstellung.lochabstand))
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
