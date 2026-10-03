//! Grundformen eines Lederteils.
//!
//! Jede Form wird für Berechnungen in eine [`Kontur`] umgewandelt. Bögen und
//! Kurven werden dabei so fein aufgeteilt, dass die Abweichung höchstens
//! [`FLACH_TOLERANZ_MM`] beträgt.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use crate::geometrie::{Kontur, KonturFehler, MAX_PUNKTE, Punkt};
use crate::mass::Mm;

/// Größte Abweichung zwischen echter Kurve und Punktfolge.
pub const FLACH_TOLERANZ_MM: f64 = 0.01;

/// Kleinste und größte erlaubte Ausdehnung einer Form.
pub const MIN_GROESSE_MM: f64 = 1.0;
pub const MAX_GROESSE_MM: f64 = 5_000.0;

/// Ab diesem Knickwinkel gilt ein Übergang zwischen zwei Pfadstücken als Ecke.
pub const ECKEN_WINKEL_GRAD: f64 = 10.0;

/// Eine Grundform. Rechteck, Kreis und Ellipse liegen mit ihrer linken
/// oberen Ecke bei (0, 0).
#[derive(Debug, Clone, PartialEq)]
pub enum Form {
    /// Rechteck; mit `eckenradius` größer null abgerundet.
    Rechteck {
        breite: Mm,
        hoehe: Mm,
        eckenradius: Mm,
    },
    Kreis {
        durchmesser: Mm,
    },
    Ellipse {
        breite: Mm,
        hoehe: Mm,
    },
    /// Kartenfach in T-Form: oben der volle Querbalken mit den Flügeln,
    /// die in der Randnaht mitlaufen; darunter der schmalere Steg, der
    /// beim Staffeln unter dem nächsten Fach verschwindet.
    TPocket {
        /// Breite über die Flügel gemessen.
        breite: Mm,
        /// Höhe von der Öffnung bis zur Unterkante des Stegs.
        hoehe: Mm,
        /// Wie weit jeder Flügel seitlich über den Steg hinausragt.
        fluegel_ueberstand: Mm,
        /// Höhe der Flügel ab der Öffnung, meist gleich dem Versatz
        /// zum nächsten Fach.
        fluegel_hoehe: Mm,
    },
    /// Freie, geschlossene Form aus Linien und Kurven.
    Pfad(Pfad),
}

/// Nahtwahl für eine T-Pocket, in Begriffen des Teils statt Kantennummern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TPocketNaehte {
    /// Seitenkanten der Flügel; sie laufen in der Randnaht mit.
    pub fluegel: bool,
    /// Unterkante des Stegs, quer genäht.
    pub steg_unten: bool,
    /// Oberkante (Öffnung). Normalerweise nur gekantet, nicht genäht.
    pub oeffnung: bool,
}

impl Default for TPocketNaehte {
    fn default() -> Self {
        TPocketNaehte {
            fluegel: true,
            steg_unten: true,
            oeffnung: false,
        }
    }
}

impl TPocketNaehte {
    /// Kantenwahl passend zur Kontur von [`Form::TPocket`]: Kante `i`
    /// läuft von Punkt `i` zu Punkt `i + 1`.
    pub fn kanten(self) -> Vec<bool> {
        // 0 Öffnung, 1 Flügel rechts, 2 Flügel rechts unten, 3 Steg rechts,
        // 4 Steg unten, 5 Steg links, 6 Flügel links unten, 7 Flügel links.
        vec![
            self.oeffnung,
            self.fluegel,
            false,
            false,
            self.steg_unten,
            false,
            false,
            self.fluegel,
        ]
    }
}

/// Mindestbreite des Stegs, damit noch eine Karte hält und genäht werden kann.
pub const MIN_STEG_MM: f64 = 10.0;

