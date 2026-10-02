# lepaca – Hinweise für Claude

Lies vor jeder Arbeit die Grundregeln in [docs/playbooks/README.md](docs/playbooks/README.md) und das passende Playbook. Sie gelten für Claude wie für alle anderen.

- Kommunikation, Commits, Tests und Doku auf Deutsch; Code-Bezeichner dürfen deutsch sein, wenn sie Fachbegriffe aus dem Lederhandwerk sind.
- Architekturentscheidungen stehen in [docs/adr/](docs/adr/). Schwer umkehrbare Entscheidungen brauchen ein neues ADR.
- Der Maintainer reviewt Rust nicht Zeile für Zeile. Tests deshalb so benennen und schreiben, dass sie die Fachlogik in Leder-Begriffen beschreiben (`stichloecher_liegen_im_gleichen_abstand_zur_kante`).
- Längen-Parameter (Abstände, Breiten, Radien) im Kern immer als `Mm`, nie als nacktes `f64`. Koordinaten (`Punkt`) sind Millimeter. Vergleiche nur mit der zentralen Toleranz.
- `unsafe` ist verboten. Neue Abhängigkeiten nach Playbook 06 im PR begründen.
- PRs als Entwurf öffnen, CI auf allen drei Plattformen grün treiben, Merge entscheidet ein Mensch.
- Commits im Format Conventional Commits.
