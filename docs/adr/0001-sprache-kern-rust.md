# ADR 0001: Rust als Sprache für Kern und Parser

- Status: angenommen
- Datum: 2026-10-02
- Beteiligte: Nurag (Entscheidung, Review), Claude (Umsetzung)

## Kontext

lepaca braucht einen Geometrie-Kern (Formen, Offsets, Stichlöcher, Kachelung, später Nesting) und Parser für fremde Formate (SVG, DXF, PDF, Bilder). Playbook 03 nennt als Kriterien in dieser Reihenfolge: Speichersicherheit, Plattformgleichheit, präzise 2D-Grafik, Reife/Lizenz, Paketierbarkeit.

Claude programmiert primär, Nurag reviewt. Nurags Sprachen: JS, TS, Go, Python, PHP.

## Entscheidung

Kern, Anwendungsschicht, Dateiformat und alle Parser werden in **Rust** geschrieben (stable, Edition 2024, Toolchain per `rust-toolchain.toml` gepinnt).

## Betrachtete Alternativen

| Option | Pro | Contra |
|---|---|---|
| **Rust** | Speichersicher ohne GC, reifes 2D-Geometrie-Ökosystem (z. B. kurbo, lyon, Clipping/Offset-Bibliotheken), cargo-fuzz, schlankes WASM | Nurag kann Rust noch nicht flüssig reviewen; Cross-Compiling aufwendiger als in Go |
| Go | Nurag kann jede Zeile reviewen, speichersicher, einfaches Cross-Compiling, natives Fuzzing | Dünneres Geometrie-Ökosystem, mehr Eigenbau; große WASM-Binaries |
| C++ (Qt 6) | Reifstes GUI-Toolkit inkl. Druck und Barrierefreiheit | Nicht speichersicher, verletzt Kriterium 1 für Parser |
| Kotlin/Compose | Speichersicher | JVM-Laufzeit, Desktop-Barrierefreiheit unter Linux schwach |

Performance war kein Entscheidungsgrund: Für Schnittmuster-Größenordnungen rechnen Rust und Go beide in Millisekunden. Rust hätte erst bei Nesting einen spürbaren Vorsprung.

## Folgen

- Reviewbarkeit: Nurag prüft den Kern vor allem über lesbare Tests (Unit, Property-based, Roundtrip) und Ergebnisse (exportierte PDF/SVG). Tests werden so geschrieben, dass sie die Fachlogik in Leder-Begriffen beschreiben.
- `unsafe` ist im Kern verboten (`#![forbid(unsafe_code)]`); Ausnahmen nur per ADR und mit zweitem Review (Playbook 05).
- Kern wird zuerst ohne GUI gebaut, mit kleinem CLI für Export, damit die Maßtreue früh prüfbar ist.
- Der Kern kann später nach WASM kompiliert werden (Browser-Version oder Kern direkt in der Oberfläche).
- Linux-CLI-Builds können vollständig statisch (musl) erfolgen.

## Security-Auswirkungen

- Speichersicherheit für alle Parser und den Kern ohne Garbage Collector.
- Fuzz-Targets mit cargo-fuzz für jeden Parser, in CI regelmäßig (Playbook 04).
- Abhängigkeiten über `cargo-deny`/`cargo-audit` geprüft und in `Cargo.lock` gepinnt (Playbook 06).

## Usability-Auswirkungen

- Keine direkten Auswirkungen auf Nutzer:innen. Kurze Rechenzeiten für Stichlöcher und Kachelung halten die Bedienung flüssig.
