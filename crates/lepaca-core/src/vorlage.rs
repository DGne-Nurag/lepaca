//! Fertige Muster als Vorlage.
//!
//! Die Vorlagen sind mit denselben Werkzeugen gebaut, die auch in der
//! Oberfläche zur Verfügung stehen: freie Skizzen mit Maßen und
//! Bedingungen, Kantenwahl für die Naht und der Zusammenbau mit
//! Nähschritten. Sie dienen damit auch als Test dieser Werkzeuge.

use crate::eisen::Eisenart;
use crate::form::{Form, Pfad, PfadElement, TPocketNaehte};
use crate::geometrie::Punkt;
use crate::mass::Mm;
use crate::muster::{Montage, Musterteil, Naht, Nahtkanten, Teil, TeilForm};
use crate::skizze::{self, Bedingung, SkizzenFehler};
use crate::stichlinie::{Eckausrichtung, Einstellung, StichFehler, Stichloch};

/// Eine Vorlage: Teile und die Nähanleitung in Schritten.
#[derive(Debug, Clone, PartialEq)]
pub struct Vorlage {
    pub teile: Vec<Musterteil>,
    pub schritte: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VorlagenFehler {
    Skizze(SkizzenFehler),
    Stich(StichFehler),
    /// Die Skizze ist nicht vollständig bestimmt.
    Unbestimmt,
}

impl From<SkizzenFehler> for VorlagenFehler {
    fn from(f: SkizzenFehler) -> Self {
        VorlagenFehler::Skizze(f)
    }
}

impl From<StichFehler> for VorlagenFehler {
    fn from(f: StichFehler) -> Self {
        VorlagenFehler::Stich(f)
    }
}

fn mm(x: f64) -> Mm {
    // Nur mit endlichen Werten aufgerufen; null fällt bei der Prüfung auf.
    Mm::neu(x).unwrap_or(Mm::NULL)
}

fn p(x: f64, y: f64) -> Punkt {
    Punkt::neu(x, y)
}

/// Maße der Bifold-Geldbörse in Millimetern.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BifoldMasse {
    /// Breite aufgeklappt (zugeklappt die Hälfte).
    pub breite: f64,
    pub hoehe: f64,
    /// Eckenradius außen.
    pub eckenradius: f64,
    /// Kantenabstand aller Nähte.
    pub kantenabstand: f64,
    /// Zinkenabstand des Eisens.
    pub lochabstand: f64,
    /// Gewünschter Abstand der Innennaht (Kartenfächer zur Mitte hin) von
    /// der Außenkante; genommen wird das nächste Loch der Randnaht.
    pub innennaht_ab: f64,
    /// Gewünschte Oberkante des hintersten Kartenfachs.
    pub erstes_fach: f64,
    /// Höhe der T-Fächer von der Öffnung bis zur Unterkante des Stegs.
    pub fachhoehe: f64,
    /// Flügelüberstand der T-Fächer je Seite.
    pub fluegel: f64,
    /// Versatz von Fach zu Fach in Löchern der Randnaht.
    pub versatz_loecher: usize,
}

impl Default for BifoldMasse {
    fn default() -> Self {
        BifoldMasse {
            // Breite, Höhe und Radius so gewählt, dass die Löcher von Rand-
            // und Innennaht im gleichen Takt laufen: Dann bleibt an jeder
            // Fachkante gut 1,9 mm Platz zum nächsten Loch.
            breite: 217.0,
            hoehe: 93.0,
            eckenradius: 6.0,
            kantenabstand: 3.0,
            lochabstand: 3.85,
            innennaht_ab: 96.0,
            erstes_fach: 16.0,
            fachhoehe: 46.0,
            fluegel: 10.0,
            versatz_loecher: 4,
        }
    }
}

fn naht(m: &BifoldMasse, kanten: Nahtkanten) -> Naht {
    Naht {
        einstellung: Einstellung {
            kantenabstand: mm(m.kantenabstand),
            lochabstand: mm(m.lochabstand),
            ecke: Eckausrichtung::Einlaufend,
        },
        eisen: Eisenart::Diamant,
        kanten,
    }
}

fn bogen(ziel: Punkt, r: f64) -> PfadElement {
    PfadElement::Bogen {
        ziel,
        radius: mm(r),
        uhrzeigersinn: true,
        gross: false,
    }
}

