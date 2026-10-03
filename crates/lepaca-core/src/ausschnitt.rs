//! Ausschnitte: Formen, die aus einem Teil ausgestanzt werden, und Maße,
//! mit denen sie genau im Teil sitzen.
//!
//! Ein Ausschnitt ist eine geschlossene Form (z. B. ein Kreis für ein
//! Loch oder ein Rechteck für ein Sichtfenster), die ganz im Teil liegt.
//! Beim Zuschnitt wird sie herausgeschnitten; im PDF ist sie eine weitere
//! Schnittlinie.
//!
//! Lage: Jeder Ausschnitt hat eine Position relativ zum Bezugspunkt des
//! Teils (dort, wo die Form ihren Punkt (0, 0) hat). Für Maße zählt sein
//! **Mittelpunkt** (Mitte des umschließenden Rechtecks, beim Kreis also
//! der Kreismittelpunkt) oder sein **Rand** (die Stelle des Umfangs, die
//! dem Gegenüber am nächsten liegt).
//!
//! Maße verschieben nur die Ausschnitte, nie den Umriss des Teils: Der
//! Umriss ist vorher fertig (Grundform oder gelöste Skizze), die
//! Ausschnitte werden daran ausgerichtet. Ändert sich der Umriss, wandern
//! die Ausschnitte mit.

use crate::form::{Form, FormFehler, Pfad, PfadElement};
use crate::geometrie::{Kontur, Punkt};
use crate::loeser::{self, System};
use crate::mass::Mm;
use crate::muster::enthaelt;
use crate::skizze::{self, MASS_ABSTAND, Masslinie, masszahl, pruefe_wert, seite};

/// Höchstzahl Ausschnitte je Teil (Playbook 01).
pub const MAX_AUSSCHNITTE: usize = 100;
/// Höchstzahl Abstandsmaße je Teil.
pub const MAX_MASSE: usize = 400;

/// Eine Form, die aus einem Teil ausgestanzt wird.
#[derive(Debug, Clone, PartialEq)]
pub struct Ausschnitt {
    pub form: Form,
    /// Lage des Bezugspunkts der Form relativ zum Bezugspunkt des Teils.
    pub position: Punkt,
}

/// Wovon aus ein Abstand gemessen wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bezug {
    /// Vom Mittelpunkt des Ausschnitts.
    Mitte,
    /// Vom Rand (Umfang) des Ausschnitts, an der nächstgelegenen Stelle.
    Rand,
}

/// In welche Richtung ein Abstand zwischen zwei Ausschnitten gilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Richtung {
    /// Kürzeste Verbindung. Vom Rand aus nur zwischen Kreisen.
    Direkt,
    Waagerecht,
    Senkrecht,
}

/// Ein Maß, das einen Ausschnitt im Teil festlegt. Die Seite (links oder
/// rechts einer Kante, welcher Ausschnitt weiter links liegt) bleibt, wie
/// sie gezeichnet ist.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Abstandsmass {
    /// Abstand eines Ausschnitts zu einer geraden Kante des Teils, senkrecht
    /// zur Kante gemessen. `kante` zählt wie [`gerade_kanten`].
    ZurKante {
        ausschnitt: usize,
        kante: usize,
        bezug: Bezug,
        wert: Mm,
    },
    /// Abstand zwischen zwei Ausschnitten.
    Zwischen {
        a: usize,
        b: usize,
        richtung: Richtung,
        bezug: Bezug,
        wert: Mm,
    },
}

/// Warum Ausschnitte nicht gehen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AusschnittFehler {
    /// Mehr als [`MAX_AUSSCHNITTE`] Ausschnitte oder [`MAX_MASSE`] Maße.
    ZuGross,
    /// Ein Ausschnitt reicht über den Rand des Teils hinaus oder liegt
    /// ganz daneben.
    NichtImTeil,
    /// Zwei Ausschnitte überschneiden sich.
    Ueberschneiden,
    /// Eine offene Linie hat keine Fläche, aus der etwas ausgestanzt werden
    /// könnte.
    OffeneLinie,
    /// Ein Maß nennt einen Ausschnitt oder eine Kante, die es nicht gibt.
    Verweis,
    /// Das Maß passt nicht, z. B. eine gebogene Kante oder ein Abstand vom
    /// Rand zwischen zwei Formen, die keine Kreise sind.
    Unpassend,
    /// Ein Maß ist null, negativ oder keine Zahl.
    UngueltigerWert,
    /// Die Maße widersprechen sich.
    Widerspruch,
    Form(FormFehler),
}

impl From<FormFehler> for AusschnittFehler {
    fn from(f: FormFehler) -> Self {
        AusschnittFehler::Form(f)
    }
}

