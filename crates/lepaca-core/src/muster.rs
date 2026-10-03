//! Ein Muster aus mehreren Teilen und wie die Teile zusammengenäht werden.
//!
//! Jedes Teil hat eine Lage im Muster (dort wird es gedruckt und
//! ausgeschnitten). Teile, die zusammengenäht werden, haben zusätzlich eine
//! Lage im **Zusammenbau**: wo sie im fertigen Stück aufeinanderliegen,
//! und den **Nähschritt**, in dem sie dazukommen.
//!
//! Regel für die Löcher: Eine Naht geht durch alle Lagen, die beim Nähen
//! schon da sind. Eine Naht aus Schritt `s` sticht also durch jedes Teil
//! mit Schritt ≤ `s`, das an dieser Stelle liegt. So bekommen alle Teile
//! einer Naht genau dieselben Löcher, und eine Naht, die vor dem Aufsetzen
//! eines Teils genäht wird, geht nicht hindurch.

use crate::eisen::{Eisenart, Lochform};
use crate::form::{Form, FormFehler, OffeneLinie, Pfad, TPocketNaehte};
use crate::geometrie::{Kontur, Punkt};
use crate::stichlinie::{self, Einstellung, StichFehler, Stichloch, Stichreihe};

/// Zwei Löcher, die näher als das beieinanderliegen, sind dasselbe Loch
/// (zwei Nähte teilen sich ein Eckloch).
pub const GLEICHES_LOCH_MM: f64 = 0.1;
/// Liegen zwei verschiedene Löcher näher als das, passen die Nähte nicht
/// zueinander.
pub const MIN_LOCHABSTAND_MM: f64 = 1.5;
/// Ein Loch neben einem Teil muss mindestens so weit von dessen Kante
/// wegbleiben, sonst schneidet es sie beim Abstechen an.
pub const MIN_SCHNITTABSTAND_MM: f64 = 0.5;
/// Mindestabstand einer Lochmitte zu einer Schnittkante, durch die die
/// Naht nicht verläuft (z. B. die Oberkante eines Kartenfachs).
pub const MIN_KANTENABSTAND_MM: f64 = 1.5;

/// Was ein Teil ist: eine Form mit Schnittkante oder eine offene Linie
/// (z. B. eine Aufnähnaht).
#[derive(Debug, Clone, PartialEq)]
pub enum TeilForm {
    Geschlossen(Form),
    Offen(Pfad),
}