/// Löst eine Skizze und verlangt, dass sie vollständig bestimmt ist.
fn bestimmt(pfad: &Pfad, geschlossen: bool, b: &[Bedingung]) -> Result<Pfad, VorlagenFehler> {
    let e = skizze::loesen(pfad, geschlossen, b)?;
    if e.freiheitsgrade != 0 {
        return Err(VorlagenFehler::Unbestimmt);
    }
    Ok(e.pfad)
}

/// Außenleder bzw. Futter: Rechteck mit vier gleichen, tangentialen
/// Eckenbögen. Bewusst grob „von Hand" vorgezeichnet; die Maße machen es
/// genau.
fn aussenform(m: &BifoldMasse) -> Result<(Pfad, Vec<Bedingung>), VorlagenFehler> {
    let (b, h, r) = (m.breite, m.hoehe, m.eckenradius);
    let skizze = Pfad {
        start: p(r, 0.0),
        elemente: vec![
            PfadElement::Linie(p(b - r + 0.8, 0.6)),
            bogen(p(b + 0.4, r + 1.0), r + 0.5),
            PfadElement::Linie(p(b - 0.7, h - r - 0.9)),
            bogen(p(b - r - 0.3, h + 0.5), r - 0.4),
            PfadElement::Linie(p(r + 1.1, h - 0.6)),
            bogen(p(0.6, h - r + 0.2), r + 0.3),
            PfadElement::Linie(p(-0.5, r + 0.8)),
            bogen(p(r, 0.0), r),
        ],
    };
    // Punkte: 0 Start oben links (Ende des Eckbogens), 1 oben rechts,
    // 2 rechts oben, 3 rechts unten, 4 unten rechts, 5 unten links,
    // 6 links unten, 7 links oben.
    let mut bed = vec![
        Bedingung::Horizontal { kante: 0 },
        Bedingung::Vertikal { kante: 2 },
        Bedingung::Horizontal { kante: 4 },
        Bedingung::Vertikal { kante: 6 },
        Bedingung::Radius {
            kante: 1,
            wert: mm(r),
        },
        Bedingung::Gleich { a: 1, b: 3 },
        Bedingung::Gleich { a: 1, b: 5 },
        Bedingung::Gleich { a: 1, b: 7 },
        Bedingung::AbstandX {
            a: 7,
            b: 2,
            wert: mm(b),
        },
        Bedingung::AbstandY {
            a: 1,
            b: 4,
            wert: mm(h),
        },
    ];
    for k in 0..8 {
        bed.push(Bedingung::Tangential {
            a: k,
            b: (k + 1) % 8,
        });
    }
    let pfad = bestimmt(&skizze, true, &bed)?;
    Ok((pfad, bed))
}

/// Vorderstes Kartenfach: reicht bis zur Unterkante und hat dort die
/// gleiche Rundung wie das Außenleder. `links`: Rundung unten links.
fn vorderfach(
    m: &BifoldMasse,
    breite: f64,
    hoehe: f64,
    links: bool,
) -> Result<(Pfad, Vec<Bedingung>), VorlagenFehler> {
    let r = m.eckenradius;
    let (skizze, bed) = if links {
        (
            Pfad {
                start: p(0.0, 0.0),
                elemente: vec![
                    PfadElement::Linie(p(breite + 0.6, 0.4)),
                    PfadElement::Linie(p(breite - 0.3, hoehe + 0.5)),
                    PfadElement::Linie(p(r + 0.7, hoehe - 0.2)),
                    bogen(p(0.3, hoehe - r), r + 0.6),
                    PfadElement::Linie(p(0.0, 0.0)),
                ],
            },
            vec![
                Bedingung::Horizontal { kante: 0 },
                Bedingung::Vertikal { kante: 1 },
                Bedingung::Horizontal { kante: 2 },
                Bedingung::Vertikal { kante: 4 },
                Bedingung::Tangential { a: 2, b: 3 },
                Bedingung::Tangential { a: 3, b: 4 },
                Bedingung::Radius {
                    kante: 3,
                    wert: mm(r),
                },
                Bedingung::AbstandX {
                    a: 0,
                    b: 1,
                    wert: mm(breite),
                },
                Bedingung::AbstandY {
                    a: 0,
                    b: 2,
                    wert: mm(hoehe),
                },
            ],
        )
    } else {
        (
            Pfad {
                start: p(0.0, 0.0),
                elemente: vec![
                    PfadElement::Linie(p(breite - 0.5, -0.3)),
                    PfadElement::Linie(p(breite + 0.2, hoehe - r + 0.4)),
                    bogen(p(breite - r - 0.4, hoehe + 0.3), r - 0.3),
                    PfadElement::Linie(p(0.4, hoehe - 0.6)),
                    PfadElement::Linie(p(0.0, 0.0)),
                ],
            },
            vec![
                Bedingung::Horizontal { kante: 0 },
                Bedingung::Vertikal { kante: 1 },
                Bedingung::Horizontal { kante: 3 },
                Bedingung::Vertikal { kante: 4 },
                Bedingung::Tangential { a: 1, b: 2 },
                Bedingung::Tangential { a: 2, b: 3 },
                Bedingung::Radius {
                    kante: 2,
                    wert: mm(r),
                },
                Bedingung::AbstandX {
                    a: 0,
                    b: 1,
                    wert: mm(breite),
                },
                Bedingung::AbstandY {
                    a: 0,
                    b: 4,
                    wert: mm(hoehe),
                },
            ],
        )
    };
    let pfad = bestimmt(&skizze, true, &bed)?;
    Ok((pfad, bed))
}