/// Ein Pfad aus Linien, Bögen und Kurven. Als Teil ([`Form::Pfad`]) wird
/// nach dem letzten Element automatisch zum Start zurückgezogen; als
/// offene Linie liefert [`Pfad::linienzug`] die Punkte ohne Schluss.
#[derive(Debug, Clone, PartialEq)]
pub struct Pfad {
    pub start: Punkt,
    pub elemente: Vec<PfadElement>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PfadElement {
    Linie(Punkt),
    /// Kubische Bézierkurve mit zwei Kontrollpunkten.
    Kurve {
        k1: Punkt,
        k2: Punkt,
        ziel: Punkt,
    },
    /// Kreisbogen zum Ziel mit festem Radius. Durch zwei Punkte passen bei
    /// gegebenem Radius vier Bögen; `uhrzeigersinn` (so wie man auf den
    /// Bildschirm schaut) und `gross` (mehr als ein Halbkreis) wählen einen
    /// davon aus, wie bei SVG.
    Bogen {
        ziel: Punkt,
        radius: Mm,
        uhrzeigersinn: bool,
        gross: bool,
    },
}

/// Warum eine Form nicht gebaut werden kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFehler {
    /// Breite, Höhe oder Durchmesser liegt außerhalb von
    /// [`MIN_GROESSE_MM`]..=[`MAX_GROESSE_MM`].
    GroesseAusserhalb,
    /// Die Flügel einer T-Pocket sind so breit, dass vom Steg weniger als
    /// [`MIN_STEG_MM`] bleibt, oder so hoch wie das ganze Fach.
    FluegelZuGross,
    /// Der Eckenradius ist größer als die halbe kürzere Seite.
    EckenradiusZuGross,
    /// Ein Bogen ist unmöglich: Der Radius ist kleiner als der halbe
    /// Abstand von Start und Ziel, oder Start und Ziel fallen zusammen.
    BogenUnmoeglich,
    /// Ein offener Pfad braucht mindestens ein Element.
    PfadLeer,
    Kontur(KonturFehler),
}

impl From<KonturFehler> for FormFehler {
    fn from(f: KonturFehler) -> Self {
        FormFehler::Kontur(f)
    }
}

impl Form {
    /// Wandelt die Form in eine Kontur um.
    pub fn kontur(&self) -> Result<Kontur, FormFehler> {
        match self {
            Form::Rechteck {
                breite,
                hoehe,
                eckenradius,
            } => rechteck(breite.wert(), hoehe.wert(), eckenradius.wert()),
            Form::Kreis { durchmesser } => {
                let d = durchmesser.wert();
                ellipse(d, d)
            }
            Form::Ellipse { breite, hoehe } => ellipse(breite.wert(), hoehe.wert()),
            Form::TPocket {
                breite,
                hoehe,
                fluegel_ueberstand,
                fluegel_hoehe,
            } => t_pocket(
                breite.wert(),
                hoehe.wert(),
                fluegel_ueberstand.wert(),
                fluegel_hoehe.wert(),
            ),
            Form::Pfad(p) => pfad(p),
        }
    }
}

fn pruefe_groesse(w: f64) -> Result<(), FormFehler> {
    if (MIN_GROESSE_MM..=MAX_GROESSE_MM).contains(&w) {
        Ok(())
    } else {
        Err(FormFehler::GroesseAusserhalb)
    }
}

/// Schrittwinkel, damit ein Bogen mit Radius `r` höchstens um die Toleranz
/// von der Sehne abweicht.
fn bogen_schritt(r: f64) -> f64 {
    if r <= FLACH_TOLERANZ_MM {
        FRAC_PI_2
    } else {
        (2.0 * (1.0 - FLACH_TOLERANZ_MM / r).acos()).min(FRAC_PI_2)
    }
}

fn rechteck(b: f64, h: f64, r: f64) -> Result<Kontur, FormFehler> {
    pruefe_groesse(b)?;
    pruefe_groesse(h)?;
    if !(0.0..=b.min(h) / 2.0).contains(&r) {
        return Err(FormFehler::EckenradiusZuGross);
    }
    if r < FLACH_TOLERANZ_MM {
        let p = vec![
            Punkt::neu(0.0, 0.0),
            Punkt::neu(b, 0.0),
            Punkt::neu(b, h),
            Punkt::neu(0.0, h),
        ];
        return Ok(Kontur::neu(p, vec![true; 4])?);
    }
    // Vier Viertelbögen, im Uhrzeigersinn auf dem Bildschirm.
    let mitten = [
        (Punkt::neu(b - r, r), -FRAC_PI_2),
        (Punkt::neu(b - r, h - r), 0.0),
        (Punkt::neu(r, h - r), FRAC_PI_2),
        (Punkt::neu(r, r), PI),
    ];
    let n = (FRAC_PI_2 / bogen_schritt(r)).ceil().max(1.0) as usize;
    let mut punkte = Vec::new();
    for (m, start) in mitten {
        for i in 0..=n {
            let w = start + FRAC_PI_2 * i as f64 / n as f64;
            punkte.push(m.plus(Punkt::neu(w.cos(), w.sin()).mal(r)));
        }
    }
    let ecke = vec![false; punkte.len()];
    Ok(Kontur::neu(punkte, ecke)?)
}