/// An welchen Kanten genäht wird.
#[derive(Debug, Clone, PartialEq)]
pub enum Nahtkanten {
    Ringsum,
    TPocket(TPocketNaehte),
    /// Je Pfadkante (siehe [`Pfad::kantenzahl`]).
    Pfad(Vec<bool>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Naht {
    pub einstellung: Einstellung,
    pub eisen: Eisenart,
    pub kanten: Nahtkanten,
}

/// Lage eines Teils im fertigen Stück.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Montage {
    /// Wo der Bezugspunkt des Teils im Zusammenbau liegt (mm).
    pub position: Punkt,
    /// Nähschritt, in dem das Teil dazukommt; seine eigene Naht wird in
    /// diesem Schritt genäht.
    pub schritt: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Teil {
    pub form: TeilForm,
    /// Lage im Muster (Bezugspunkt der Form).
    pub position: Punkt,
    pub naht: Option<Naht>,
    pub montage: Option<Montage>,
}

/// Die Umrisslinie eines Teils an seiner Lage im Muster.
#[derive(Debug, Clone, PartialEq)]
pub enum Umriss {
    Teil {
        kontur: Kontur,
        /// Je Konturkante, ob dort genäht wird; `None` heißt ringsum.
        kanten: Option<Vec<bool>>,
    },
    Linie(OffeneLinie),
}

impl Umriss {
    pub fn punkte(&self) -> &[Punkt] {
        match self {
            Umriss::Teil { kontur, .. } => kontur.punkte(),
            Umriss::Linie(l) => &l.punkte,
        }
    }

    pub fn geschlossen(&self) -> bool {
        matches!(self, Umriss::Teil { .. })
    }
}

/// Ein Teil mit Namen und den Maßen und Bedingungen seiner Skizze.
#[derive(Debug, Clone, PartialEq)]
pub struct Musterteil {
    pub name: String,
    pub teil: Teil,
    /// Maße und Bedingungen, falls das Teil eine Skizze ist.
    pub bedingungen: Vec<crate::skizze::Bedingung>,
}

/// Höchstzahl Teile in einem Muster.
pub const MAX_TEILE: usize = 1000;

impl Teil {
    /// Umriss an der Lage im Muster, mit der Kantenwahl der Naht.
    pub fn umriss(&self) -> Result<Umriss, StichOderForm> {
        let d = self.position;
        match &self.form {
            TeilForm::Offen(p) => {
                let mut l = p.linienzug()?;
                for q in &mut l.punkte {
                    *q = q.plus(d);
                }
                Ok(Umriss::Linie(l))
            }
            TeilForm::Geschlossen(f) => {
                let kanten = self.naht.as_ref().map(|n| &n.kanten);
                let (kontur, kanten) = match (f, kanten) {
                    (Form::Pfad(p), Some(Nahtkanten::Pfad(wahl))) => {
                        let (k, herkunft) = p.kontur_mit_kanten()?;
                        if wahl.len() != p.kantenzahl(true) {
                            return Err(StichFehler::KantenUnpassend.into());
                        }
                        let kanten = herkunft.iter().map(|&h| wahl[h]).collect();
                        (k, Some(kanten))
                    }
                    (Form::TPocket { .. }, Some(Nahtkanten::TPocket(t))) => {
                        (f.kontur()?, Some(t.kanten()))
                    }
                    (_, Some(Nahtkanten::Ringsum) | None) => (f.kontur()?, None),
                    _ => return Err(StichFehler::KantenUnpassend.into()),
                };
                Ok(Umriss::Teil {
                    kontur: kontur.verschoben(d.x, d.y),
                    kanten,
                })
            }
        }
    }

    /// Die eigene Naht des Teils (ohne übernommene Löcher).
    pub fn stiche(&self, u: &Umriss) -> Option<Result<(Stichreihe, Lochform), StichFehler>> {
        let n = self.naht.as_ref()?;
        let e = n.einstellung;
        let reihe = match u {
            Umriss::Teil {
                kontur,
                kanten: Some(k),
            } => {
                if k.iter().any(|x| *x) {
                    stichlinie::stichreihe_an_kanten(kontur, k, e)
                } else {
                    Ok(Stichreihe {
                        linien: Vec::new(),
                        loecher: Vec::new(),
                        abschnitte: Vec::new(),
                    })
                }
            }
            Umriss::Teil {
                kontur,
                kanten: None,
            } => stichlinie::stichreihe(kontur, e),
            Umriss::Linie(l) => stichlinie::stichreihe_entlang(&l.punkte, &l.ecke, e),
        };
        Some(reihe.map(|r| (r, n.eisen.lochform(e.lochabstand))))
    }
}

/// Fehler beim Bauen eines Teils.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StichOderForm {
    Form(FormFehler),
    Stich(StichFehler),
}

impl From<FormFehler> for StichOderForm {
    fn from(f: FormFehler) -> Self {
        StichOderForm::Form(f)
    }
}

impl From<StichFehler> for StichOderForm {
    fn from(f: StichFehler) -> Self {
        StichOderForm::Stich(f)
    }
}

/// Ein Loch, das ein Teil von der Naht eines anderen Teils bekommt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UebernommenesLoch {
    /// Lage im Muster des empfangenden Teils.
    pub loch: Stichloch,
    pub form: Lochform,
    /// Index des Teils, dessen Naht hier durchgeht.
    pub von: usize,
}