/// Gerade, senkrechte Naht (offene Linie) der Länge `laenge`.
fn senkrechte_naht(laenge: f64) -> Result<(Pfad, Vec<Bedingung>), VorlagenFehler> {
    let skizze = Pfad {
        start: p(0.0, 0.0),
        elemente: vec![PfadElement::Linie(p(0.4, laenge + 0.5))],
    };
    let bed = vec![
        Bedingung::Vertikal { kante: 0 },
        Bedingung::Laenge {
            kante: 0,
            wert: mm(laenge),
        },
    ];
    let pfad = bestimmt(&skizze, false, &bed)?;
    Ok((pfad, bed))
}

/// Die wichtigsten Maße der Bifold-Geldbörse, wie sie aus den Löchern der
/// Randnaht abgeleitet werden.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BifoldAbgeleitet {
    /// Tatsächlicher Lochabstand der Randnaht.
    pub lochabstand: f64,
    /// x der linken Innennaht (rechte: Breite − x).
    pub innennaht_x: f64,
    /// Oberkanten der Fächer: T-Fach 1, T-Fach 2, Vorderfach.
    pub faecher_y: [f64; 3],
    /// Kleinster Abstand einer Fachoberkante zu einem Loch.
    pub freiraum: f64,
    /// Oberstes Loch der Innennaht.
    pub innennaht_y: f64,
}

/// Leitet Innennaht und Fachkanten aus der Randnaht ab: Die Innennaht
/// endet unten in einem Loch der Randnaht, und jede Fachoberkante liegt
/// mittig zwischen den Löchern von Rand- und Innennaht.
fn ableiten(m: &BifoldMasse, aussen: &Teil) -> Result<BifoldAbgeleitet, VorlagenFehler> {
    let u = aussen
        .umriss()
        .map_err(|_| VorlagenFehler::Stich(StichFehler::KantenUnpassend))?;
    let (reihe, _) = aussen
        .stiche(&u)
        .ok_or(VorlagenFehler::Stich(StichFehler::KantenUnpassend))??;
    let ka = m.kantenabstand;
    let teilung = reihe
        .abschnitte
        .first()
        .map(|a| a.lochabstand.wert())
        .ok_or(VorlagenFehler::Stich(StichFehler::KantenUnpassend))?;
    let unten = m.hoehe - ka;
    let innennaht_x = reihe
        .loecher
        .iter()
        .map(|l| l.mitte)
        .filter(|q| (q.y - unten).abs() < 1e-6 && q.x >= m.innennaht_ab)
        .map(|q| q.x)
        .fold(f64::INFINITY, f64::min);
    let seite: Vec<f64> = reihe
        .loecher
        .iter()
        .map(|l| l.mitte)
        .filter(|q| (q.x - ka).abs() < 1e-6)
        .map(|q| q.y)
        .collect();
    let innen = |y: f64| {
        // Löcher der Innennaht liegen bei unten − j·teilung.
        let j = ((unten - y) / teilung).round();
        (unten - j * teilung - y).abs()
    };
    let versatz = m.versatz_loecher as f64 * teilung;
    let freiraum = |y: f64| {
        (0..3)
            .map(|k| y + k as f64 * versatz)
            .map(|yk| {
                let s = seite
                    .iter()
                    .map(|h| (h - yk).abs())
                    .fold(f64::INFINITY, f64::min);
                s.min(innen(yk))
            })
            .fold(f64::INFINITY, f64::min)
    };
    // Im Umkreis einer Teilung um die gewünschte Höhe die Stelle mit dem
    // meisten Abstand zu allen Löchern suchen.
    let mut best = (m.erstes_fach, f64::NEG_INFINITY);
    let schritte = 2000;
    for i in 0..=schritte {
        let y = m.erstes_fach - teilung / 2.0 + teilung * i as f64 / schritte as f64;
        let f = freiraum(y);
        if f > best.1 + 1e-12 {
            best = (y, f);
        }
    }
    let y1 = best.0;
    // Oberstes Loch der Innennaht: das erste unter der Oberkante des
    // hintersten Fachs.
    let j = ((unten - y1) / teilung).floor();
    Ok(BifoldAbgeleitet {
        lochabstand: teilung,
        innennaht_x,
        faecher_y: [y1, y1 + versatz, y1 + 2.0 * versatz],
        freiraum: best.1,
        innennaht_y: unten - j * teilung,
    })
}

