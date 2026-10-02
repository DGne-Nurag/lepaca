//! SVG-Export in Originalgröße.
//!
//! Breite und Höhe stehen in Millimetern im SVG, die `viewBox` ebenfalls in
//! Millimetern. So druckt jedes Programm, das SVG korrekt umsetzt, 1:1.

use std::fmt::Write;

use lepaca_core::eisen::Lochform;
use lepaca_core::geometrie::{Kontur, Punkt};
use lepaca_core::stichlinie::Stichreihe;

/// Rand um das Teil.
pub const RAND_MM: f64 = 5.0;
/// Seitenlänge des Prüfquadrats zum Nachmessen nach dem Druck.
pub const PRUEFQUADRAT_MM: f64 = 10.0;

/// Erzeugt ein SVG mit Schnittkante, Stichlinie, Löchern und Prüfquadrat.
pub fn teil(kontur: &Kontur, stiche: &Stichreihe, loch: &Lochform) -> String {
    let (min, max) = kontur.grenzen();
    let versatz = Punkt::neu(RAND_MM - min.x, RAND_MM - min.y);
    let breite = max.x - min.x + 2.0 * RAND_MM;
    // Unter dem Teil Platz für das Prüfquadrat.
    let hoehe = max.y - min.y + 3.0 * RAND_MM + PRUEFQUADRAT_MM;

    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{b}mm" height="{h}mm" viewBox="0 0 {b} {h}">
<title>lepaca Teil</title>"#,
        b = zahl(breite),
        h = zahl(hoehe)
    );
    let _ = writeln!(
        s,
        r##"<path d="{}" fill="none" stroke="#000" stroke-width="0.25"/>"##,
        pfad(kontur.punkte(), versatz)
    );
    let _ = writeln!(
        s,
        r##"<path d="{}" fill="none" stroke="#888" stroke-width="0.1" stroke-dasharray="1 1"/>"##,
        pfad(stiche.linie.punkte(), versatz)
    );
    let _ = writeln!(s, r##"<g fill="#000">"##);
    for l in &stiche.loecher {
        let u = loch.umriss(l);
        let _ = writeln!(s, r#"<path d="{}"/>"#, pfad(&u, versatz));
    }
    let _ = writeln!(s, "</g>");

    let y = max.y - min.y + 2.0 * RAND_MM;
    let q = zahl(PRUEFQUADRAT_MM);
    let _ = writeln!(
        s,
        r##"<rect x="{x}" y="{y}" width="{q}" height="{q}" fill="none" stroke="#000" stroke-width="0.2"/>
<text x="{tx}" y="{ty}" font-family="sans-serif" font-size="3">10 mm Prüfquadrat</text>
</svg>"##,
        x = zahl(RAND_MM),
        y = zahl(y),
        tx = zahl(RAND_MM + PRUEFQUADRAT_MM + 2.0),
        ty = zahl(y + PRUEFQUADRAT_MM / 2.0 + 1.0),
    );
    s
}

fn pfad(punkte: &[Punkt], versatz: Punkt) -> String {
    let mut d = String::new();
    for (i, p) in punkte.iter().enumerate() {
        let p = p.plus(versatz);
        let _ = write!(
            d,
            "{}{} {} ",
            if i == 0 { "M" } else { "L" },
            zahl(p.x),
            zahl(p.y)
        );
    }
    d.push('Z');
    d
}

/// Zahl mit drei Nachkommastellen und Punkt als Trenner, unabhängig von der
/// Spracheinstellung des Systems.
fn zahl(x: f64) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lepaca_core::eisen::Eisenart;
    use lepaca_core::form::Form;
    use lepaca_core::mass::Mm;
    use lepaca_core::stichlinie::{Einstellung, stichreihe};

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    fn beispiel() -> String {
        let k = Form::Rechteck {
            breite: mm(80.0),
            hoehe: mm(50.0),
            eckenradius: mm(0.0),
        }
        .kontur()
        .unwrap();
        let e = Einstellung {
            kantenabstand: mm(3.0),
            lochabstand: mm(4.0),
        };
        let r = stichreihe(&k, e).unwrap();
        teil(&k, &r, &Eisenart::Diamant.lochform(e.lochabstand))
    }

    #[test]
    fn das_svg_hat_seine_groesse_in_millimetern() {
        let s = beispiel();
        // 80 + 2·5 breit, 50 + 3·5 + 10 hoch.
        assert!(s.contains(r#"width="90mm" height="75mm" viewBox="0 0 90 75""#));
    }

    #[test]
    fn jedes_loch_ist_im_svg() {
        let s = beispiel();
        let loecher = s
            .lines()
            .filter(|z| z.starts_with("<path d=\"M") && z.ends_with("Z\"/>"))
            .count();
        // 2 × (19 + 11) Abstände auf 74 × 44 mm Stichlinie.
        assert_eq!(loecher, 60);
    }

    #[test]
    fn zahlen_haben_immer_einen_punkt_als_dezimaltrenner() {
        assert_eq!(zahl(1.23456), "1.235");
        assert_eq!(zahl(10.0), "10");
        assert_eq!(zahl(-0.0001), "0");
    }
}
