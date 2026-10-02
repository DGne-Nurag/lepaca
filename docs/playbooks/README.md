# lepaca Playbooks – Grundregeln

lepaca ist ein CAD-Programm für Lederhandwerk (Schnittmuster, Nähte, Stanzlöcher, Nesting, Druck/Export), vergleichbar mit Leathercraft CAD, aber plattformübergreifend (Linux, Windows, macOS).

Diese Playbooks sind die verbindlichen Grundregeln für alle, die an lepaca arbeiten, Menschen wie Claude. Wer von einer Regel abweicht, begründet das schriftlich (im PR oder in einem ADR, siehe `03-architektur-plattform.md`).

## Die zehn Kernregeln

1. **Sicher als Standard.** Jede Funktion ist ohne Konfiguration sicher. Unsichere Optionen gibt es nur explizit, sichtbar und begründet.
2. **Jede Datei ist feindlich.** Importierte und geöffnete Dateien (eigenes Format, DXF, SVG, PDF, Bilder) werden wie Eingaben aus dem Internet behandelt: validiert, begrenzt, isoliert.
3. **Keine Netzwerkverbindung ohne Zustimmung.** Kein Telemetrie-, Update- oder Cloud-Aufruf ohne Opt-in. Offline ist ein vollwertiger Modus.
4. **Nutzerarbeit geht nie verloren.** Autosave, atomares Speichern, Undo für jede Aktion, Wiederherstellung nach Absturz.
5. **Maße sind heilig.** Was auf dem Bildschirm 100 mm sind, sind auf Papier und im Laser 100 mm. Einheiten sind immer sichtbar und nie mehrdeutig.
6. **Handwerker zuerst.** Fachbegriffe aus dem Lederhandwerk statt CAD-Jargon. Häufige Aufgaben in wenigen Schritten, Expertenfunktionen per Tastatur.
7. **Barrierefrei und international.** Tastaturbedienbar, skalierbar, kontrastreich, übersetzbar (mindestens Deutsch und Englisch).
8. **Plattformgleichheit.** Linux ist kein Nachzügler. Jede Funktion wird auf allen drei Plattformen gebaut und getestet.
9. **Offene, dokumentierte Formate.** Das eigene Dateiformat ist versioniert, dokumentiert und ohne lepaca lesbar. Kein Lock-in.
10. **Nachvollziehbare Lieferkette.** Abhängigkeiten minimal, gepinnt und geprüft. Builds reproduzierbar, Releases signiert.

## Die Playbooks

| Datei | Inhalt |
|---|---|
| [01-security.md](01-security.md) | Bedrohungsmodell, sichere Programmierung, Secrets, Netzwerk, Plugins |
| [02-usability.md](02-usability.md) | UX-Prinzipien, Präzision und Einheiten, Barrierefreiheit, Fehlermeldungen |
| [03-architektur-plattform.md](03-architektur-plattform.md) | Plattformunabhängigkeit, Schichten, Technologiewahl, ADRs |
| [04-dateiformate-daten.md](04-dateiformate-daten.md) | Eigenes Format, Import/Export, Parser-Härtung, Datensicherheit |
| [05-entwicklung-workflow.md](05-entwicklung-workflow.md) | Git, Reviews, Tests, Definition of Done |
| [06-release-lieferkette.md](06-release-lieferkette.md) | Abhängigkeiten, Builds, Signaturen, Verteilung, Updates |

## Pflege

- Playbooks sind lebende Dokumente. Änderungen laufen wie Code über Review.
- Konflikt zwischen Security und Komfort: Security gewinnt, aber wir suchen aktiv nach einer Lösung, die beides erfüllt, statt die Last auf die Nutzer abzuwälzen.
- Die Playbooks liegen im Repository unter `docs/playbooks/` und werden aus `CLAUDE.md` verlinkt.