/// Breite der Kartenfächer bis zur inneren Kante. Die Vorderfächer reichen
/// bis in die Randnaht unten; ihre innere Kante muss deshalb vor dem
/// nächsten Loch der Randnaht enden, mit mindestens 0,6 mm Luft zum
/// Lochrand. Höchstens aber Kantenabstand hinter der Innennaht.
fn fachbreite(m: &BifoldMasse, a: &BifoldAbgeleitet) -> f64 {
    let loch = Eisenart::Diamant
        .lochform(mm(m.lochabstand))
        .umriss(&Stichloch {
            mitte: p(0.0, 0.0),
            richtung: p(1.0, 0.0),
            auf_ecke: false,
        });
    let halbe = loch.iter().map(|q| q.x.abs()).fold(0.0, f64::max);
    a.innennaht_x + (a.lochabstand - halbe - 0.6).min(m.kantenabstand)
}

/// Bifold-Geldbörse mit 2 × 3 Kartenfächern und Scheinfach.
///
/// Aufbau (von hinten nach vorn): Außenleder, Futter, je Seite zwei
/// T-Fächer und ein Vorderfach. Das Scheinfach ist der Raum zwischen
/// Außenleder und Futter, oben offen.
pub fn bifold() -> Result<Vorlage, VorlagenFehler> {
    bifold_mit(BifoldMasse::default())
}