fn t_pocket(b: f64, h: f64, u: f64, fh: f64) -> Result<Kontur, FormFehler> {
    pruefe_groesse(b)?;
    pruefe_groesse(h)?;
    if u <= 0.0 || b - 2.0 * u < MIN_STEG_MM || fh <= 0.0 || fh >= h {
        return Err(FormFehler::FluegelZuGross);
    }
    // Im Uhrzeigersinn auf dem Bildschirm, beginnend oben links.
    let p = vec![
        Punkt::neu(0.0, 0.0),
        Punkt::neu(b, 0.0),
        Punkt::neu(b, fh),
        Punkt::neu(b - u, fh),
        Punkt::neu(b - u, h),
        Punkt::neu(u, h),
        Punkt::neu(u, fh),
        Punkt::neu(0.0, fh),
    ];
    Ok(Kontur::neu(p, vec![true; 8])?)
}

fn ellipse(b: f64, h: f64) -> Result<Kontur, FormFehler> {
    pruefe_groesse(b)?;
    pruefe_groesse(h)?;
    let (a, c) = (b / 2.0, h / 2.0);
    let n = (TAU / bogen_schritt(a.max(c))).ceil().max(8.0) as usize;
    let punkte: Vec<Punkt> = (0..n)
        .map(|i| {
            let w = TAU * i as f64 / n as f64;
            Punkt::neu(a + a * w.cos(), c + c * w.sin())
        })
        .collect();
    let ecke = vec![false; punkte.len()];
    Ok(Kontur::neu(punkte, ecke)?)
}

fn ist_knick(ein: Option<Punkt>, aus: Option<Punkt>) -> bool {
    match (ein.and_then(Punkt::normiert), aus.and_then(Punkt::normiert)) {
        (Some(a), Some(b)) => a.skalar(b) < ECKEN_WINKEL_GRAD.to_radians().cos(),
        _ => false,
    }
}

/// Pfad in Punkte zerlegt, mit Knick-Markierung je Punkt und den
/// Tangenten am Anfang und Ende (für den Schluss eines geschlossenen Pfads).
struct Abgeflacht {
    punkte: Vec<Punkt>,
    ecke: Vec<bool>,
    /// Je Punkt das Element, das ihn erzeugt hat (der Start hat keins).
    herkunft: Vec<Option<usize>>,
    erste_tangente: Option<Punkt>,
    letzte_tangente: Option<Punkt>,
}

fn abflachen(p: &Pfad) -> Result<Abgeflacht, FormFehler> {
    if !p.start.ist_endlich() {
        return Err(KonturFehler::UngueltigeZahl.into());
    }
    let mut punkte = vec![p.start];
    let mut ecke = vec![false];
    let mut herkunft = vec![None];
    let mut letzte_tangente: Option<Punkt> = None;
    let mut erste_tangente: Option<Punkt> = None;

    for (nr, el) in p.elemente.iter().enumerate() {
        let von = punkte[punkte.len() - 1];
        let (t_start, t_ende, neue) = match *el {
            PfadElement::Linie(ziel) => (ziel.minus(von), ziel.minus(von), vec![ziel]),
            PfadElement::Kurve { k1, k2, ziel } => {
                let t_start = if k1.abstand(von) > 1e-9 { k1 } else { k2 }.minus(von);
                let t_ende = ziel.minus(if ziel.abstand(k2) > 1e-9 { k2 } else { k1 });
                (t_start, t_ende, kurve_flach(von, k1, k2, ziel))
            }
            PfadElement::Bogen {
                ziel,
                radius,
                uhrzeigersinn,
                gross,
            } => bogen_flach(von, ziel, radius.wert(), uhrzeigersinn, gross)?,
        };
        if neue.iter().any(|q| !q.ist_endlich()) {
            return Err(KonturFehler::UngueltigeZahl.into());
        }
        if erste_tangente.is_none() {
            erste_tangente = Some(t_start);
        }
        let j = ecke.len() - 1;
        ecke[j] = ist_knick(letzte_tangente, Some(t_start));
        for q in neue {
            if punkte.len() >= MAX_PUNKTE {
                return Err(KonturFehler::ZuVielePunkte.into());
            }
            punkte.push(q);
            ecke.push(false);
            herkunft.push(Some(nr));
        }
        letzte_tangente = Some(t_ende);
    }
    Ok(Abgeflacht {
        punkte,
        ecke,
        herkunft,
        erste_tangente,
        letzte_tangente,
    })
}

