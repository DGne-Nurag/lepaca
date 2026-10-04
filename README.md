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
| `crates/lepaca-cli` | Kommandozeile für Tests und Entwicklung (kein Nutzer-Download) |
| `apps/desktop` | Oberfläche (Tauri 2 + TypeScript, vorläufig nach ADR 0002) |
| `docs/playbooks` | Verbindliche Grundregeln (Security, Usability, Architektur …) |
| `docs/adr` | Architekturentscheidungen |

## Entwickeln

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Die Kommandozeile `lepaca-cli` ist ein Entwicklerwerkzeug für Tests und wird nicht als Download angeboten:

```sh
cargo run -p lepaca-cli -- pdf --breite 300 --hoehe 200 --radius 5 --eisen french --ausgabe teil.pdf
```

Die Rust-Version ist in `rust-toolchain.toml` festgelegt und wird von `rustup` automatisch installiert.

Die Oberfläche ist ein eigener Cargo-Workspace, damit der Kern ohne WebKitGTK testbar bleibt. Unter Linux braucht sie `libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`, `librsvg2-dev` und `libxdo-dev`, dazu Node.js 22.

```sh
cd apps/desktop
npm ci
npm run tauri dev                      # mit Neuladen beim Speichern
npm run tauri build -- --no-bundle     # ausführbare Datei ohne Installer
```

Im Fuß der App startet *Leistungstest* den Stresstest aus ADR 0002 (50 Teile, rund 20 000 Stichlöcher, Zoomen und Verschieben) und zeigt die Messwerte an.

## Lizenz

GPL-3.0-or-later, siehe [LICENSE](LICENSE) und [ADR 0003](docs/adr/0003-lizenz-gpl-3.md). Mit lepaca erstellte Schnittmuster gehören dir und fallen nicht unter die GPL.

## Test-Builds

Jeder CI-Lauf erzeugt unsignierte Test-Builds der Desktop-App (`lepaca-desktop-…`) für Linux (x86_64), Windows (x86_64) und macOS (Apple Silicon). Die App braucht unter Linux WebKitGTK 4.1 aus der Paketverwaltung, unter Windows WebView2 (ab Windows 11 vorhanden). Zu finden im Reiter *Actions* → Lauf auswählen → *Artifacts*. Sie sind nur zum Ausprobieren gedacht; signierte Releases folgen später (Playbook 06).

Unter macOS muss das unsignierte Binary einmal per Rechtsklick → Öffnen bzw. `xattr -d com.apple.quarantine lepaca-desktop` freigegeben werden.
