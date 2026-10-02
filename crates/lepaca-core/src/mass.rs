//! Längen und Toleranzen.
//!
//! Kernregel 5: Maße sind heilig. Längen werden deshalb nie als nacktes
//! `f64` herumgereicht, sondern als [`Mm`], damit Einheiten nicht
//! verwechselt werden können.

use std::fmt;

/// Toleranz für Längenvergleiche, zentral festgelegt (Playbook 05).
/// Ein Tausendstel Millimeter liegt weit unter jeder Werkzeuggenauigkeit.
pub const TOLERANZ_MM: f64 = 0.001;

/// Eine Länge in Millimetern.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Mm(f64);

/// Fehler beim Anlegen einer Länge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassFehler {
    /// Der Wert ist keine endliche Zahl (NaN oder unendlich).
    NichtEndlich,
}

impl fmt::Display for MassFehler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MassFehler::NichtEndlich => write!(f, "Die Länge ist keine gültige Zahl."),
        }
    }
}

impl std::error::Error for MassFehler {}

impl Mm {
    /// Legt eine Länge an. NaN und unendliche Werte werden abgewiesen
    /// (Playbook 04: Zahlen sind immer endlich).
    pub fn neu(wert: f64) -> Result<Self, MassFehler> {
        if wert.is_finite() {
            Ok(Mm(wert))
        } else {
            Err(MassFehler::NichtEndlich)
        }
    }

    /// Der Zahlenwert in Millimetern.
    pub fn wert(self) -> f64 {
        self.0
    }

    /// Vergleicht zwei Längen mit der zentralen [`TOLERANZ_MM`].
    pub fn etwa_gleich(self, andere: Mm) -> bool {
        (self.0 - andere.0).abs() <= TOLERANZ_MM
    }
}

impl fmt::Display for Mm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} mm", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eine_laenge_zeigt_immer_ihre_einheit() {
        assert_eq!(Mm::neu(100.0).unwrap().to_string(), "100 mm");
    }

    #[test]
    fn ungueltige_zahlen_werden_abgewiesen() {
        assert_eq!(Mm::neu(f64::NAN), Err(MassFehler::NichtEndlich));
        assert_eq!(Mm::neu(f64::INFINITY), Err(MassFehler::NichtEndlich));
    }

    #[test]
    fn rundungsfehler_unter_einem_tausendstel_millimeter_zaehlen_als_gleich() {
        let a = Mm::neu(0.1 + 0.2).unwrap();
        let b = Mm::neu(0.3).unwrap();
        assert!(a.etwa_gleich(b));
        assert!(!a.etwa_gleich(Mm::neu(0.302).unwrap()));
    }
}