fn pfad(p: &Pfad) -> Result<Kontur, FormFehler> {
    Ok(p.kontur_mit_kanten()?.0)
}

impl Pfad {
    /// Anzahl der Kanten, an denen genäht werden kann: ein Element ist eine
    /// Kante; ein geschlossener Pfad, der nicht am Start endet, hat dazu die
    /// gerade Schlusslinie als letzte Kante.
    pub fn kantenzahl(&self, geschlossen: bool) -> usize {
        let n = self.elemente.len();
        if geschlossen && self.hat_schlusslinie() {
            n + 1
        } else {
            n
        }
    }

    /// Endet der Pfad nicht am Start, sodass beim Schließen eine Linie
    /// dazukommt?
    pub fn hat_schlusslinie(&self) -> bool {
        self.elemente
            .last()
            .is_none_or(|e| e.ziel().abstand(self.start) >= 1e-9)
    }

    /// Der geschlossene Pfad als Kontur, dazu je Konturkante (von Punkt `i`
    /// zu Punkt `i + 1`) die Pfadkante, aus der sie stammt (siehe
    /// [`Pfad::kantenzahl`]). Damit lässt sich die Naht je Pfadkante an-
    /// und abschalten.
    pub fn kontur_mit_kanten(&self) -> Result<(Kontur, Vec<usize>), FormFehler> {
        let Abgeflacht {
            punkte,
            ecke,
            herkunft,
            erste_tangente,
            mut letzte_tangente,
        } = abflachen(self)?;
        let schluss = self.elemente.len();

        // Doppelte Punkte (Elemente der Länge null) zusammenfassen, bevor
        // die Kontur es tut, damit die Zuordnung der Kanten stimmt.
        let mut p2: Vec<Punkt> = Vec::with_capacity(punkte.len());
        let mut e2: Vec<bool> = Vec::with_capacity(punkte.len());
        let mut h2: Vec<Option<usize>> = Vec::with_capacity(punkte.len());
        for ((q, e), h) in punkte.into_iter().zip(ecke).zip(herkunft) {
            match p2.last() {
                Some(letzter) if letzter.abstand(q) < 1e-9 => {
                    let i = e2.len() - 1;
                    e2[i] |= e;
                }
                _ => {
                    p2.push(q);
                    e2.push(e);
                    h2.push(h);
                }
            }
        }

        // Schließen: entweder endet der Pfad schon am Start, oder es kommt
        // eine gerade Schlusslinie dazu.
        let letzter = p2[p2.len() - 1];
        let letzte_kante = if p2.len() > 1 && letzter.abstand(self.start) < 1e-9 {
            p2.pop();
            let e = e2.pop().unwrap_or(false);
            e2[0] |= e;
            h2.pop().flatten().unwrap_or(schluss)
        } else {
            let t_schluss = self.start.minus(letzter);
            let j = e2.len() - 1;
            e2[j] = ist_knick(letzte_tangente, Some(t_schluss));
            letzte_tangente = Some(t_schluss);
            schluss
        };
        e2[0] = ist_knick(letzte_tangente, erste_tangente);

        // Kante i läuft zu Punkt i + 1 und gehört zu dessen Element.
        let n = p2.len();
        let kanten: Vec<usize> = (0..n)
            .map(|i| {
                if i + 1 < n {
                    h2[i + 1].unwrap_or(schluss)
                } else {
                    letzte_kante
                }
            })
            .collect();

        let kontur = Kontur::neu(p2, e2)?;
        if kontur.len() != kanten.len() {
            return Err(FormFehler::Kontur(KonturFehler::ZuWenigPunkte));
        }
        let (min, max) = kontur.grenzen();
        pruefe_groesse((max.x - min.x).max(max.y - min.y))?;
        Ok((kontur, kanten))
    }
}