pub fn bifold_mit(m: BifoldMasse) -> Result<Vorlage, VorlagenFehler> {
    let (b, h, ka) = (m.breite, m.hoehe, m.kantenabstand);
    let (aussen_pfad, aussen_bed) = aussenform(&m)?;
    // Randnaht: alles außer der Oberkante (dort ist das Scheinfach offen).
    let mut randnaht = vec![true; aussen_pfad.kantenzahl(true)];
    randnaht[0] = false;

    // Platz zwischen den Teilen, damit die Maße nicht ineinanderlaufen.
    let zwischenraum = 40.0;
    let aussen = Teil {
        form: TeilForm::Geschlossen(Form::Pfad(aussen_pfad.clone())),
        position: p(0.0, 0.0),
        naht: Some(naht(&m, Nahtkanten::Pfad(randnaht))),
        montage: Some(Montage {
            position: p(0.0, 0.0),
            schritt: 5,
        }),
    };
    let a = ableiten(&m, &aussen)?;
    let fach_breite = fachbreite(&m, &a);
    let fluegel_hoehe = m.versatz_loecher as f64 * a.lochabstand;
    let vorder_hoehe = h - a.faecher_y[2];

    let mut teile = vec![Musterteil {
        name: "Außenleder".into(),
        teil: aussen,
        bedingungen: aussen_bed.clone(),
    }];

    // Futter: gleicher Umriss, keine eigene Naht; es bekommt die Löcher
    // aller Nähte, die durch es hindurchgehen.
    let futter_pos = p(0.0, h + zwischenraum);
    teile.push(Musterteil {
        name: "Futter".into(),
        teil: Teil {
            form: TeilForm::Geschlossen(Form::Pfad(aussen_pfad)),
            position: futter_pos,
            naht: None,
            montage: Some(Montage {
                position: p(0.0, 0.0),
                schritt: 0,
            }),
        },
        bedingungen: aussen_bed,
    });

    // T-Fächer: nur der Steg wird eigens genäht (verdeckt vom nächsten
    // Fach); die Flügel laufen in Rand- und Innennaht mit.
    let reihe_y = 2.0 * (h + zwischenraum);
    let t_fach = Form::TPocket {
        breite: mm(fach_breite),
        hoehe: mm(m.fachhoehe),
        fluegel_ueberstand: mm(m.fluegel),
        fluegel_hoehe: mm(fluegel_hoehe),
    };
    let steg = TPocketNaehte {
        fluegel: false,
        steg_unten: true,
        oeffnung: false,
    };
    for (k, &y) in a.faecher_y[..2].iter().enumerate() {
        for (seite, x_muster, x_montage) in [
            ("links", 0.0, 0.0),
            ("rechts", fach_breite + zwischenraum, b - fach_breite),
        ] {
            teile.push(Musterteil {
                name: format!("Kartenfach {} {seite}", k + 1),
                teil: Teil {
                    form: TeilForm::Geschlossen(t_fach.clone()),
                    position: p(x_muster, reihe_y + k as f64 * (m.fachhoehe + zwischenraum)),
                    naht: Some(naht(&m, Nahtkanten::TPocket(steg))),
                    montage: Some(Montage {
                        position: p(x_montage, y),
                        schritt: k as u32 + 1,
                    }),
                },
                bedingungen: Vec::new(),
            });
        }
    }

    // Vorderfächer: ohne eigene Naht, alle Löcher kommen von Rand- und
    // Innennaht.
    let vorder_y = reihe_y + 2.0 * (m.fachhoehe + zwischenraum);
    for (seite, links, x_muster, x_montage) in [
        ("links", true, 0.0, 0.0),
        ("rechts", false, fach_breite + zwischenraum, b - fach_breite),
    ] {
        let (pfad, bed) = vorderfach(&m, fach_breite, vorder_hoehe, links)?;
        teile.push(Musterteil {
            name: format!("Vorderfach {seite}"),
            teil: Teil {
                form: TeilForm::Geschlossen(Form::Pfad(pfad)),
                position: p(x_muster, vorder_y),
                naht: None,
                montage: Some(Montage {
                    position: p(x_montage, a.faecher_y[2]),
                    schritt: 3,
                }),
            },
            bedingungen: bed,
        });
    }

    // Innennähte: senkrecht vom obersten Loch bis in die Randnaht. Im
    // Muster liegen sie auf dem Futter, wo sie genäht werden.
    let laenge = h - ka - a.innennaht_y;
    for (seite, x) in [("links", a.innennaht_x), ("rechts", b - a.innennaht_x)] {
        let (pfad, bed) = senkrechte_naht(laenge)?;
        let montage = p(x, a.innennaht_y);
        teile.push(Musterteil {
            name: format!("Innennaht {seite}"),
            teil: Teil {
                form: TeilForm::Offen(pfad),
                position: futter_pos.plus(montage),
                naht: Some(naht(&m, Nahtkanten::Ringsum)),
                montage: Some(Montage {
                    position: montage,
                    schritt: 4,
                }),
            },
            bedingungen: bed,
        });
    }

    let schritte = vec![
        "Alle Teile ausschneiden und die Löcher abstechen. Kanten der \
         Fachöffnungen und die Oberkanten von Außenleder und Futter vorher \
         kanten und polieren."
            .into(),
        "Schritt 1: Kartenfach 1 links und rechts auflegen (Lage siehe \
         Zusammenbau) und den Steg unten quer auf das Futter nähen."
            .into(),
        "Schritt 2: Kartenfach 2 genauso aufnähen; es verdeckt die Stegnaht \
         von Fach 1."
            .into(),
        "Schritt 3: Vorderfächer auflegen und am Rand fixieren (kleben), \
         noch nicht nähen."
            .into(),
        "Schritt 4: Innennähte links und rechts nähen: durch Futter und alle \
         Fächer, nicht durch das Außenleder."
            .into(),
        "Schritt 5: Futter mit den Fächern auf das Außenleder kleben, dabei \
         zugeklappt halten, damit sich das Futter beim Falten nicht wirft. \
         Dann die Randnaht ringsum nähen; die Oberkante bleibt offen \
         (Scheinfach)."
            .into(),
    ];
    Ok(Vorlage { teile, schritte })
}

