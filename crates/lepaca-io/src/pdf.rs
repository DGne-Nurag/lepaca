//! PDF-Export in Originalgröße, auf Blätter gekachelt.
//!
//! Das PDF wird ohne Bibliothek erzeugt: nur Vektorgrafik und die
//! eingebaute Schrift Helvetica, keine eingebetteten Dateien, kein
//! JavaScript. `PrintScaling /None` bittet Druckdialoge, nicht „an Seite
//! anzupassen". Zusätzlich enthält das PDF eine Prüfseite mit
//! 10-mm-Quadrat und 100-mm-Lineal.

use std::fmt::Write;

use lepaca_core::eisen::Lochform;
use lepaca_core::geometrie::{Kontur, Punkt};
use lepaca_core::kachelung::{self, Kachel, Kachelplan};
use lepaca_core::stichlinie::Stichreihe;

const PT_PRO_MM: f64 = 72.0 / 25.4;

/// Ein Teil, wie es gedruckt wird: Schnittkante und optional Stichreihe
/// mit Lochform. Koordinaten in Millimetern im gemeinsamen Muster.
#[derive(Debug, Clone, Copy)]
pub struct Druckteil<'a> {
    pub kontur: &'a Kontur,
    pub stiche: Option<(&'a Stichreihe, &'a Lochform)>,
}

/// Erzeugt das komplette PDF für ein einzelnes Teil.
pub fn teil_gekachelt(
    kontur: &Kontur,
    stiche: &Stichreihe,
    loch: &Lochform,
    plan: &Kachelplan,
) -> Vec<u8> {
    muster_gekachelt(
        &[Druckteil {
            kontur,
            stiche: Some((stiche, loch)),
        }],
        plan,
    )
}

