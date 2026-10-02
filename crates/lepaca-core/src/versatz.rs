//! Parallele Linie nach innen (z. B. die Stichlinie im Kantenabstand).

use crate::form;
use crate::geometrie::{Kontur, Punkt};
use crate::mass::Mm;

/// Warum keine Parallele berechnet werden kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersatzFehler {
    /// Der Abstand ist für die Form zu groß: Die Linie würde in sich
    /// zusammenfallen oder sich selbst kreuzen.
    AbstandZuGross,
    /// Die Form hat eine Spitze, die genau zurückläuft (180°-Knick).
    Spitze,
}

/// Berechnet die Linie im Abstand `abstand` innerhalb der Kontur.
///
/// Jede Kante wird um `abstand` nach innen parallel verschoben; neue
/// Punkte sind die Schnittpunkte benachbarter verschobener Kanten. Kanten,
/// die dabei auf null schrumpfen oder sich umdrehen (z. B. ein Eckenradius
/// kleiner als der Kantenabstand), werden entfernt; an ihrer Stelle entsteht
/// eine Ecke. Ist das Ergebnis trotzdem ungültig (Selbstkreuzung, Umlauf
/// gekippt), gibt es einen Fehler statt einer halbgültigen Linie
/// (Playbook 01: Fehler sicher behandeln).
pub fn nach_innen(kontur: &Kontur, abstand: Mm) -> Result<Kontur, VersatzFehler> {
    let d = abstand.wert();
    let n = kontur.len();
    let s = kontur.flaeche_mit_vorzeichen().signum();

    // Verschobene Kanten: Startpunkt und Richtung.
    let mut kanten = Vec::with_capacity(n);
    for i in 0..n {
        let a = kontur.punkt(i);
        let Some(e) = kontur.punkt(i + 1).minus(a).normiert() else {
            return Err(VersatzFehler::Spitze);
        };
        let innen = Punkt::neu(-e.y, e.x).mal(s);
        kanten.push((a.plus(innen.mal(d)), e));
    }

    let schnitt = |a: usize, b: usize| -> Result<Punkt, VersatzFehler> {
        let (qa, ea) = kanten[a];
        let (qb, eb) = kanten[b];
        let kreuz = ea.kreuz(eb);
        if kreuz.abs() < 1e-12 {
            if ea.skalar(eb) < 0.0 {
                return Err(VersatzFehler::Spitze);
            }
            return Ok(qb);
        }
        let t = qb.minus(qa).kreuz(eb) / kreuz;
        Ok(qa.plus(ea.mal(t)))
    };

    // Kanten entfernen, bis keine mehr umgedreht ist.
    let mut behalten: Vec<usize> = (0..n).collect();
    loop {
        let m = behalten.len();
        if m < 3 {
            return Err(VersatzFehler::AbstandZuGross);
        }
        let mut punkte = Vec::with_capacity(m);
        for k in 0..m {
            punkte.push(schnitt(behalten[(k + m - 1) % m], behalten[k])?);
        }
        let umgedreht: Vec<bool> = (0..m)
            .map(|k| {
                let neu = punkte[(k + 1) % m].minus(punkte[k]);
                neu.skalar(kanten[behalten[k]].1) <= 1e-9
            })
            .collect();
        if umgedreht.iter().any(|u| *u) {
            if umgedreht.iter().all(|u| *u) {
                return Err(VersatzFehler::AbstandZuGross);
            }
            behalten = behalten
                .iter()
                .zip(&umgedreht)
                .filter(|(_, u)| !**u)
                .map(|(i, _)| *i)
                .collect();
            continue;
        }

        let knick = form::ECKEN_WINKEL_GRAD.to_radians().cos();
        let ecke: Vec<bool> = (0..m)
            .map(|k| {
                let vor = behalten[(k + m - 1) % m];
                let akt = behalten[k];
                let zusammengefallen = (vor + 1) % n != akt;
                kontur.ist_ecke(akt)
                    || (zusammengefallen && kanten[vor].1.skalar(kanten[akt].1) < knick)
            })
            .collect();
        let ergebnis = Kontur::neu(punkte, ecke).map_err(|_| VersatzFehler::AbstandZuGross)?;
        if ergebnis.flaeche_mit_vorzeichen() * s <= 0.0 || ergebnis.schneidet_sich_selbst() {
            return Err(VersatzFehler::AbstandZuGross);
        }
        return Ok(ergebnis);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::Form;
    use crate::mass::TOLERANZ_MM;
    use std::f64::consts::PI;

    fn mm(x: f64) -> Mm {
        Mm::neu(x).unwrap()
    }

    fn rechteck(b: f64, h: f64, r: f64) -> Kontur {
        Form::Rechteck {
            breite: mm(b),
            hoehe: mm(h),
            eckenradius: mm(r),
        }
        .kontur()
        .unwrap()
    }

    #[test]
    fn die_stichlinie_eines_rechtecks_liegt_ueberall_genau_im_kantenabstand() {
        let k = rechteck(80.0, 50.0, 0.0);
        let innen = nach_innen(&k, mm(3.0)).unwrap();
        let (min, max) = innen.grenzen();
        assert!((min.x - 3.0).abs() < TOLERANZ_MM && (min.y - 3.0).abs() < TOLERANZ_MM);
        assert!((max.x - 77.0).abs() < TOLERANZ_MM && (max.y - 47.0).abs() < TOLERANZ_MM);
        assert!(innen.ecken().iter().all(|e| *e), "Ecken bleiben Ecken");
    }

    #[test]
    fn die_stichlinie_eines_kreises_ist_ein_kleinerer_kreis() {
        let k = Form::Kreis {
            durchmesser: mm(60.0),
        }
        .kontur()
        .unwrap();
        let innen = nach_innen(&k, mm(4.0)).unwrap();
        let mitte = Punkt::neu(30.0, 30.0);
        for p in innen.punkte() {
            assert!((p.abstand(mitte) - 26.0).abs() < 0.02);
        }
        assert!((innen.umfang() - 2.0 * PI * 26.0).abs() < 0.05);
    }

    #[test]
    fn die_umlaufrichtung_spielt_keine_rolle() {
        let mut p = vec![
            Punkt::neu(0.0, 0.0),
            Punkt::neu(40.0, 0.0),
            Punkt::neu(40.0, 20.0),
            Punkt::neu(0.0, 20.0),
        ];
        let rechts = nach_innen(&Kontur::neu(p.clone(), vec![true; 4]).unwrap(), mm(2.0)).unwrap();
        p.reverse();
        let links = nach_innen(&Kontur::neu(p, vec![true; 4]).unwrap(), mm(2.0)).unwrap();
        assert_eq!(rechts.grenzen(), links.grenzen());
    }

    #[test]
    fn an_einer_einbuchtung_bleibt_der_abstand_zu_beiden_kanten_gleich() {
        // L-Form: die innere Ecke bei (20, 20) ist eine Einbuchtung.
        let p = vec![
            Punkt::neu(0.0, 0.0),
            Punkt::neu(20.0, 0.0),
            Punkt::neu(20.0, 20.0),
            Punkt::neu(40.0, 20.0),
            Punkt::neu(40.0, 40.0),
            Punkt::neu(0.0, 40.0),
        ];
        let k = Kontur::neu(p, vec![true; 6]).unwrap();
        let innen = nach_innen(&k, mm(3.0)).unwrap();
        assert!(innen.punkt(2).abstand(Punkt::neu(17.0, 23.0)) < TOLERANZ_MM);
    }

    #[test]
    fn ein_zu_grosser_kantenabstand_wird_gemeldet_statt_unsinn_zu_liefern() {
        let k = rechteck(20.0, 10.0, 0.0);
        assert_eq!(nach_innen(&k, mm(5.0)), Err(VersatzFehler::AbstandZuGross));
        assert_eq!(nach_innen(&k, mm(6.0)), Err(VersatzFehler::AbstandZuGross));
        assert!(nach_innen(&k, mm(4.9)).is_ok());
    }

    #[test]
    fn ist_der_eckenradius_kleiner_als_der_kantenabstand_wird_die_stichlinie_eckig() {
        let k = rechteck(60.0, 30.0, 2.0);
        let innen = nach_innen(&k, mm(3.0)).unwrap();
        assert_eq!(innen.len(), 4);
        assert!(innen.ecken().iter().all(|e| *e));
        let (min, max) = innen.grenzen();
        assert!(min.abstand(Punkt::neu(3.0, 3.0)) < TOLERANZ_MM);
        assert!(max.abstand(Punkt::neu(57.0, 27.0)) < TOLERANZ_MM);
    }

    #[test]
    fn auch_abgerundete_rechtecke_bleiben_bis_kurz_vor_der_mitte_gueltig() {
        let k = rechteck(60.0, 30.0, 8.0);
        for d in [1.0, 3.0, 8.0, 10.0, 14.0] {
            let innen = nach_innen(&k, mm(d)).unwrap();
            let (min, max) = innen.grenzen();
            assert!((min.y - d).abs() < 0.02 && (max.y - (30.0 - d)).abs() < 0.02);
        }
    }
}