/// Was beim Durchstechen der Lagen für ein Teil herauskommt.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Durchstich {
    pub loecher: Vec<UebernommenesLoch>,
    /// Löcher, die zu nah an einer Schnittkante liegen oder sie anschneiden.
    pub kantennah: usize,
    /// Paare von Löchern, die sich fast treffen: Die Nähte passen hier
    /// nicht zueinander.
    pub zu_dicht: usize,
}

/// Abstand eines Punkts zur nächsten Kante der Kontur.
pub fn abstand_zur_kontur(k: &Kontur, q: Punkt) -> f64 {
    let n = k.len();
    (0..n)
        .map(|i| {
            let (a, b) = (k.punkt(i), k.punkt(i + 1));
            let ab = b.minus(a);
            let l2 = ab.skalar(ab);
            let t = if l2 > 0.0 {
                (q.minus(a).skalar(ab) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            q.abstand(a.plus(ab.mal(t)))
        })
        .fold(f64::INFINITY, f64::min)
}

/// Punkt-in-Kontur nach der Gerade-Ungerade-Regel.
pub fn enthaelt(k: &Kontur, q: Punkt) -> bool {
    let p = k.punkte();
    let mut drin = false;
    let mut j = p.len() - 1;
    for i in 0..p.len() {
        let (a, b) = (p[i], p[j]);
        if (a.y > q.y) != (b.y > q.y) && q.x < (b.x - a.x) * (q.y - a.y) / (b.y - a.y) + a.x {
            drin = !drin;
        }
        j = i;
    }
    drin
}

/// Überträgt die Löcher jeder Naht auf alle Lagen, durch die sie beim Nähen
/// geht. `umrisse` und `stiche` gehören zu `teile` (gleiche Reihenfolge);
/// fehlerhafte Teile stehen als `None` darin und werden übergangen.
pub fn durchstechen(
    teile: &[Teil],
    umrisse: &[Option<&Umriss>],
    stiche: &[Option<(&Stichreihe, &Lochform)>],
) -> Vec<Durchstich> {
    let mut ergebnis = vec![Durchstich::default(); teile.len()];
    // Verschiebung Muster → Zusammenbau je Teil.
    let versatz: Vec<Option<Punkt>> = teile
        .iter()
        .map(|t| t.montage.map(|m| m.position.minus(t.position)))
        .collect();

    for (q, ziel) in teile.iter().enumerate() {
        let (Some(mq), Some(dq), Some(Umriss::Teil { kontur, .. })) =
            (ziel.montage, versatz[q], umrisse[q])
        else {
            continue;
        };
        // Kontur des empfangenden Teils im Zusammenbau.
        let kontur_z = kontur.verschoben(dq.x, dq.y);
        // Alle Löcher, die das Teil schon hat (eigene und übernommene), im
        // Zusammenbau, um doppelte und zu dichte Löcher zu erkennen.
        let mut vorhanden: Vec<Punkt> = stiche[q]
            .map(|(r, _)| r.loecher.iter().map(|l| l.mitte.plus(dq)).collect())
            .unwrap_or_default();
        let d = &mut ergebnis[q];
        for (p, geber) in teile.iter().enumerate() {
            let (Some(mp), Some(dp), Some((reihe, form))) = (geber.montage, versatz[p], stiche[p])
            else {
                continue;
            };
            if p == q || mp.schritt < mq.schritt {
                continue;
            }
            for l in &reihe.loecher {
                let m = l.mitte.plus(dp);
                let drin = enthaelt(&kontur_z, m);
                let rand = abstand_zur_kontur(&kontur_z, m);
                if !drin {
                    // Knapp daneben schneidet das Loch die Kante an.
                    let umriss = form.umriss(&Stichloch { mitte: m, ..*l });
                    let schneidet = umriss.iter().any(|e| enthaelt(&kontur_z, *e))
                        || umriss
                            .iter()
                            .any(|e| abstand_zur_kontur(&kontur_z, *e) < MIN_SCHNITTABSTAND_MM);
                    if schneidet {
                        d.kantennah += 1;
                    }
                    continue;
                }
                if rand < MIN_KANTENABSTAND_MM {
                    // Gilt nur für Kanten, an denen die Naht nicht entlang
                    // läuft; eine Randnaht liegt immer im Kantenabstand.
                    if rand < l_kantenabstand(geber) - 0.01 {
                        d.kantennah += 1;
                    }
                }
                let naechstes = vorhanden
                    .iter()
                    .map(|v| v.abstand(m))
                    .fold(f64::INFINITY, f64::min);
                if naechstes < GLEICHES_LOCH_MM {
                    continue;
                }
                if naechstes < MIN_LOCHABSTAND_MM {
                    d.zu_dicht += 1;
                }
                vorhanden.push(m);
                d.loecher.push(UebernommenesLoch {
                    loch: Stichloch {
                        mitte: m.minus(dq),
                        ..*l
                    },
                    form: *form,
                    von: p,
                });
            }
        }
    }
    ergebnis
}

/// Kantenabstand der Naht eines Teils; bei einer offenen Linie gibt es
/// keinen, dann zählt jede Nähe zur Kante.
fn l_kantenabstand(t: &Teil) -> f64 {
    match (&t.form, &t.naht) {
        (TeilForm::Geschlossen(_), Some(n)) => n.einstellung.kantenabstand.wert(),
        _ => f64::INFINITY,
    }
}

/// Ein Teil komplett berechnet: Umriss, eigene Naht und übernommene Löcher.
#[derive(Debug, Clone, PartialEq)]
pub struct Berechnet {
    pub umriss: Umriss,
    pub stiche: Option<Result<(Stichreihe, Lochform), StichFehler>>,
}

/// Berechnet Umriss und eigene Naht aller Teile.
pub fn berechnen(teile: &[Teil]) -> Vec<Result<Berechnet, StichOderForm>> {
    teile
        .iter()
        .map(|t| {
            let umriss = t.umriss()?;
            let stiche = t.stiche(&umriss);
            Ok(Berechnet { umriss, stiche })
        })
        .collect()
}

/// Durchstechen für bereits berechnete Teile.
pub fn durchstechen_berechnet(
    teile: &[Teil],
    berechnet: &[Result<Berechnet, StichOderForm>],
) -> Vec<Durchstich> {
    let umrisse: Vec<Option<&Umriss>> = berechnet
        .iter()
        .map(|b| b.as_ref().ok().map(|b| &b.umriss))
        .collect();
    let stiche: Vec<Option<(&Stichreihe, &Lochform)>> = berechnet
        .iter()
        .map(|b| {
            b.as_ref()
                .ok()
                .and_then(|b| b.stiche.as_ref())
                .and_then(|s| s.as_ref().ok())
                .map(|(r, l)| (r, l))
        })
        .collect();
    durchstechen(teile, &umrisse, &stiche)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass::{Mm, TOLERANZ_MM};
    use crate::stichlinie::Eckausrichtung;

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    fn naht(kanten: Nahtkanten) -> Naht {
        Naht {
            einstellung: Einstellung {
                kantenabstand: mm(3.0),
                lochabstand: mm(4.0),
                ecke: Eckausrichtung::default(),
            },
            eisen: Eisenart::Diamant,
            kanten,
        }
    }

    fn rechteck(b: f64, h: f64) -> Form {
        Form::Rechteck {
            breite: mm(b),
            hoehe: mm(h),
            eckenradius: mm(0.0),
        }
    }

    /// Grundteil 100 × 60 mit Randnaht in Schritt 2, darauf eine Tasche
    /// 51 × 29 unten links bündig in Schritt 1, ohne eigene Naht.
    fn muster() -> Vec<Teil> {
        vec![
            Teil {
                form: TeilForm::Geschlossen(rechteck(100.0, 60.0)),
                position: Punkt::neu(0.0, 0.0),
                naht: Some(naht(Nahtkanten::Ringsum)),
                montage: Some(Montage {
                    position: Punkt::neu(0.0, 0.0),
                    schritt: 2,
                }),
            },
            Teil {
                form: TeilForm::Geschlossen(rechteck(51.0, 29.0)),
                // Im Muster woanders gedruckt als im Zusammenbau.
                position: Punkt::neu(0.0, 100.0),
                naht: None,
                montage: Some(Montage {
                    position: Punkt::neu(0.0, 31.0),
                    schritt: 1,
                }),
            },
        ]
    }

    #[test]
    fn eine_aufgesetzte_tasche_bekommt_die_loecher_der_randnaht_die_durch_sie_gehen() {
        let teile = muster();
        let b = berechnen(&teile);
        let d = durchstechen_berechnet(&teile, &b);
        let grund = b[0]
            .as_ref()
            .unwrap()
            .stiche
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap();
        // Löcher der Randnaht, die im Zusammenbau auf der Tasche liegen.
        let erwartet: Vec<Punkt> = grund
            .0
            .loecher
            .iter()
            .map(|l| l.mitte)
            .filter(|m| m.x < 51.0 && m.y > 31.0)
            .collect();
        assert!(!erwartet.is_empty());
        assert_eq!(d[1].loecher.len(), erwartet.len());
        // Auf dem Muster der Tasche liegen sie 69 mm tiefer (100 statt 31).
        for (ist, soll) in d[1].loecher.iter().zip(&erwartet) {
            assert!(ist.loch.mitte.abstand(soll.plus(Punkt::neu(0.0, 69.0))) < TOLERANZ_MM);
        }
        assert_eq!(d[1].zu_dicht, 0);
        // Das Grundteil bekommt von der Tasche nichts (sie hat keine Naht).
        assert!(d[0].loecher.is_empty());
    }

    #[test]
    fn eine_naht_geht_nicht_durch_teile_die_erst_spaeter_dazukommen() {
        let mut teile = muster();
        // Das Grundteil wird jetzt zuerst genäht, die Tasche kommt danach.
        if let Some(m) = teile[1].montage.as_mut() {
            m.schritt = 3;
        }
        let b = berechnen(&teile);
        let d = durchstechen_berechnet(&teile, &b);
        assert!(d[1].loecher.is_empty());
    }

    #[test]
    fn eine_naht_die_eine_fachkante_anschneidet_wird_gemeldet() {
        let mut teile = muster();
        // Taschenoberkante 0,5 mm unter einer Lochreihe der Seitennaht:
        // Seitenlöcher bei y = 3 + k·(54/14) …, hier gezielt danebenlegen.
        let grund = berechnen(&teile)[0].clone().unwrap();
        let (reihe, _) = grund.stiche.unwrap().unwrap();
        let loch_y = reihe
            .loecher
            .iter()
            .map(|l| l.mitte)
            .find(|m| (m.x - 3.0).abs() < 1e-6 && m.y > 20.0)
            .unwrap()
            .y;
        if let Some(m) = teile[1].montage.as_mut() {
            m.position = Punkt::neu(0.0, loch_y - 0.5);
        }
        if let TeilForm::Geschlossen(f) = &mut teile[1].form {
            *f = rechteck(50.0, 60.0 - (loch_y - 0.5));
        }
        let b = berechnen(&teile);
        let d = durchstechen_berechnet(&teile, &b);
        assert!(d[1].kantennah >= 1);
    }

    #[test]
    fn zwei_naehte_die_nicht_zueinander_passen_werden_gemeldet() {
        let mut teile = muster();
        // Die Tasche bekommt eine eigene Randnaht mit anderem Lochabstand.
        let mut n = naht(Nahtkanten::Ringsum);
        n.einstellung.lochabstand = mm(3.0);
        teile[1].naht = Some(n);
        let b = berechnen(&teile);
        let d = durchstechen_berechnet(&teile, &b);
        assert!(d[1].zu_dicht > 0);
    }
}
