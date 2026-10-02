//! Sicheres Schreiben von Dateien (Playbook 04).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Schreibt `inhalt` atomar nach `ziel`: erst in eine temporäre Datei im
/// selben Ordner, `fsync`, dann umbenennen. Bei einem Absturz bleibt also
/// entweder die alte oder die neue Datei vollständig erhalten.
///
/// Eine vorhandene Datei wird nur überschrieben, wenn `ueberschreiben`
/// gesetzt ist.
pub fn atomar_schreiben(ziel: &Path, inhalt: &[u8], ueberschreiben: bool) -> io::Result<()> {
    if !ueberschreiben && ziel.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Die Datei gibt es schon.",
        ));
    }
    let ordner = match ziel.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let name = ziel
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Kein Dateiname."))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(format!(".lepaca-{}.tmp", std::process::id()));
    let tmp = ordner.join(tmp_name);

    let ergebnis = (|| {
        let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        f.write_all(inhalt)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, ziel)?;
        // Den Ordner synchronisieren, damit die Umbenennung selbst dauerhaft ist
        // (unter Windows nicht möglich und nicht nötig).
        #[cfg(unix)]
        File::open(ordner)?.sync_all()?;
        Ok(())
    })();
    if ergebnis.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    ergebnis
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testordner(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("lepaca-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn eine_neue_datei_wird_vollstaendig_geschrieben() {
        let d = testordner("neu");
        let ziel = d.join("teil.svg");
        atomar_schreiben(&ziel, b"hallo", false).unwrap();
        assert_eq!(fs::read(&ziel).unwrap(), b"hallo");
        assert_eq!(fs::read_dir(&d).unwrap().count(), 1, "keine Reste");
        fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn eine_vorhandene_datei_wird_ohne_erlaubnis_nicht_ueberschrieben() {
        let d = testordner("vorhanden");
        let ziel = d.join("teil.svg");
        fs::write(&ziel, b"alt").unwrap();
        let fehler = atomar_schreiben(&ziel, b"neu", false).unwrap_err();
        assert_eq!(fehler.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&ziel).unwrap(), b"alt");
        atomar_schreiben(&ziel, b"neu", true).unwrap();
        assert_eq!(fs::read(&ziel).unwrap(), b"neu");
        fs::remove_dir_all(d).unwrap();
    }
}