/// Erzeugt das komplette PDF für ein Muster aus mehreren Teilen: ein Blatt
/// pro Kachel und eine Prüfseite.
pub fn muster_gekachelt(teile: &[Druckteil<'_>], plan: &Kachelplan) -> Vec<u8> {
    let gesamt = plan.kacheln.len() + 1;
    let mut seiten: Vec<String> = plan
        .kacheln
        .iter()
        .enumerate()
        .map(|(i, k)| kachelseite(teile, plan, k, i + 1, gesamt))
        .collect();
    seiten.push(pruefseite(plan, gesamt));
    dokument(&seiten, plan.blatt_breite, plan.blatt_hoehe)
}

/// Inhalt eines Blatts. Gezeichnet wird in Millimetern mit y nach unten;
/// die erste Transformation rechnet das in PDF-Punkte um.
fn kachelseite(
    teile: &[Druckteil<'_>],
    plan: &Kachelplan,
    k: &Kachel,
    nummer: usize,
    gesamt: usize,
) -> String {
    let r = plan.rand;
    let (nb, nh) = plan.nutzflaeche();
    let kl = plan.klebekante;
    let mut s = String::new();
    mm_koordinaten(&mut s, plan.blatt_hoehe);

    // Klebekanten grau hinterlegen.
    let _ = writeln!(s, "0.88 g");
    if k.klebekante_rechts {
        let _ = writeln!(s, "{} {} {} {} re f", z(r + nb - kl), z(r), z(kl), z(nh));
    }
    if k.klebekante_unten {
        let _ = writeln!(s, "{} {} {} {} re f", z(r), z(r + nh - kl), z(nb), z(kl));
    }

    // Muster, auf die Nutzfläche beschnitten.
    let _ = writeln!(s, "q {} {} {} {} re W n", z(r), z(r), z(nb), z(nh));
    let _ = writeln!(s, "1 0 0 1 {} {} cm", z(r - k.von.x), z(r - k.von.y));
    // Nur zeichnen, was auf diesem Blatt zu sehen ist; sonst trüge jedes
    // Blatt das ganze Muster und große Muster würden riesige Dateien.
    let sichtbar = |von: Punkt, bis: Punkt| {
        bis.x >= k.von.x && von.x <= k.von.x + nb && bis.y >= k.von.y && von.y <= k.von.y + nh
    };
    for teil in teile {
        let (von, bis) = teil.kontur.grenzen();
        if !sichtbar(von, bis) {
            continue;
        }
        let _ = writeln!(s, "0 G 0.25 w");
        pfad(&mut s, teil.kontur.punkte());
        let _ = writeln!(s, "s");
        if let Some((stiche, loch)) = teil.stiche {
            let _ = writeln!(s, "0.5 G 0.1 w [1 1] 0 d");
            for linie in &stiche.linien {
                linienzug(&mut s, &linie.punkte);
                // Geschlossen: schließen und zeichnen; offen: nur zeichnen.
                let _ = writeln!(s, "{}", if linie.geschlossen { "h S" } else { "S" });
            }
            let _ = writeln!(s, "[] 0 d 0 g");
            let halb = loch.laenge.max(loch.breite);
            for l in &stiche.loecher {
                let m = l.mitte;
                if !sichtbar(
                    Punkt::neu(m.x - halb, m.y - halb),
                    Punkt::neu(m.x + halb, m.y + halb),
                ) {
                    continue;
                }
                pfad(&mut s, &loch.umriss(l));
                let _ = writeln!(s, "f");
            }
        }
    }
    let _ = writeln!(s, "Q");

    // Rahmen der Nutzfläche = Schnittlinie für links/oben anschließende
    // Blätter. Passkreuze an den Enden der Klebekanten.
    let _ = writeln!(s, "0 G 0.15 w {} {} {} {} re S", z(r), z(r), z(nb), z(nh));
    if k.klebekante_rechts {
        passkreuz(&mut s, r + nb - kl, r);
        passkreuz(&mut s, r + nb - kl, r + nh);
    }
    if k.spalte > 0 {
        passkreuz(&mut s, r, r);
        passkreuz(&mut s, r, r + nh);
    }
    if k.klebekante_unten {
        passkreuz(&mut s, r, r + nh - kl);
        passkreuz(&mut s, r + nb, r + nh - kl);
    }
    if k.zeile > 0 {
        passkreuz(&mut s, r, r);
        passkreuz(&mut s, r + nb, r);
    }

    // Beschriftung im Rand.
    let kopf = format!(
        "Blatt {} ({} von {})   Maßstab 1:1 - beim Drucken \"Tatsächliche Größe\" wählen",
        k.name(),
        nummer,
        gesamt
    );
    text(&mut s, r, (r - 3.0).max(3.0), 9.0, &kopf);
    let mut hinweise = Vec::new();
    if k.spalte > 0 {
        hinweise.push(format!(
            "linken Rand abschneiden, auf {} kleben",
            kachelung::name(k.zeile, k.spalte - 1)
        ));
    }
    if k.zeile > 0 {
        hinweise.push(format!(
            "oberen Rand abschneiden, auf {} kleben",
            kachelung::name(k.zeile - 1, k.spalte)
        ));
    }
    if k.klebekante_rechts || k.klebekante_unten {
        hinweise.push("grauer Streifen = Klebekante".to_string());
    }
    if !hinweise.is_empty() {
        let unten = (r + nh + 4.0).min(plan.blatt_hoehe - 2.0);
        text(&mut s, r, unten, 8.0, &hinweise.join("  |  "));
    }
    s
}

/// Prüfseite: 10-mm-Quadrat, 100-mm-Lineal und Übersicht der Blätter.
fn pruefseite(plan: &Kachelplan, gesamt: usize) -> String {
    let mut s = String::new();
    mm_koordinaten(&mut s, plan.blatt_hoehe);
    let x = plan.rand.max(15.0);
    text(&mut s, x, 25.0, 14.0, "Prüfseite");
    text(
        &mut s,
        x,
        33.0,
        9.0,
        &format!("Seite {gesamt} von {gesamt}. Bitte nachmessen, bevor du zuschneidest."),
    );
    let _ = writeln!(s, "0 G 0.2 w {} 45 10 10 re S", z(x));
    text(&mut s, x + 14.0, 51.5, 9.0, "muss genau 10 x 10 mm messen");

    // Lineal 100 mm mit Millimeterstrichen.
    let _ = writeln!(s, "0.15 w");
    let y = 75.0;
    let _ = writeln!(s, "{} {} m {} {} l S", z(x), z(y), z(x + 100.0), z(y));
    for i in 0..=100 {
        let l = if i % 10 == 0 {
            5.0
        } else if i % 5 == 0 {
            3.5
        } else {
            2.0
        };
        let xi = x + i as f64;
        let _ = writeln!(s, "{} {} m {} {} l S", z(xi), z(y), z(xi), z(y - l));
        if i % 10 == 0 {
            text(&mut s, xi - 1.0, y + 4.0, 7.0, &format!("{}", i / 10));
        }
    }
    text(
        &mut s,
        x,
        y + 10.0,
        9.0,
        "Lineal in cm, muss genau 100 mm lang sein",
    );

    // Übersicht: Blätter als kleine Rechtecke.
    let oy = 105.0;
    text(&mut s, x, oy, 11.0, "Klebeplan");
    let zelle = 12.0;
    for k in &plan.kacheln {
        let kx = x + k.spalte as f64 * zelle;
        let ky = oy + 6.0 + k.zeile as f64 * zelle;
        if ky + zelle > plan.blatt_hoehe - 10.0 || kx + zelle > plan.blatt_breite - 10.0 {
            continue;
        }
        let _ = writeln!(s, "{} {} {} {} re S", z(kx), z(ky), z(zelle), z(zelle));
        text(&mut s, kx + 2.0, ky + 7.5, 7.0, &k.name());
    }
    s
}

fn mm_koordinaten(s: &mut String, blatt_hoehe: f64) {
    let f = z(PT_PRO_MM);
    let _ = writeln!(s, "{f} 0 0 -{f} 0 {} cm", z(blatt_hoehe * PT_PRO_MM));
    let _ = writeln!(s, "1 J 1 j");
}

fn pfad(s: &mut String, punkte: &[Punkt]) {
    linienzug(s, punkte);
    let _ = writeln!(s, "h");
}

/// Linienzug ohne ihn zu schließen.
fn linienzug(s: &mut String, punkte: &[Punkt]) {
    for (i, p) in punkte.iter().enumerate() {
        let _ = writeln!(
            s,
            "{} {} {}",
            z(p.x),
            z(p.y),
            if i == 0 { "m" } else { "l" }
        );
    }
}

fn passkreuz(s: &mut String, x: f64, y: f64) {
    let a = 3.0;
    let _ = writeln!(
        s,
        "0 G 0.15 w {} {} m {} {} l {} {} m {} {} l S",
        z(x - a),
        z(y),
        z(x + a),
        z(y),
        z(x),
        z(y - a),
        z(x),
        z(y + a)
    );
}

/// Text an (x, y) in mm, Schriftgröße in pt. Die Textmatrix dreht die
/// y-Spiegelung der Seite für die Schrift wieder um.
fn text(s: &mut String, x: f64, y: f64, groesse_pt: f64, inhalt: &str) {
    let g = z(groesse_pt / PT_PRO_MM);
    let _ = writeln!(
        s,
        "0 g BT /F1 1 Tf {g} 0 0 -{g} {} {} Tm ({}) Tj ET",
        z(x),
        z(y),
        pdf_text(inhalt)
    );
}

/// Text für einen PDF-String in WinAnsi-Kodierung. Zeichen außerhalb von
/// Latin-1 werden durch „?" ersetzt; Klammern und Backslash maskiert.
fn pdf_text(t: &str) -> String {
    let mut aus = String::new();
    for c in t.chars() {
        match c {
            '(' | ')' | '\\' => {
                aus.push('\\');
                aus.push(c);
            }
            ' '..='~' => aus.push(c),
            c if (c as u32) >= 0xA0 && (c as u32) <= 0xFF => {
                let _ = write!(aus, "\\{:03o}", c as u32);
            }
            _ => aus.push('?'),
        }
    }
    aus
}

/// Zahl mit höchstens vier Nachkommastellen, Punkt als Trenner.
fn z(x: f64) -> String {
    let s = format!("{x:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Setzt die Objekte zu einer PDF-Datei mit Querverweistabelle zusammen.
fn dokument(seiten: &[String], breite_mm: f64, hoehe_mm: f64) -> Vec<u8> {
    let b = z(breite_mm * PT_PRO_MM);
    let h = z(hoehe_mm * PT_PRO_MM);
    // Objekte: 1 Katalog, 2 Seitenbaum, 3 Schrift, dann je Seite Seite + Inhalt.
    let mut objekte: Vec<String> = Vec::new();
    objekte.push(
        "<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << /PrintScaling /None >> >>".into(),
    );
    let kinder: Vec<String> = (0..seiten.len())
        .map(|i| format!("{} 0 R", 4 + 2 * i))
        .collect();
    objekte.push(format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kinder.join(" "),
        seiten.len()
    ));
    objekte.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into(),
    );
    for (i, inhalt) in seiten.iter().enumerate() {
        objekte.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {b} {h}] \
             /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            5 + 2 * i
        ));
        objekte.push(format!(
            "<< /Length {} >>\nstream\n{}endstream",
            inhalt.len(),
            inhalt
        ));
    }

    let mut pdf: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut positionen = Vec::with_capacity(objekte.len());
    for (i, o) in objekte.iter().enumerate() {
        positionen.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, o).as_bytes());
    }
    let xref = pdf.len();
    let mut tabelle = format!("xref\n0 {}\n0000000000 65535 f \n", objekte.len() + 1);
    for p in positionen {
        let _ = writeln!(tabelle, "{p:010} 00000 n ");
    }
    let _ = write!(
        tabelle,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objekte.len() + 1
    );
    pdf.extend_from_slice(tabelle.as_bytes());
    pdf
}