/// Gerade Kanten des Teils, auf die sich Maße beziehen können, als
/// Anfangs- und Endpunkt (relativ zum Teil). `None` für gebogene Kanten.
///
/// Zählweise: Rechteck oben, rechts, unten, links (bei runden Ecken nur
/// das gerade Stück); T-Pocket wie ihre Kontur (0 ist die Öffnung); ein
/// gezeichnetes Teil wie seine Kanten in der Skizze. Kreis und Ellipse
/// haben keine geraden Kanten.
pub fn gerade_kanten(form: &Form) -> Vec<Option<(Punkt, Punkt)>> {
    match *form {
        Form::Rechteck {
            breite,
            hoehe,
            eckenradius,
        } => {
            let (b, h) = (breite.wert(), hoehe.wert());
            let r = eckenradius.wert().clamp(0.0, b.min(h) / 2.0);
            let kante = |a: Punkt, e: Punkt| (a.abstand(e) > 1e-9).then_some((a, e));
            vec![
                kante(Punkt::neu(r, 0.0), Punkt::neu(b - r, 0.0)),
                kante(Punkt::neu(b, r), Punkt::neu(b, h - r)),
                kante(Punkt::neu(b - r, h), Punkt::neu(r, h)),
                kante(Punkt::neu(0.0, h - r), Punkt::neu(0.0, r)),
            ]
        }
        Form::TPocket { .. } => match form.kontur() {
            Ok(k) => (0..k.len())
                .map(|i| Some((k.punkt(i), k.punkt(i + 1))))
                .collect(),
            Err(_) => Vec::new(),
        },
        Form::Kreis { .. } | Form::Ellipse { .. } => Vec::new(),
        Form::Pfad(ref p) => pfad_kanten(p),
    }
}

fn pfad_kanten(p: &Pfad) -> Vec<Option<(Punkt, Punkt)>> {
    let mut aus = Vec::with_capacity(p.elemente.len() + 1);
    let mut von = p.start;
    for e in &p.elemente {
        aus.push(matches!(e, PfadElement::Linie(_)).then_some((von, e.ziel())));
        von = e.ziel();
    }
    if p.hat_schlusslinie() && !p.elemente.is_empty() {
        aus.push(Some((von, p.start)));
    }
    aus
}

/// Ein Ausschnitt, für die Rechnung vorbereitet.
struct Vorbereitet {
    /// Mittelpunkt relativ zur Position.
    mitte: Punkt,
    /// Punkte des Umfangs relativ zum Mittelpunkt.
    rand: Vec<Punkt>,
    /// Radius, falls der Ausschnitt ein Kreis ist.
    kreis: Option<f64>,
}

impl Vorbereitet {
    fn neu(a: &Ausschnitt) -> Result<Self, AusschnittFehler> {
        let k = a.form.kontur()?;
        // Grundformen liegen mit ihrer linken oberen Ecke bei (0, 0); ihre
        // Mitte ist genau bekannt. Gezeichnete Formen: Mitte des
        // umschließenden Rechtecks.
        let mitte = match a.form {
            Form::Kreis { durchmesser: d } => Punkt::neu(d.wert(), d.wert()).mal(0.5),
            Form::Rechteck { breite, hoehe, .. }
            | Form::Ellipse { breite, hoehe }
            | Form::TPocket { breite, hoehe, .. } => {
                Punkt::neu(breite.wert(), hoehe.wert()).mal(0.5)
            }
            Form::Pfad(_) => {
                let (min, max) = k.grenzen();
                min.plus(max).mal(0.5)
            }
        };
        Ok(Vorbereitet {
            mitte,
            rand: k.punkte().iter().map(|q| q.minus(mitte)).collect(),
            kreis: match a.form {
                Form::Kreis { durchmesser } => Some(durchmesser.wert() / 2.0),
                _ => None,
            },
        })
    }

    /// Randpunkt (relativ zur Mitte), der in Richtung `r` am weitesten
    /// hinausragt.
    fn aeusserster(&self, r: Punkt) -> Punkt {
        // Beim Kreis genau statt über die abgeflachte Kontur.
        if let (Some(radius), Some(u)) = (self.kreis, r.normiert()) {
            return u.mal(radius);
        }
        self.rand
            .iter()
            .copied()
            .fold(None::<Punkt>, |b, q| match b {
                Some(b) if b.skalar(r) >= q.skalar(r) => Some(b),
                _ => Some(q),
            })
            .unwrap_or(Punkt::neu(0.0, 0.0))
    }
}

/// Mittelpunkt eines Ausschnitts relativ zum Teil (Bezugspunkt für Maße
/// „zur Mitte“).
pub fn mitte(a: &Ausschnitt) -> Result<Punkt, AusschnittFehler> {
    Ok(a.position.plus(Vorbereitet::neu(a)?.mitte))
}