impl PfadElement {
    /// Endpunkt des Elements.
    pub fn ziel(&self) -> Punkt {
        match *self {
            PfadElement::Linie(z) => z,
            PfadElement::Kurve { ziel, .. } | PfadElement::Bogen { ziel, .. } => ziel,
        }
    }
}

/// Eine offene Linie: Punkte und je Punkt, ob dort ein Knick ist. Anfang
/// und Ende zählen als Knick, damit eine Naht dort ein Loch bekommt.
#[derive(Debug, Clone, PartialEq)]
pub struct OffeneLinie {
    pub punkte: Vec<Punkt>,
    pub ecke: Vec<bool>,
}

impl Pfad {
    /// Der Pfad als offene Linie, ohne Schluss zum Start.
    pub fn linienzug(&self) -> Result<OffeneLinie, FormFehler> {
        if self.elemente.is_empty() {
            return Err(FormFehler::PfadLeer);
        }
        let Abgeflacht {
            mut punkte,
            mut ecke,
            ..
        } = abflachen(self)?;
        // Doppelte Punkte (Element der Länge null) entfernen.
        let mut i = 1;
        while i < punkte.len() {
            if punkte[i].abstand(punkte[i - 1]) < 1e-9 {
                ecke[i - 1] |= ecke[i];
                punkte.remove(i);
                ecke.remove(i);
            } else {
                i += 1;
            }
        }
        if punkte.len() < 2 {
            return Err(FormFehler::PfadLeer);
        }
        ecke[0] = true;
        let n = ecke.len();
        ecke[n - 1] = true;
        let (mut min, mut max) = (punkte[0], punkte[0]);
        for q in &punkte {
            min = Punkt::neu(min.x.min(q.x), min.y.min(q.y));
            max = Punkt::neu(max.x.max(q.x), max.y.max(q.y));
        }
        if (max.x - min.x).max(max.y - min.y) > MAX_GROESSE_MM {
            return Err(FormFehler::GroesseAusserhalb);
        }
        Ok(OffeneLinie { punkte, ecke })
    }
}

/// Teilt einen Kreisbogen auf. Liefert Start- und Endtangente und die
/// Punkte ohne den Startpunkt.
fn bogen_flach(
    von: Punkt,
    ziel: Punkt,
    r: f64,
    uhrzeigersinn: bool,
    gross: bool,
) -> Result<(Punkt, Punkt, Vec<Punkt>), FormFehler> {
    let zentrum = bogen_mitte(von, ziel, r, uhrzeigersinn, gross)?;
    let r = r.max(ziel.minus(von).laenge() / 2.0);
    let richtung = if uhrzeigersinn { 1.0 } else { -1.0 };
    let w0 = (von.y - zentrum.y).atan2(von.x - zentrum.x);
    let s = bogen_spanne(von, ziel, zentrum, uhrzeigersinn);
    let n = (s / bogen_schritt(r)).ceil().clamp(1.0, 10_000.0) as usize;
    let mut punkte: Vec<Punkt> = (1..n)
        .map(|i| {
            let w = w0 + richtung * s * i as f64 / n as f64;
            zentrum.plus(Punkt::neu(w.cos(), w.sin()).mal(r))
        })
        .collect();
    // Das Ziel exakt treffen statt über cos/sin anzunähern.
    punkte.push(ziel);
    let tangente = |w: f64| Punkt::neu(-w.sin(), w.cos()).mal(richtung);
    Ok((tangente(w0), tangente(w0 + richtung * s), punkte))
}

