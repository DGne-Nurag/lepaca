# ADR 0003: Lizenz GPL-3.0-or-later

- Status: angenommen
- Datum: 2026-10-02
- Beteiligte: Nurag (Entscheidung), Claude (Vorschlag)

## Kontext

Playbook 03 verlangt, die Projektlizenz früh festzulegen. lepaca soll offen bleiben: Niemand soll es übernehmen, verbessern und als geschlossenes Produkt vertreiben können.

## Entscheidung

lepaca steht unter der **GNU General Public License, Version 3 oder später** (SPDX: `GPL-3.0-or-later`). Der Lizenztext liegt in `LICENSE`.

## Betrachtete Alternativen

| Option | Pro | Contra |
|---|---|---|
| **GPL-3.0-or-later** | Ableger bleiben offen, Patentklausel, Schutz vor Tivoisierung | Nicht mit dem Mac App Store vereinbar; spätere Lizenzänderung braucht Zustimmung aller Beitragenden |
| MIT / Apache-2.0 | Maximal freizügig, einfache Weiterverwendung | Geschlossene Ableger möglich |
| MPL-2.0 | Mittelweg: geänderte Dateien bleiben offen | Ableger können geschlossene Teile hinzufügen |
| GPL-3.0-only | wie oben | Kein Umstieg auf künftige GPL-Versionen ohne alle Beitragenden |

## Folgen

- Abhängigkeiten müssen GPL-3.0-verträglich sein. MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0 und Unicode-3.0 sind es; die erlaubte Liste steht in `deny.toml` und wird in CI geprüft. GPL-2.0-only-Bibliotheken sind **nicht** verträglich.
- Verteilung auf macOS als signiertes DMG statt über den Mac App Store (passt zu Playbook 06).
- Mit lepaca erstellte Schnittmuster fallen nicht unter die GPL.
- Ein Contributor License Agreement (CLA) wird vorerst nicht eingeführt; bei Bedarf eigenes ADR.

## Security-Auswirkungen

Keine direkten. Offener Quellcode erlaubt unabhängige Prüfung.

## Usability-Auswirkungen

Keine direkten. Die Oberfläche zeigt später im Info-Dialog Lizenz und Bezugsquelle des Quellcodes.
