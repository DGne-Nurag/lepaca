# Playbook 06 – Release und Lieferkette

## Abhängigkeiten

Bevor eine neue Abhängigkeit aufgenommen wird, wird geprüft und im PR notiert:
- Wird sie wirklich gebraucht, oder reichen wenige Zeilen eigener Code?
- Wird sie aktiv gepflegt (letzte Releases, offene Sicherheitsmeldungen, Anzahl Maintainer)?
- Ist die Lizenz verträglich?
- Wie groß ist der transitive Abhängigkeitsbaum?

Regeln:
- Versionen über **Lockfiles gepinnt**, Lockfiles sind eingecheckt.
- Automatische Schwachstellen-Scans (z. B. `cargo audit`/`osv-scanner` je nach Stack) in CI; bekannte kritische Lücken blockieren den Merge.
- Update-Bot (Dependabot/Renovate) für regelmäßige, kleine Updates statt seltener großer Sprünge.
- GitHub Actions und andere CI-Bausteine auf Commit-SHA gepinnt, Token-Rechte minimal (`permissions:` explizit).

## Builds

- **Reproduzierbar**: gleicher Quellstand ergibt gleiche Artefakte; Build-Umgebung in Containern bzw. festgelegten Runnern.
- Release-Builds nur aus CI, nie von Entwicklerrechnern.
- SBOM (Software Bill of Materials) wird pro Release erzeugt und veröffentlicht.
- Build-Provenienz (z. B. SLSA-Attestierung) für Release-Artefakte.

## Signaturen

- Alle Release-Artefakte werden signiert: Linux (Flatpak-Repo-Signatur, AppImage-Signatur, Prüfsummen mit Signatur), Windows (Authenticode), macOS (Developer ID + Notarisierung).
- Signaturschlüssel liegen in einem Secrets-Store/HSM; Zugriff nur für den Release-Workflow.

## Verteilung

| Plattform | Kanal (Priorität) |
|---|---|
| Linux | Flatpak (Flathub), AppImage, später distributionseigene Pakete |
| Windows | Signiertes Installationspaket (MSI/MSIX), optional winget |
| macOS | Signiertes, notarisiertes DMG, optional Homebrew Cask |

## Updates

- Update-Prüfung ist **Opt-in** (siehe Playbook 01). Bei Paketmanagern übernimmt der Paketmanager die Updates.
- Eingebauter Updater (falls nötig) prüft Signaturen gegen einen fest eingebauten Schlüssel, unterstützt Schlüsselrotation und verhindert Downgrades.
- Release Notes in Alltagssprache: was ist neu, was ist behoben, gibt es Sicherheitskorrekturen.

## Versionierung

- Semantic Versioning. Dateiformat-Version wird unabhängig geführt (Playbook 04).
- Sicherheitsfixes werden für die aktuelle Hauptversion zeitnah ausgeliefert.