/// Mittelpunkt eines Bogens von `von` nach `ziel` mit Radius `r`; die
/// Flags wählen einen der vier möglichen Bögen wie bei SVG.
pub(crate) fn bogen_mitte(
    von: Punkt,
    ziel: Punkt,
    r: f64,
    uhrzeigersinn: bool,
    gross: bool,
) -> Result<Punkt, FormFehler> {
    let sehne = ziel.minus(von);
    let d = sehne.laenge();
    if d < 1e-6 || r < d / 2.0 - 1e-6 || !r.is_finite() {
        return Err(FormFehler::BogenUnmoeglich);
    }
    let r = r.max(d / 2.0);
    let h = (r * r - d * d / 4.0).max(0.0).sqrt();
    let mitte = von.plus(sehne.mal(0.5));
    let quer = Punkt::neu(-sehne.y / d, sehne.x / d);
    let zentrum = mitte.plus(quer.mal(h));
    if (bogen_spanne(von, ziel, zentrum, uhrzeigersinn) > PI + 1e-9) != gross {
        return Ok(mitte.minus(quer.mal(h)));
    }
    Ok(zentrum)
}

/// Überstrichener Winkel (0 bis 2π) eines Bogens um `zentrum`. Winkel
/// wachsen auf dem Bildschirm (y nach unten) im Uhrzeigersinn.
pub(crate) fn bogen_spanne(von: Punkt, ziel: Punkt, zentrum: Punkt, uhrzeigersinn: bool) -> f64 {
    let richtung = if uhrzeigersinn { 1.0 } else { -1.0 };
    let w0 = (von.y - zentrum.y).atan2(von.x - zentrum.x);
    let w1 = (ziel.y - zentrum.y).atan2(ziel.x - zentrum.x);
    ((w1 - w0) * richtung).rem_euclid(TAU)
}

