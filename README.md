# lepaca

**Le**ather **Pa**ttern **CA**D: Schnittmuster für Lederhandwerk zeichnen, Stichlöcher setzen und in 1:1 drucken. Läuft unter Linux, Windows und macOS. Maskottchen ist ein französisches Alpaka (*le paca*).

Status: frühe Entwicklung, noch nicht benutzbar.

## Ziele für 0.1

- Grundformen zeichnen
- Stichlinie mit einstellbarem Kantenabstand und Lochabstand
- Eisenarten French, Diamond und Japan
- Große Teile auf A4 kacheln, mit Klebekanten

## Aufbau

| Pfad | Inhalt |
|---|---|
| `crates/lepaca-core` | Geometrie-Kern in Rust, ohne I/O |
| `crates/lepaca-io` | Export (SVG, gekacheltes PDF) und sicheres Speichern |
| `crates/lepaca-cli` | Kommandozeile (Export ohne Oberfläche) |
| `docs/playbooks` | Verbindliche Grundregeln (Security, Usability, Architektur …) |
| `docs/adr` | Architekturentscheidungen |

## Entwickeln

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Die Rust-Version ist in `rust-toolchain.toml` festgelegt und wird von `rustup` automatisch installiert.

## Lizenz

GPL-3.0-or-later, siehe [LICENSE](LICENSE) und [ADR 0003](docs/adr/0003-lizenz-gpl-3.md). Mit lepaca erstellte Schnittmuster gehören dir und fallen nicht unter die GPL.

## Test-Binaries

Jeder CI-Lauf erzeugt unsignierte Test-Binaries für Linux (x86_64, statisch), Windows (x86_64) und macOS (Apple Silicon). Zu finden im Reiter *Actions* → Lauf auswählen → *Artifacts*. Sie sind nur zum Ausprobieren gedacht; signierte Releases folgen später (Playbook 06).

```sh
lepaca pdf --breite 300 --hoehe 200 --radius 5 --eisen french --ausgabe teil.pdf
```

Unter macOS muss das unsignierte Binary einmal per Rechtsklick → Öffnen bzw. `xattr -d com.apple.quarantine lepaca` freigegeben werden.