/// Eine Gleichung über den Positionen der Ausschnitte. Ausschnitt `i` hat
/// die Unbekannten `2i` (x) und `2i + 1` (y).
#[derive(Debug, Clone, Copy)]
enum Gleichung {
    /// s · n · (Mitte − a) + versatz = soll
    Kante {
        i: usize,
        mitte: Punkt,
        a: Punkt,
        n: Punkt,
        versatz: f64,
        soll: f64,
    },
    /// |Mitte_b − Mitte_a| − abzug = soll
    Direkt {
        a: usize,
        mitte_a: Punkt,
        b: usize,
        mitte_b: Punkt,
        abzug: f64,
        soll: f64,
    },
    /// richtung · (Mitte_b − Mitte_a) − abzug = soll
    Achse {
        a: usize,
        mitte_a: Punkt,
        b: usize,
        mitte_b: Punkt,
        richtung: Punkt,
        abzug: f64,
        soll: f64,
    },
}

fn lage(v: &[f64], i: usize, mitte: Punkt) -> Punkt {
    Punkt::neu(v[2 * i], v[2 * i + 1]).plus(mitte)
}

impl Gleichung {
    fn wert(&self, v: &[f64]) -> f64 {
        match *self {
            Gleichung::Kante {
                i,
                mitte,
                a,
                n,
                versatz,
                soll,
            } => n.skalar(lage(v, i, mitte).minus(a)) + versatz - soll,
            Gleichung::Direkt {
                a,
                mitte_a,
                b,
                mitte_b,
                abzug,
                soll,
            } => lage(v, b, mitte_b).abstand(lage(v, a, mitte_a)) - abzug - soll,
            Gleichung::Achse {
                a,
                mitte_a,
                b,
                mitte_b,
                richtung,
                abzug,
                soll,
            } => richtung.skalar(lage(v, b, mitte_b).minus(lage(v, a, mitte_a))) - abzug - soll,
        }
    }

    fn variablen(&self) -> Vec<usize> {
        match *self {
            Gleichung::Kante { i, .. } => vec![2 * i, 2 * i + 1],
            Gleichung::Direkt { a, b, .. } | Gleichung::Achse { a, b, .. } => {
                let mut v = vec![2 * a, 2 * a + 1, 2 * b, 2 * b + 1];
                v.sort_unstable();
                v
            }
        }
    }
}

/// Ergebnis des Lösers.
#[derive(Debug, Clone, PartialEq)]
pub struct Loesung {
    /// Neue Position je Ausschnitt.
    pub positionen: Vec<Punkt>,
    /// Wie viele Freiheitsgrade noch offen sind; 0 heißt, alle Ausschnitte
    /// liegen fest.
    pub freiheitsgrade: usize,
    /// Mindestens ein Maß ist schon durch andere festgelegt.
    pub ueberbestimmt: bool,
    /// Je Ausschnitt, ob er sich noch verschieben lässt.
    pub frei: Vec<bool>,
}

/// Richtungen eines Abstands und die Seite, auf der `b` von `a` aus liegt.
fn achse(richtung: Richtung, a: Punkt, b: Punkt) -> Option<Punkt> {
    match richtung {
        Richtung::Direkt => None,
        Richtung::Waagerecht => Some(Punkt::neu(seite(b.x - a.x), 0.0)),
        Richtung::Senkrecht => Some(Punkt::neu(0.0, seite(b.y - a.y))),
    }
}

/// Einheitsnormale einer Kante, so gedreht, dass `zu` auf ihrer positiven
/// Seite liegt.
fn normale(a: Punkt, e: Punkt, zu: Punkt) -> Option<Punkt> {
    let d = e.minus(a).normiert()?;
    let n = Punkt::neu(-d.y, d.x);
    Some(n.mal(seite(n.skalar(zu.minus(a)))))
}