#[cfg(test)]
mod tests {
    use super::*;
    use lepaca_core::eisen::Eisenart;
    use lepaca_core::form::Form;
    use lepaca_core::kachelung::{Einstellung as KachelEinstellung, kacheln};
    use lepaca_core::mass::Mm;
    use lepaca_core::stichlinie::{Einstellung, stichreihe};

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    fn beispiel(b: f64, h: f64) -> (Vec<u8>, Kachelplan) {
        let k = Form::Rechteck {
            breite: mm(b),
            hoehe: mm(h),
            eckenradius: mm(5.0),
        }
        .kontur()
        .unwrap();
        let e = Einstellung {
            kantenabstand: mm(3.0),
            lochabstand: mm(3.85),
            ecke: Default::default(),
        };
        let r = stichreihe(&k, e).unwrap();
        let (von, bis) = k.grenzen();
        let plan = kacheln(von, bis, KachelEinstellung::default()).unwrap();
        let pdf = teil_gekachelt(&k, &r, &Eisenart::Diamant.lochform(e.lochabstand), &plan);
        (pdf, plan)
    }

    fn als_text(pdf: &[u8]) -> String {
        String::from_utf8_lossy(pdf).into_owned()
    }

    #[test]
    fn ein_a4_blatt_ist_im_pdf_genau_210_x_297_mm_gross() {
        let (pdf, _) = beispiel(100.0, 60.0);
        // 210 mm = 595,2756 pt, 297 mm = 841,8898 pt.
        assert!(als_text(&pdf).contains("/MediaBox [0 0 595.2756 841.8898]"));
    }

