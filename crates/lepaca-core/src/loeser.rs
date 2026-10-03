//! Allgemeiner Löser für Maße und Bedingungen.
//!
//! Ein Gleichungssystem besteht aus Gleichungen `wert(v) = 0` über einem
//! Vektor von Unbekannten (Koordinaten in mm). Der Löser verschiebt die
//! Unbekannten so wenig wie möglich, bis alle Gleichungen erfüllt sind.
//! Genutzt von der Skizze ([`crate::skizze`]) und von den Ausschnitten
//! ([`crate::ausschnitt`]).

/// Größte Restabweichung, ab der eine Gleichung als erfüllt gilt (mm bzw.
/// mm-skaliert bei Winkeln). Weit unter der zentralen Toleranz.
pub(crate) const LOES_TOLERANZ: f64 = 1e-9;
const MAX_SCHRITTE: usize = 100;

/// Wert einer Gleichung (Index) für die Unbekannten.
pub(crate) type Wert<'a> = Box<dyn Fn(usize, &[f64]) -> f64 + 'a>;

/// Ein Gleichungssystem: je Gleichung die Unbekannten, von denen sie
/// abhängt, und ihr Wert.
pub(crate) struct System<'a> {
    /// Zahl der Unbekannten.
    pub anzahl: usize,
    /// Je Gleichung die Indizes der Unbekannten, von denen sie abhängt.
    pub variablen: Vec<Vec<usize>>,
    /// Wert der Gleichung `i` für die Unbekannten `v`.
    pub wert: Wert<'a>,
}

/// Dünn besetzte Jacobi-Matrix: je Gleichung (Variable, Ableitung).
pub(crate) type Zeilen = Vec<Vec<(usize, f64)>>;

impl System<'_> {
    fn gleichungen(&self) -> usize {
        self.variablen.len()
    }

    fn reste(&self, v: &[f64]) -> Vec<f64> {
        (0..self.gleichungen()).map(|i| (self.wert)(i, v)).collect()
    }

    pub(crate) fn jacobi(&self, v: &mut [f64]) -> Zeilen {
        self.variablen
            .iter()
            .enumerate()
            .map(|(i, vs)| {
                vs.iter()
                    .map(|&k| {
                        // Zentrale Differenz; Schrittweite passend zur Größe.
                        let alt = v[k];
                        let h = 1e-6 * alt.abs().max(1.0);
                        v[k] = alt + h;
                        let plus = (self.wert)(i, v);
                        v[k] = alt - h;
                        let minus = (self.wert)(i, v);
                        v[k] = alt;
                        (k, (plus - minus) / (2.0 * h))
                    })
                    .collect()
            })
            .collect()
    }
}

fn norm(r: &[f64]) -> f64 {
    r.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Löst (A + λ·I)·y = r für die symmetrische, positiv semidefinite Matrix A
/// per Cholesky. `None`, wenn die Matrix numerisch nicht positiv ist.
fn cholesky_loesen(a: &[f64], n: usize, lambda: f64, r: &[f64]) -> Option<Vec<f64>> {
    let mut l = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..=i {
            let mut s = a[i * n + j];
            if i == j {
                s += lambda;
            }
            for k in 0..j {
                s -= l[i * n + k] * l[j * n + k];
            }
            if i == j {
                if s <= 0.0 || !s.is_finite() {
                    return None;
                }
                l[i * n + i] = s.sqrt();
            } else {
                l[i * n + j] = s / l[j * n + j];
            }
        }
    }
    let mut y = r.to_vec();
    for i in 0..n {
        for k in 0..i {
            y[i] -= l[i * n + k] * y[k];
        }
        y[i] /= l[i * n + i];
    }
    for i in (0..n).rev() {
        for k in (i + 1)..n {
            y[i] -= l[k * n + i] * y[k];
        }
        y[i] /= l[i * n + i];
    }
    Some(y)
}