fn gleichungen(
    form: &Form,
    ausschnitte: &[Ausschnitt],
    vor: &[Vorbereitet],
    masse: &[Abstandsmass],
) -> Result<Vec<Gleichung>, AusschnittFehler> {
    let kanten = gerade_kanten(form);
    let wert = |w: Mm| pruefe_wert(w).map_err(|_| AusschnittFehler::UngueltigerWert);
    let index = |i: usize| {
        if i < ausschnitte.len() {
            Ok(i)
        } else {
            Err(AusschnittFehler::Verweis)
        }
    };
    let mitte = |i: usize| ausschnitte[i].position.plus(vor[i].mitte);
    let mut g = Vec::with_capacity(masse.len());
    for m in masse {
        match *m {
            Abstandsmass::ZurKante {
                ausschnitt,
                kante,
                bezug,
                wert: w,
            } => {
                let i = index(ausschnitt)?;
                let (a, e) = kanten
                    .get(kante)
                    .ok_or(AusschnittFehler::Verweis)?
                    .ok_or(AusschnittFehler::Unpassend)?;
                let n = normale(a, e, mitte(i)).ok_or(AusschnittFehler::Unpassend)?;
                // Der Randpunkt, der der Kante am nächsten liegt.
                let versatz = match bezug {
                    Bezug::Mitte => 0.0,
                    Bezug::Rand => n.skalar(vor[i].aeusserster(n.mal(-1.0))),
                };
                g.push(Gleichung::Kante {
                    i,
                    mitte: vor[i].mitte,
                    a,
                    n,
                    versatz,
                    soll: wert(w)?,
                });
            }
            Abstandsmass::Zwischen {
                a,
                b,
                richtung,
                bezug,
                wert: w,
            } => {
                let (a, b) = (index(a)?, index(b)?);
                if a == b {
                    return Err(AusschnittFehler::Verweis);
                }
                let (ma, mb) = (mitte(a), mitte(b));
                match achse(richtung, ma, mb) {
                    None => {
                        let abzug = match (bezug, vor[a].kreis, vor[b].kreis) {
                            (Bezug::Mitte, _, _) => 0.0,
                            (Bezug::Rand, Some(ra), Some(rb)) => ra + rb,
                            _ => return Err(AusschnittFehler::Unpassend),
                        };
                        g.push(Gleichung::Direkt {
                            a,
                            mitte_a: vor[a].mitte,
                            b,
                            mitte_b: vor[b].mitte,
                            abzug,
                            soll: wert(w)?,
                        });
                    }
                    Some(r) => {
                        let abzug = match bezug {
                            Bezug::Mitte => 0.0,
                            Bezug::Rand => {
                                vor[a].aeusserster(r).skalar(r)
                                    + vor[b].aeusserster(r.mal(-1.0)).skalar(r.mal(-1.0))
                            }
                        };
                        g.push(Gleichung::Achse {
                            a,
                            mitte_a: vor[a].mitte,
                            b,
                            mitte_b: vor[b].mitte,
                            richtung: r,
                            abzug,
                            soll: wert(w)?,
                        });
                    }
                }
            }
        }
    }
    Ok(g)
}

/// Verschiebt die Ausschnitte so wenig wie möglich, bis alle Maße
/// stimmen. `form` ist der Umriss des Teils.
pub fn loesen(
    form: &Form,
    ausschnitte: &[Ausschnitt],
    masse: &[Abstandsmass],
) -> Result<Loesung, AusschnittFehler> {
    if ausschnitte.len() > MAX_AUSSCHNITTE || masse.len() > MAX_MASSE {
        return Err(AusschnittFehler::ZuGross);
    }
    if ausschnitte.iter().any(|a| !a.position.ist_endlich()) {
        return Err(AusschnittFehler::UngueltigerWert);
    }
    let vor = ausschnitte
        .iter()
        .map(Vorbereitet::neu)
        .collect::<Result<Vec<_>, _>>()?;
    let g = gleichungen(form, ausschnitte, &vor, masse)?;
    let mut v: Vec<f64> = ausschnitte
        .iter()
        .flat_map(|a| [a.position.x, a.position.y])
        .collect();
    let system = System {
        anzahl: v.len(),
        variablen: g.iter().map(Gleichung::variablen).collect(),
        wert: Box::new(|i, v| g[i].wert(v)),
    };
    if !loeser::loese(&system, &mut v) || v.iter().any(|x| !x.is_finite()) {
        return Err(AusschnittFehler::Widerspruch);
    }
    let j = system.jacobi(&mut v);
    let (rang, kern) = loeser::rang_und_kern(&j, v.len());
    let frei = |k: usize| kern.iter().any(|x| x[k].abs() > 1e-6);
    Ok(Loesung {
        positionen: (0..ausschnitte.len())
            .map(|i| Punkt::neu(v[2 * i], v[2 * i + 1]))
            .collect(),
        freiheitsgrade: v.len() - rang,
        ueberbestimmt: rang < g.len(),
        frei: (0..ausschnitte.len())
            .map(|i| frei(2 * i) || frei(2 * i + 1))
            .collect(),
    })
}

/// Konturen der Ausschnitte an ihrer Position (relativ zum Teil).
pub fn konturen(ausschnitte: &[Ausschnitt]) -> Result<Vec<Kontur>, AusschnittFehler> {
    if ausschnitte.len() > MAX_AUSSCHNITTE {
        return Err(AusschnittFehler::ZuGross);
    }
    ausschnitte
        .iter()
        .map(|a| {
            if !a.position.ist_endlich() {
                return Err(AusschnittFehler::UngueltigerWert);
            }
            Ok(a.form.kontur()?.verschoben(a.position.x, a.position.y))
        })
        .collect()
}

