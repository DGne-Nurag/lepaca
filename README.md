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