/// Abgeleitete Maße der Standard-Bifold, z. B. für Tests und Anzeige.
pub fn bifold_abgeleitet(m: BifoldMasse) -> Result<BifoldAbgeleitet, VorlagenFehler> {
    let (pfad, _) = aussenform(&m)?;
    let mut randnaht = vec![true; pfad.kantenzahl(true)];
    randnaht[0] = false;
    let aussen = Teil {
        form: TeilForm::Geschlossen(Form::Pfad(pfad)),
        position: p(0.0, 0.0),
        naht: Some(naht(&m, Nahtkanten::Pfad(randnaht))),
        montage: None,
    };
    ableiten(&m, &aussen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass::TOLERANZ_MM;
    use crate::muster::{self, Umriss};

    fn vorlage() -> Vorlage {
        bifold().unwrap()
    }

    fn teil<'a>(v: &'a Vorlage, name: &str) -> (usize, &'a Musterteil) {
        v.teile
            .iter()
            .enumerate()
            .find(|(_, t)| t.name == name)
            .unwrap()
    }

    fn rechnen(v: &Vorlage) -> (Vec<Teil>, Vec<muster::Berechnet>, Vec<muster::Durchstich>) {
        let teile: Vec<Teil> = v.teile.iter().map(|t| t.teil.clone()).collect();
        let b = muster::berechnen(&teile);
        let d = muster::durchstechen_berechnet(&teile, &b);
        let b = b.into_iter().map(|x| x.unwrap()).collect();
        (teile, b, d)
    }

    /// Alle Löcher eines Teils (eigene und übernommene) im Zusammenbau.
    fn loecher_im_zusammenbau(
        t: &Teil,
        b: &muster::Berechnet,
        d: &muster::Durchstich,
    ) -> Vec<Punkt> {
        let m = t.montage.unwrap();
        let versatz = m.position.minus(t.position);
        let eigene = b
            .stiche
            .as_ref()
            .and_then(|s| s.as_ref().ok())
            .map(|(r, _)| r.loecher.iter().map(|l| l.mitte).collect::<Vec<_>>())
            .unwrap_or_default();
        eigene
            .into_iter()
            .chain(d.loecher.iter().map(|l| l.loch.mitte))
            .map(|q| q.plus(versatz))
            .collect()
    }

    #[test]
    fn alle_skizzen_der_vorlage_sind_vollstaendig_bestimmt() {
        for t in &vorlage().teile {
            if let TeilForm::Geschlossen(Form::Pfad(pfad)) | TeilForm::Offen(pfad) = &t.teil.form {
                let geschlossen = matches!(t.teil.form, TeilForm::Geschlossen(_));
                let e = skizze::loesen(pfad, geschlossen, &t.bedingungen).unwrap();
                assert_eq!(e.freiheitsgrade, 0, "{}", t.name);
                assert!(!e.ueberbestimmt, "{}", t.name);
            }
        }
    }

    #[test]
    fn das_aussenleder_ist_genau_217_x_93_mm_mit_radius_6() {
        let v = vorlage();
        let (_, t) = teil(&v, "Außenleder");
        let Umriss::Teil { kontur, .. } = t.teil.umriss().unwrap() else {
            panic!("Teil erwartet");
        };
        let (von, bis) = kontur.grenzen();
        assert!((bis.x - von.x - 217.0).abs() < TOLERANZ_MM);
        assert!((bis.y - von.y - 93.0).abs() < TOLERANZ_MM);
        let TeilForm::Geschlossen(Form::Pfad(pfad)) = &t.teil.form else {
            panic!("Pfad erwartet");
        };
        for e in &pfad.elemente {
            if let PfadElement::Bogen { radius, .. } = e {
                assert!(radius.etwa_gleich(mm(6.0)), "{radius}");
            }
        }
    }

    #[test]
    fn alle_naehte_passen_zusammen_und_kein_loch_liegt_an_einer_fachkante() {
        let v = vorlage();
        let (_, _, d) = rechnen(&v);
        for (t, d) in v.teile.iter().zip(&d) {
            assert_eq!(d.zu_dicht, 0, "{}: Löcher zu dicht", t.name);
            assert_eq!(d.kantennah, 0, "{}: Loch an der Kante", t.name);
        }
        let a = bifold_abgeleitet(BifoldMasse::default()).unwrap();
        assert!(
            a.freiraum >= 1.9,
            "Freiraum an den Fachkanten {}",
            a.freiraum
        );
    }

    #[test]
    fn jedes_loch_eines_kartenfachs_hat_ein_gegenloch_im_futter() {
        let v = vorlage();
        let (teile, b, d) = rechnen(&v);
        let (f, _) = teil(&v, "Futter");
        let futter = loecher_im_zusammenbau(&teile[f], &b[f], &d[f]);
        for (i, t) in v.teile.iter().enumerate() {
            if !t.name.contains("fach") {
                continue;
            }
            let eigene = loecher_im_zusammenbau(&teile[i], &b[i], &d[i]);
            assert!(!eigene.is_empty(), "{} hat Löcher", t.name);
            for q in eigene {
                assert!(
                    futter.iter().any(|f| f.abstand(q) < TOLERANZ_MM),
                    "{}: Loch bei {q:?} ohne Gegenloch im Futter",
                    t.name
                );
            }
        }
    }

    #[test]
    fn die_innennaht_geht_nicht_durch_das_aussenleder() {
        let v = vorlage();
        let (teile, b, d) = rechnen(&v);
        let (a, _) = teil(&v, "Außenleder");
        let aussen = loecher_im_zusammenbau(&teile[a], &b[a], &d[a]);
        let x = bifold_abgeleitet(BifoldMasse::default())
            .unwrap()
            .innennaht_x;
        // Auf der Innennaht liegt im Außenleder nur das gemeinsame Loch in
        // der Randnaht unten.
        let auf_innennaht: Vec<&Punkt> = aussen
            .iter()
            .filter(|q| (q.x - x).abs() < TOLERANZ_MM)
            .collect();
        assert_eq!(auf_innennaht.len(), 1);
        assert!((auf_innennaht[0].y - 90.0).abs() < TOLERANZ_MM);
    }

    #[test]
    fn das_scheinfach_bleibt_oben_offen() {
        let v = vorlage();
        let (teile, b, d) = rechnen(&v);
        for name in ["Außenleder", "Futter"] {
            let (i, _) = teil(&v, name);
            let l = loecher_im_zusammenbau(&teile[i], &b[i], &d[i]);
            // Oben zwischen den Eckbögen kein Loch.
            assert!(
                !l.iter().any(|q| q.y < 5.0 && q.x > 7.0 && q.x < 209.0),
                "{name}"
            );
        }
    }

    #[test]
    fn die_stegnaht_von_fach_1_geht_ins_futter_aber_nicht_durch_fach_2() {
        let v = vorlage();
        let (teile, b, d) = rechnen(&v);
        let (k1, _) = teil(&v, "Kartenfach 1 links");
        let (k2, _) = teil(&v, "Kartenfach 2 links");
        let (f, _) = teil(&v, "Futter");
        let steg: Vec<Punkt> = b[k1]
            .stiche
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .0
            .loecher
            .iter()
            .map(|l| {
                l.mitte.plus(
                    teile[k1]
                        .montage
                        .unwrap()
                        .position
                        .minus(teile[k1].position),
                )
            })
            .collect();
        assert!(steg.len() > 10);
        let futter = loecher_im_zusammenbau(&teile[f], &b[f], &d[f]);
        let fach2 = loecher_im_zusammenbau(&teile[k2], &b[k2], &d[k2]);
        for q in &steg {
            assert!(futter.iter().any(|f| f.abstand(*q) < TOLERANZ_MM));
            assert!(!fach2.iter().any(|f| f.abstand(*q) < TOLERANZ_MM));
        }
    }

    #[test]
    fn eine_karte_passt_in_jedes_fach() {
        // Kreditkarte 85,6 × 54 mm; zwischen Rand- und Innennaht braucht
        // sie gut 4 mm Luft.
        let a = bifold_abgeleitet(BifoldMasse::default()).unwrap();
        let lichte_breite = a.innennaht_x - 3.0;
        let m = BifoldMasse::default();
        let rand_innen = fachbreite(&m, &a) - a.innennaht_x;
        assert!(
            rand_innen >= 2.4,
            "Innennaht {rand_innen} mm von der Fachkante"
        );
        assert!(lichte_breite >= 85.6 + 4.0, "{lichte_breite}");
        // Die Karte im hintersten Fach ragt nicht über die Oberkante.
        let kartenboden = a.faecher_y[0] + m.fachhoehe - m.kantenabstand;
        assert!(kartenboden - 54.0 > 0.0, "{}", kartenboden - 54.0);
    }
}