/// Prüft, dass jeder Ausschnitt ganz im Teil liegt und keiner einen
/// anderen berührt.
pub fn pruefen(aussen: &Kontur, innen: &[Kontur]) -> Result<(), AusschnittFehler> {
    for (i, k) in innen.iter().enumerate() {
        if !k.punkte().iter().all(|q| enthaelt(aussen, *q)) || konturen_schneiden(aussen, k) {
            return Err(AusschnittFehler::NichtImTeil);
        }
        for anderer in &innen[..i] {
            if konturen_schneiden(anderer, k)
                || enthaelt(anderer, k.punkt(0))
                || enthaelt(k, anderer.punkt(0))
            {
                return Err(AusschnittFehler::Ueberschneiden);
            }
        }
    }
    Ok(())
}

/// Schneiden oder berühren sich die Kanten zweier Konturen?
fn konturen_schneiden(a: &Kontur, b: &Kontur) -> bool {
    let (bmin, bmax) = b.grenzen();
    let nb = b.len();
    (0..a.len()).any(|i| {
        let (p, q) = (a.punkt(i), a.punkt(i + 1));
        // Nur Kanten, die überhaupt in die Nähe kommen.
        if p.x.max(q.x) < bmin.x
            || p.x.min(q.x) > bmax.x
            || p.y.max(q.y) < bmin.y
            || p.y.min(q.y) > bmax.y
        {
            return false;
        }
        (0..nb).any(|j| strecken_beruehren(p, q, b.punkt(j), b.punkt(j + 1)))
    })
}

/// Schneiden oder berühren sich zwei Strecken (Berühren zählt mit: Ein
/// Ausschnitt, der genau auf der Kante liegt, reißt aus)?
fn strecken_beruehren(a: Punkt, b: Punkt, c: Punkt, d: Punkt) -> bool {
    let eps = 1e-9;
    let d1 = b.minus(a).kreuz(c.minus(a));
    let d2 = b.minus(a).kreuz(d.minus(a));
    let d3 = d.minus(c).kreuz(a.minus(c));
    let d4 = d.minus(c).kreuz(b.minus(c));
    if ((d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps))
        && ((d3 > eps && d4 < -eps) || (d3 < -eps && d4 > eps))
    {
        return true;
    }
    let auf = |p: Punkt, q: Punkt, r: Punkt, x: f64| {
        x.abs() <= eps
            && r.x >= p.x.min(q.x) - eps
            && r.x <= p.x.max(q.x) + eps
            && r.y >= p.y.min(q.y) - eps
            && r.y <= p.y.max(q.y) + eps
    };
    auf(a, b, c, d1) || auf(a, b, d, d2) || auf(c, d, a, d3) || auf(c, d, b, d4)
}

fn verschoben(mut m: Masslinie, d: Punkt) -> Masslinie {
    for s in &mut m.striche {
        *s = (s.0.plus(d), s.1.plus(d));
    }
    for p in &mut m.pfeile {
        p.0 = p.0.plus(d);
    }
    m.text_pos = m.text_pos.plus(d);
    m
}