/// Gedämpftes Gauß-Newton-Verfahren mit Schritt kleinster Norm: Bei einem
/// nicht vollständig bestimmten System bewegt sich so nur, was sich
/// bewegen muss. `false`, wenn sich die Gleichungen widersprechen.
pub(crate) fn loese(s: &System<'_>, v: &mut [f64]) -> bool {
    let anzahl = s.gleichungen();
    if anzahl == 0 {
        return true;
    }
    let mut r = s.reste(v);
    let mut lambda_faktor = 1e-12;
    for _ in 0..MAX_SCHRITTE {
        if r.iter().all(|x| x.abs() < LOES_TOLERANZ) {
            return true;
        }
        let j = s.jacobi(v);
        // A = J·Jᵀ (anzahl × anzahl), aus den dünnen Zeilen.
        let mut a = vec![0.0; anzahl * anzahl];
        let mut dicht = vec![0.0; s.anzahl];
        for p in 0..anzahl {
            for &(k, d) in &j[p] {
                dicht[k] = d;
            }
            for q in 0..=p {
                let summe: f64 = j[q].iter().map(|&(k, d)| d * dicht[k]).sum();
                a[p * anzahl + q] = summe;
                a[q * anzahl + p] = summe;
            }
            for &(k, _) in &j[p] {
                dicht[k] = 0.0;
            }
        }
        let spur = (0..anzahl)
            .map(|i| a[i * anzahl + i])
            .fold(0.0f64, f64::max)
            .max(1e-12);
        let alt = norm(&r);
        let mut weiter = false;
        while lambda_faktor < 1e3 {
            let Some(y) = cholesky_loesen(&a, anzahl, lambda_faktor * spur, &r) else {
                lambda_faktor *= 100.0;
                continue;
            };
            // δ = −Jᵀ·y
            let mut delta = vec![0.0; s.anzahl];
            for (p, zeile) in j.iter().enumerate() {
                for &(k, d) in zeile {
                    delta[k] -= d * y[p];
                }
            }
            let mut schritt = 1.0;
            while schritt > 1e-3 {
                let neu: Vec<f64> = v.iter().zip(&delta).map(|(x, d)| x + schritt * d).collect();
                let r_neu = s.reste(&neu);
                if norm(&r_neu) < alt {
                    v.copy_from_slice(&neu);
                    r = r_neu;
                    weiter = true;
                    break;
                }
                schritt /= 2.0;
            }
            if weiter {
                lambda_faktor = (lambda_faktor / 10.0).max(1e-12);
                break;
            }
            lambda_faktor *= 100.0;
        }
        if !weiter {
            break;
        }
    }
    r.iter().all(|x| x.abs() < LOES_TOLERANZ * 1e3)
}

/// Rang der Jacobi-Matrix und eine Basis ihres Kerns (die Richtungen, in
/// die sich die Unbekannten noch bewegen lassen).
pub(crate) fn rang_und_kern(j: &Zeilen, spalten: usize) -> (usize, Vec<Vec<f64>>) {
    let zeilen = j.len();
    let mut a = vec![0.0; zeilen * spalten];
    for (p, z) in j.iter().enumerate() {
        let groesste = z.iter().map(|&(_, d)| d.abs()).fold(0.0f64, f64::max);
        if groesste < 1e-12 {
            continue;
        }
        for &(k, d) in z {
            a[p * spalten + k] = d / groesste;
        }
    }
    // Reduzierte Stufenform mit Spaltenpivotsuche.
    let mut pivots: Vec<usize> = Vec::new();
    let mut zeile = 0;
    for s in 0..spalten {
        if zeile >= zeilen {
            break;
        }
        let (best, wert) = (zeile..zeilen)
            .map(|p| (p, a[p * spalten + s].abs()))
            .fold((zeile, 0.0), |b, x| if x.1 > b.1 { x } else { b });
        if wert < 1e-7 {
            continue;
        }
        for k in 0..spalten {
            a.swap(zeile * spalten + k, best * spalten + k);
        }
        let p = a[zeile * spalten + s];
        for k in 0..spalten {
            a[zeile * spalten + k] /= p;
        }
        for q in 0..zeilen {
            if q == zeile {
                continue;
            }
            let f = a[q * spalten + s];
            if f.abs() < 1e-15 {
                continue;
            }
            for k in 0..spalten {
                a[q * spalten + k] -= f * a[zeile * spalten + k];
            }
        }
        pivots.push(s);
        zeile += 1;
    }
    let rang = pivots.len();
    let mut ist_pivot = vec![false; spalten];
    for &s in &pivots {
        ist_pivot[s] = true;
    }
    let kern = (0..spalten)
        .filter(|&f| !ist_pivot[f])
        .map(|f| {
            let mut x = vec![0.0; spalten];
            x[f] = 1.0;
            for (z, &s) in pivots.iter().enumerate() {
                x[s] = -a[z * spalten + f];
            }
            x
        })
        .collect();
    (rang, kern)
}