    #[test]
    fn das_pdf_bittet_den_drucker_nicht_zu_skalieren() {
        let (pdf, _) = beispiel(100.0, 60.0);
        assert!(als_text(&pdf).contains("/PrintScaling /None"));
    }

    #[test]
    fn jedes_blatt_und_die_pruefseite_sind_im_pdf() {
        let (pdf, plan) = beispiel(400.0, 300.0);
        let t = als_text(&pdf);
        assert!(t.contains(&format!("/Count {}", plan.kacheln.len() + 1)));
        assert_eq!(t.matches("/Type /Page ").count(), plan.kacheln.len() + 1);
    }

    #[test]
    fn die_querverweistabelle_zeigt_auf_die_richtigen_objekte() {
        let (pdf, _) = beispiel(400.0, 300.0);
        // Byte-Positionen prüfen; der Kopf enthält absichtlich Nicht-ASCII.
        let suche = |nadel: &[u8]| pdf.windows(nadel.len()).rposition(|w| w == nadel);
        let s = suche(b"startxref\n").unwrap() + b"startxref\n".len();
        let ende = s + pdf[s..].iter().position(|c| *c == b'\n').unwrap();
        let start: usize = std::str::from_utf8(&pdf[s..ende]).unwrap().parse().unwrap();
        assert!(pdf[start..].starts_with(b"xref\n"));
        let tabelle = std::str::from_utf8(&pdf[start..]).unwrap();
        let mut anzahl = 0;
        for (nr, zeile) in tabelle
            .lines()
            .skip(3)
            .take_while(|z| z.ends_with(" n "))
            .enumerate()
        {
            let pos: usize = zeile[..10].parse().unwrap();
            assert!(pdf[pos..].starts_with(format!("{} 0 obj", nr + 1).as_bytes()));
            anzahl += 1;
        }
        assert!(anzahl > 3);
    }

    #[test]
    fn jedes_blatt_traegt_nur_die_stichloecher_die_darauf_liegen() {
        let (pdf, plan) = beispiel(400.0, 300.0);
        let t = als_text(&pdf);
        // Jedes Loch ist ein gefülltes Viereck, das mit „f“ endet. Über alle
        // Blätter zusammen kommt jedes Loch nur einmal vor, außer auf den
        // Klebekanten, wo sich die Blätter überlappen.
        let loecher_im_pdf = t.lines().filter(|z| *z == "f").count();
        let loecher_im_teil = (2.0 * (400.0 + 300.0) / 3.85) as usize; // grob
        assert!(plan.kacheln.len() >= 4);
        assert!(
            loecher_im_pdf < loecher_im_teil * 2,
            "{loecher_im_pdf} Löcher im PDF bei etwa {loecher_im_teil} im Teil"
        );
    }

    #[test]
    fn umlaute_und_klammern_werden_fuer_pdf_maskiert() {
        assert_eq!(pdf_text("Prüfseite (1)"), "Pr\\374fseite \\(1\\)");
        assert_eq!(pdf_text("→"), "?");
    }
}