/// Maße zum Zeichnen: Größe jedes Ausschnitts (beim Kreis der Durchmesser
/// mit Mittelpunktkreuz) und die Abstandsmaße. Koordinaten relativ zum
/// Teil. Maße, die nicht passen, werden übersprungen.
pub fn zeichnung(
    form: &Form,
    ausschnitte: &[Ausschnitt],
    masse: &[Abstandsmass],
) -> Vec<Masslinie> {
    let Ok(vor) = ausschnitte
        .iter()
        .map(Vorbereitet::neu)
        .collect::<Result<Vec<_>, _>>()
    else {
        return Vec::new();
    };
    let mitte = |i: usize| ausschnitte[i].position.plus(vor[i].mitte);
    let mut aus = Vec::new();
    for (i, a) in ausschnitte.iter().enumerate() {
        match (vor[i].kreis, &a.form) {
            (Some(r), _) => {
                // Schräg nach rechts oben durch die Mitte, damit der
                // Durchmesser nicht auf den waagerechten und senkrechten
                // Abstandsmaßen liegt.
                let c = mitte(i);
                let d = Punkt::neu(1.0, -1.0).mal(std::f64::consts::FRAC_1_SQRT_2);
                let (innen, aussen) = (c.minus(d.mal(r)), c.plus(d.mal(r)));
                let ende = aussen.plus(d.mal(MASS_ABSTAND));
                let k = 1.5;
                aus.push(Masslinie {
                    bedingung: None,
                    striche: vec![
                        (innen, ende),
                        (c.minus(Punkt::neu(k, 0.0)), c.plus(Punkt::neu(k, 0.0))),
                        (c.minus(Punkt::neu(0.0, k)), c.plus(Punkt::neu(0.0, k))),
                    ],
                    pfeile: vec![(aussen, d), (innen, d.mal(-1.0))],
                    text: format!("Ø {}", masszahl(2.0 * r)),
                    text_pos: ende.plus(d.mal(3.0)).plus(Punkt::neu(3.0, 0.0)),
                    text_winkel: 0.0,
                });
            }
            (None, f) => {
                aus.extend(
                    skizze::form_masse(f)
                        .into_iter()
                        .map(|m| verschoben(m, a.position)),
                );
            }
        }
    }
    let kanten = gerade_kanten(form);
    for m in masse {
        match *m {
            Abstandsmass::ZurKante {
                ausschnitt: i,
                kante,
                bezug,
                wert,
            } if i < ausschnitte.len() => {
                let Some(Some((a, e))) = kanten.get(kante).copied() else {
                    continue;
                };
                let Some(n) = normale(a, e, mitte(i)) else {
                    continue;
                };
                let p = match bezug {
                    Bezug::Mitte => mitte(i),
                    Bezug::Rand => mitte(i).plus(vor[i].aeusserster(n.mal(-1.0))),
                };
                let fuss = p.minus(n.mal(n.skalar(p.minus(a))));
                let quer = Punkt::neu(-n.y, n.x);
                let mut linie =
                    skizze::mass_zwischen(None, p, fuss, quer, p, fuss, masszahl(wert.wert()));
                // Liegt der Fußpunkt neben dem geraden Stück, die Kante bis
                // dorthin verlängern.
                let t = fuss.minus(a).skalar(e.minus(a)) / e.minus(a).skalar(e.minus(a));
                if t < 0.0 {
                    linie.striche.push((a, fuss));
                } else if t > 1.0 {
                    linie.striche.push((e, fuss));
                }
                aus.push(linie);
            }
            Abstandsmass::Zwischen {
                a,
                b,
                richtung,
                bezug,
                wert,
            } if a < ausschnitte.len() && b < ausschnitte.len() && a != b => {
                let (ma, mb) = (mitte(a), mitte(b));
                let text = masszahl(wert.wert());
                match achse(richtung, ma, mb) {
                    None => {
                        let Some(u) = mb.minus(ma).normiert() else {
                            continue;
                        };
                        let (pa, pb) = match (bezug, vor[a].kreis, vor[b].kreis) {
                            (Bezug::Rand, Some(ra), Some(rb)) => {
                                (ma.plus(u.mal(ra)), mb.minus(u.mal(rb)))
                            }
                            _ => (ma, mb),
                        };
                        let quer = Punkt::neu(-u.y, u.x);
                        aus.push(skizze::mass_zwischen(None, pa, pb, quer, pa, pb, text));
                    }
                    Some(r) => {
                        let (pa, pb) = match bezug {
                            Bezug::Mitte => (ma, mb),
                            Bezug::Rand => (
                                ma.plus(vor[a].aeusserster(r)),
                                mb.plus(vor[b].aeusserster(r.mal(-1.0))),
                            ),
                        };
                        // Maßlinie auf Höhe (bzw. Breite) von `pa`, etwas
                        // abgesetzt, damit sie nicht im Ausschnitt liegt.
                        let quer = Punkt::neu(-r.y.abs(), -r.x.abs());
                        let versatz = quer.mal(MASS_ABSTAND / 2.0);
                        let la = pa.plus(versatz);
                        let lb = if r.x != 0.0 {
                            Punkt::neu(pb.x, la.y)
                        } else {
                            Punkt::neu(la.x, pb.y)
                        };
                        aus.push(skizze::mass_zwischen(None, pa, pb, quer, la, lb, text));
                    }
                }
            }
            _ => {}
        }
    }
    aus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass::TOLERANZ_MM;

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    fn viereck() -> Form {
        Form::Rechteck {
            breite: mm(100.0),
            hoehe: mm(60.0),
            eckenradius: mm(0.0),
        }
    }

    /// Kreis mit 20 mm Durchmesser, ungefähr irgendwo im Viereck abgelegt.
    fn loch(x: f64, y: f64) -> Ausschnitt {
        Ausschnitt {
            form: Form::Kreis {
                durchmesser: mm(20.0),
            },
            position: Punkt::neu(x, y),
        }
    }

    /// Mittelpunkt eines Kreises mit 20 mm Durchmesser an seiner Position.
    fn mitte(p: Punkt) -> Punkt {
        p.plus(Punkt::neu(10.0, 10.0))
    }

    fn zur_kante(kante: usize, bezug: Bezug, wert: f64) -> Abstandsmass {
        Abstandsmass::ZurKante {
            ausschnitt: 0,
            kante,
            bezug,
            wert: mm(wert),
        }
    }

    #[test]
    fn ein_kreis_wird_ueber_zwei_kantenabstaende_zur_mitte_genau_platziert() {
        // Kante 3 ist links, Kante 0 oben.
        let masse = [
            zur_kante(3, Bezug::Mitte, 30.0),
            zur_kante(0, Bezug::Mitte, 25.0),
        ];
        let l = loesen(&viereck(), &[loch(13.0, 11.0)], &masse).unwrap();
        let m = mitte(l.positionen[0]);
        assert!(m.abstand(Punkt::neu(30.0, 25.0)) < TOLERANZ_MM);
        assert_eq!(l.freiheitsgrade, 0);
        assert_eq!(l.frei, vec![false]);
    }

    #[test]
    fn vom_rand_gemessen_zaehlt_der_kreisumfang_statt_der_mitte() {
        // 15 mm vom rechten Rand bis zum Kreisumfang: Mitte 25 mm vom Rand.
        let masse = [
            zur_kante(1, Bezug::Rand, 15.0),
            zur_kante(2, Bezug::Rand, 5.0),
        ];
        let l = loesen(&viereck(), &[loch(60.0, 25.0)], &masse).unwrap();
        let m = mitte(l.positionen[0]);
        assert!((m.x - 75.0).abs() < TOLERANZ_MM, "x = {}", m.x);
        assert!((m.y - 45.0).abs() < TOLERANZ_MM, "y = {}", m.y);
    }

    #[test]
    fn mit_nur_einem_mass_bleibt_der_kreis_entlang_der_kante_verschiebbar() {
        let l = loesen(
            &viereck(),
            &[loch(40.0, 20.0)],
            &[zur_kante(3, Bezug::Mitte, 30.0)],
        )
        .unwrap();
        assert_eq!(l.freiheitsgrade, 1);
        assert_eq!(l.frei, vec![true]);
        // Nur waagerecht verschoben, die Höhe bleibt wie gezeichnet.
        assert!((l.positionen[0].y - 20.0).abs() < TOLERANZ_MM);
    }

    #[test]
    fn die_seite_der_kante_bleibt_wie_gezeichnet() {
        // Der Kreis liegt rechts der linken Kante und bleibt dort.
        let l = loesen(
            &viereck(),
            &[loch(5.0, 20.0)],
            &[zur_kante(3, Bezug::Mitte, 12.0)],
        )
        .unwrap();
        assert!((mitte(l.positionen[0]).x - 12.0).abs() < TOLERANZ_MM);
    }

    #[test]
    fn zwei_loecher_halten_ihren_abstand_von_rand_zu_rand() {
        let a = loch(10.0, 20.0);
        let b = loch(50.0, 22.0);
        let masse = [
            zur_kante(3, Bezug::Mitte, 20.0),
            zur_kante(0, Bezug::Mitte, 30.0),
            Abstandsmass::Zwischen {
                a: 0,
                b: 1,
                richtung: Richtung::Waagerecht,
                bezug: Bezug::Rand,
                wert: mm(8.0),
            },
            Abstandsmass::Zwischen {
                a: 0,
                b: 1,
                richtung: Richtung::Senkrecht,
                bezug: Bezug::Mitte,
                wert: mm(0.5),
            },
        ];
        let l = loesen(&viereck(), &[a, b], &masse).unwrap();
        let (ma, mb) = (mitte(l.positionen[0]), mitte(l.positionen[1]));
        // Rand zu Rand 8 mm: Mitten 8 + 10 + 10 = 28 mm auseinander.
        assert!((mb.x - ma.x - 28.0).abs() < TOLERANZ_MM);
        assert!((mb.y - ma.y - 0.5).abs() < TOLERANZ_MM);
        assert_eq!(l.freiheitsgrade, 0);
    }

    #[test]
    fn direkter_abstand_vom_rand_geht_nur_zwischen_kreisen() {
        let mut eckig = loch(50.0, 20.0);
        eckig.form = viereck();
        let m = [Abstandsmass::Zwischen {
            a: 0,
            b: 1,
            richtung: Richtung::Direkt,
            bezug: Bezug::Rand,
            wert: mm(5.0),
        }];
        let form = Form::Rechteck {
            breite: mm(400.0),
            hoehe: mm(200.0),
            eckenradius: mm(0.0),
        };
        assert_eq!(
            loesen(&form, &[loch(10.0, 20.0), eckig], &m),
            Err(AusschnittFehler::Unpassend)
        );
    }

    #[test]
    fn widerspruechliche_abstaende_werden_abgelehnt() {
        // 30 mm von links und 30 mm von rechts geht bei 100 mm Breite nicht.
        let masse = [
            zur_kante(3, Bezug::Mitte, 30.0),
            zur_kante(1, Bezug::Mitte, 30.0),
        ];
        assert_eq!(
            loesen(&viereck(), &[loch(40.0, 20.0)], &masse),
            Err(AusschnittFehler::Widerspruch)
        );
    }

    #[test]
    fn ein_doppeltes_mass_gilt_als_ueberbestimmt() {
        let masse = [
            zur_kante(3, Bezug::Mitte, 30.0),
            zur_kante(1, Bezug::Mitte, 70.0),
        ];
        let l = loesen(&viereck(), &[loch(40.0, 20.0)], &masse).unwrap();
        assert!(l.ueberbestimmt);
    }

    #[test]
    fn an_gebogenen_kanten_gibt_es_kein_abstandsmass() {
        let rund = Form::Kreis {
            durchmesser: mm(100.0),
        };
        assert_eq!(
            loesen(
                &rund,
                &[loch(40.0, 40.0)],
                &[zur_kante(0, Bezug::Mitte, 10.0)]
            ),
            Err(AusschnittFehler::Verweis)
        );
    }

    #[test]
    fn bei_runden_ecken_zaehlt_das_gerade_stueck_der_kante() {
        let f = Form::Rechteck {
            breite: mm(100.0),
            hoehe: mm(60.0),
            eckenradius: mm(10.0),
        };
        let k = gerade_kanten(&f);
        assert_eq!(k[0], Some((Punkt::neu(10.0, 0.0), Punkt::neu(90.0, 0.0))));
        assert_eq!(k[3], Some((Punkt::neu(0.0, 50.0), Punkt::neu(0.0, 10.0))));
    }

    #[test]
    fn ein_gezeichnetes_teil_hat_gerade_kanten_wie_seine_skizze() {
        let p = Pfad {
            start: Punkt::neu(0.0, 0.0),
            elemente: vec![
                PfadElement::Linie(Punkt::neu(80.0, 0.0)),
                PfadElement::Bogen {
                    ziel: Punkt::neu(80.0, 40.0),
                    radius: mm(20.0),
                    uhrzeigersinn: true,
                    gross: false,
                },
                PfadElement::Linie(Punkt::neu(0.0, 40.0)),
            ],
        };
        let k = gerade_kanten(&Form::Pfad(p));
        assert_eq!(k.len(), 4);
        assert!(k[0].is_some() && k[1].is_none() && k[2].is_some());
        // Die Schlusslinie zurück zum Start.
        assert_eq!(k[3], Some((Punkt::neu(0.0, 40.0), Punkt::neu(0.0, 0.0))));
    }

    #[test]
    fn ein_loch_ganz_im_teil_ist_in_ordnung() {
        let aussen = viereck().kontur().unwrap();
        let innen = konturen(&[loch(20.0, 20.0), loch(60.0, 20.0)]).unwrap();
        assert_eq!(pruefen(&aussen, &innen), Ok(()));
    }

    #[test]
    fn ein_loch_ueber_der_kante_wird_abgelehnt() {
        let aussen = viereck().kontur().unwrap();
        let innen = konturen(&[loch(90.0, 20.0)]).unwrap();
        assert_eq!(pruefen(&aussen, &innen), Err(AusschnittFehler::NichtImTeil));
        // Genau auf der Kante zählt auch.
        let innen = konturen(&[loch(80.0, 20.0)]).unwrap();
        assert_eq!(pruefen(&aussen, &innen), Err(AusschnittFehler::NichtImTeil));
    }

    #[test]
    fn zwei_ueberlappende_loecher_werden_abgelehnt() {
        let aussen = viereck().kontur().unwrap();
        let innen = konturen(&[loch(20.0, 20.0), loch(30.0, 20.0)]).unwrap();
        assert_eq!(
            pruefen(&aussen, &innen),
            Err(AusschnittFehler::Ueberschneiden)
        );
    }

    #[test]
    fn ein_loch_im_loch_wird_abgelehnt() {
        let aussen = viereck().kontur().unwrap();
        let gross = Ausschnitt {
            form: Form::Kreis {
                durchmesser: mm(40.0),
            },
            position: Punkt::neu(10.0, 10.0),
        };
        let innen = konturen(&[gross, loch(20.0, 20.0)]).unwrap();
        assert_eq!(
            pruefen(&aussen, &innen),
            Err(AusschnittFehler::Ueberschneiden)
        );
    }

    #[test]
    fn die_zeichnung_zeigt_durchmesser_und_abstaende() {
        let masse = [
            zur_kante(3, Bezug::Mitte, 30.0),
            zur_kante(0, Bezug::Rand, 15.0),
        ];
        let z = zeichnung(&viereck(), &[loch(20.0, 15.0)], &masse);
        let texte: Vec<&str> = z.iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texte, vec!["Ø 20", "30", "15"]);
        // Das Maß zur linken Kante endet auf der Kante (x = 0).
        let (a, b) = z[1].striche[2];
        assert!(a.x.min(b.x).abs() < TOLERANZ_MM);
        // Vom Rand aus beginnt das obere Maß am Kreisumfang (y = 15).
        let (a, b) = z[2].striche[2];
        assert!((a.y.max(b.y) - 15.0).abs() < 0.05);
    }
}