/// Teilt eine kubische Bézierkurve gleichmäßig auf. Die Schrittzahl folgt aus
/// der Fehlerschranke für lineare Interpolation: Fehler ≤ ¾·max|Δ²P| / n².
fn kurve_flach(p0: Punkt, p1: Punkt, p2: Punkt, p3: Punkt) -> Vec<Punkt> {
    let dd1 = p0.minus(p1.mal(2.0)).plus(p2).laenge();
    let dd2 = p1.minus(p2.mal(2.0)).plus(p3).laenge();
    let dd = dd1.max(dd2);
    let n = (0.75 * dd / FLACH_TOLERANZ_MM)
        .sqrt()
        .ceil()
        .clamp(1.0, 1000.0) as usize;
    (1..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            p0.mal(u * u * u)
                .plus(p1.mal(3.0 * u * u * t))
                .plus(p2.mal(3.0 * u * t * t))
                .plus(p3.mal(t * t * t))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass::TOLERANZ_MM;

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    #[test]
    fn ein_rechteck_80_x_50_hat_vier_ecken_und_260_mm_umfang() {
        let k = Form::Rechteck {
            breite: mm(80.0),
            hoehe: mm(50.0),
            eckenradius: mm(0.0),
        }
        .kontur()
        .unwrap();
        assert_eq!(k.len(), 4);
        assert!(k.ecken().iter().all(|e| *e));
        assert!((k.umfang() - 260.0).abs() < TOLERANZ_MM);
    }

    #[test]
    fn abgerundete_ecken_verkuerzen_den_umfang_um_den_erwarteten_betrag() {
        // Jede abgerundete Ecke spart 2r gerade Strecke und fügt einen
        // Viertelkreis (πr/2) hinzu.
        let r = 5.0;
        let k = Form::Rechteck {
            breite: mm(80.0),
            hoehe: mm(50.0),
            eckenradius: mm(r),
        }
        .kontur()
        .unwrap();
        let erwartet = 260.0 - 8.0 * r + 2.0 * PI * r;
        assert!((k.umfang() - erwartet).abs() < 0.05, "{}", k.umfang());
        assert!(k.ecken().iter().all(|e| !*e), "abgerundet hat keine Knicke");
    }

    #[test]
    fn ein_kreis_mit_100_mm_durchmesser_hat_den_richtigen_umfang() {
        let k = Form::Kreis {
            durchmesser: mm(100.0),
        }
        .kontur()
        .unwrap();
        assert!((k.umfang() - PI * 100.0).abs() < 0.05);
        let (min, max) = k.grenzen();
        assert!((max.x - min.x - 100.0).abs() < 0.01);
    }

    #[test]
    fn ein_zu_grosser_eckenradius_wird_abgewiesen() {
        let f = Form::Rechteck {
            breite: mm(20.0),
            hoehe: mm(10.0),
            eckenradius: mm(6.0),
        };
        assert_eq!(f.kontur(), Err(FormFehler::EckenradiusZuGross));
    }

    #[test]
    fn formen_unter_1_mm_oder_ueber_5_m_werden_abgewiesen() {
        assert_eq!(
            Form::Kreis {
                durchmesser: mm(0.5)
            }
            .kontur(),
            Err(FormFehler::GroesseAusserhalb)
        );
        assert_eq!(
            Form::Ellipse {
                breite: mm(6000.0),
                hoehe: mm(10.0)
            }
            .kontur(),
            Err(FormFehler::GroesseAusserhalb)
        );
    }

    #[test]
    fn ein_pfad_aus_linien_und_einer_kurve_erkennt_nur_echte_knicke() {
        // Unterseite gerade, oben ein glatter Bogen (wie eine Taschenklappe).
        let pfad = Pfad {
            start: Punkt::neu(0.0, 40.0),
            elemente: vec![
                PfadElement::Linie(Punkt::neu(0.0, 20.0)),
                PfadElement::Kurve {
                    k1: Punkt::neu(0.0, -5.0),
                    k2: Punkt::neu(60.0, -5.0),
                    ziel: Punkt::neu(60.0, 20.0),
                },
                PfadElement::Linie(Punkt::neu(60.0, 40.0)),
            ],
        };
        let k = Form::Pfad(pfad).kontur().unwrap();
        let knicke = k.ecken().iter().filter(|e| **e).count();
        // Ecken unten links und unten rechts; die Übergänge Linie → Kurve
        // sind tangential und zählen nicht.
        assert_eq!(knicke, 2);
        assert!(k.ist_ecke(0));
    }

    fn t_pocket(b: f64, h: f64, u: f64, fh: f64) -> Result<Kontur, FormFehler> {
        Form::TPocket {
            breite: mm(b),
            hoehe: mm(h),
            fluegel_ueberstand: mm(u),
            fluegel_hoehe: mm(fh),
        }
        .kontur()
    }

    #[test]
    fn eine_t_pocket_hat_acht_ecken_und_einen_schmaleren_steg() {
        // Springfield-Maß: 101,6 × 44,5 mm, Flügel 12,7 mm, 12 mm hoch.
        let k = t_pocket(101.6, 44.5, 12.7, 12.0).unwrap();
        assert_eq!(k.len(), 8);
        assert!(k.ecken().iter().all(|&e| e));
        let (von, bis) = k.grenzen();
        assert!((bis.x - von.x - 101.6).abs() < 1e-9);
        assert!((bis.y - von.y - 44.5).abs() < 1e-9);
        // Fläche: Querbalken plus Steg darunter.
        let soll = 101.6 * 12.0 + (101.6 - 2.0 * 12.7) * (44.5 - 12.0);
        assert!((k.flaeche_mit_vorzeichen().abs() - soll).abs() < 1e-6);
    }

    #[test]
    fn zu_breite_oder_zu_hohe_fluegel_werden_abgewiesen() {
        // Steg bliebe nur 6 mm breit.
        assert_eq!(
            t_pocket(80.0, 40.0, 37.0, 10.0),
            Err(FormFehler::FluegelZuGross)
        );
        // Flügel so hoch wie das ganze Fach: das wäre ein Rechteck.
        assert_eq!(
            t_pocket(80.0, 40.0, 10.0, 40.0),
            Err(FormFehler::FluegelZuGross)
        );
    }

    fn bogen(ziel: (f64, f64), r: f64, uhrzeigersinn: bool, gross: bool) -> PfadElement {
        PfadElement::Bogen {
            ziel: Punkt::neu(ziel.0, ziel.1),
            radius: mm(r),
            uhrzeigersinn,
            gross,
        }
    }

    #[test]
    fn ein_halbkreisbogen_mit_radius_25_ist_so_lang_wie_ein_halber_kreis() {
        let p = Pfad {
            start: Punkt::neu(0.0, 0.0),
            elemente: vec![bogen((50.0, 0.0), 25.0, true, false)],
        };
        let l = p.linienzug().unwrap();
        let laenge: f64 = l.punkte.windows(2).map(|w| w[0].abstand(w[1])).sum();
        assert!((laenge - PI * 25.0).abs() < 0.05, "{laenge}");
        // Im Uhrzeigersinn von links nach rechts wölbt er sich nach oben
        // (y zeigt nach unten).
        assert!(l.punkte.iter().all(|q| q.y <= TOLERANZ_MM));
        assert!(l.punkte.iter().any(|q| (q.y + 25.0).abs() < 0.02));
    }

    #[test]
    fn alle_punkte_eines_bogens_liegen_im_radius_um_den_mittelpunkt() {
        // Kleiner Bogen mit 40 mm Radius über eine 30-mm-Sehne, gegen den
        // Uhrzeigersinn: wölbt sich nach unten.
        let p = Pfad {
            start: Punkt::neu(0.0, 0.0),
            elemente: vec![bogen((30.0, 0.0), 40.0, false, false)],
        };
        let l = p.linienzug().unwrap();
        let h = (40.0f64 * 40.0 - 15.0 * 15.0).sqrt();
        let zentrum = Punkt::neu(15.0, -h);
        for q in &l.punkte {
            assert!((q.abstand(zentrum) - 40.0).abs() < 1e-6, "{q:?}");
        }
        assert!(l.punkte.iter().all(|q| q.y >= -TOLERANZ_MM));
    }

    #[test]
    fn der_grosse_bogen_geht_um_mehr_als_den_halbkreis() {
        let laenge = |gross| {
            let p = Pfad {
                start: Punkt::neu(0.0, 0.0),
                elemente: vec![bogen((30.0, 0.0), 20.0, true, gross)],
            };
            let l = p.linienzug().unwrap();
            l.punkte.windows(2).map(|w| w[0].abstand(w[1])).sum::<f64>()
        };
        let klein = laenge(false);
        let gross = laenge(true);
        assert!((klein + gross - TAU * 20.0).abs() < 0.1);
        assert!(gross > PI * 20.0 && klein < PI * 20.0);
    }

    #[test]
    fn ein_zu_kleiner_radius_wird_abgewiesen() {
        let p = Pfad {
            start: Punkt::neu(0.0, 0.0),
            elemente: vec![bogen((50.0, 0.0), 20.0, true, false)],
        };
        assert_eq!(p.linienzug(), Err(FormFehler::BogenUnmoeglich));
    }

    #[test]
    fn eine_taschenklappe_aus_linien_und_bogen_hat_ecken_nur_an_den_knicken() {
        // Rechteck 60 × 30, unten ein Bogen mit 40 mm Radius statt der Kante.
        let pfad = Pfad {
            start: Punkt::neu(0.0, 0.0),
            elemente: vec![
                PfadElement::Linie(Punkt::neu(60.0, 0.0)),
                PfadElement::Linie(Punkt::neu(60.0, 30.0)),
                bogen((0.0, 30.0), 40.0, true, false),
            ],
        };
        let k = Form::Pfad(pfad).kontur().unwrap();
        assert_eq!(k.ecken().iter().filter(|e| **e).count(), 4);
        let (_, bis) = k.grenzen();
        // Der Bogen wölbt sich nach unten über die 30 mm hinaus.
        let h = 40.0 - (40.0f64 * 40.0 - 30.0 * 30.0).sqrt();
        assert!((bis.y - (30.0 + h)).abs() < 0.02, "{}", bis.y);
    }

    #[test]
    fn eine_offene_linie_hat_an_beiden_enden_einen_knick() {
        let p = Pfad {
            start: Punkt::neu(0.0, 0.0),
            elemente: vec![
                PfadElement::Linie(Punkt::neu(40.0, 0.0)),
                PfadElement::Linie(Punkt::neu(40.0, 20.0)),
            ],
        };
        let l = p.linienzug().unwrap();
        assert_eq!(l.punkte.len(), 3);
        assert_eq!(l.ecke, vec![true, true, true]);
        let leer = Pfad {
            start: Punkt::neu(0.0, 0.0),
            elemente: vec![],
        };
        assert_eq!(leer.linienzug(), Err(FormFehler::PfadLeer));
    }
}
