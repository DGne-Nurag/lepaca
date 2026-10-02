# Playbook 01 – Security

## Bedrohungsmodell (Kurzfassung)

lepaca ist eine Desktop-Anwendung, die Dateien aus fremden Quellen öffnet (Schnittmuster aus Foren, Shops, Tauschbörsen). Daraus folgt:

| Angriffsfläche | Risiko | Wichtigste Gegenmaßnahme |
|---|---|---|
| Geöffnete/importierte Dateien | Codeausführung, Absturz, Ressourcenerschöpfung über präparierte Dateien | Gehärtete Parser, Grenzwerte, Fuzzing (siehe Playbook 04) |
| Plugins/Skripte | Beliebiger Code mit Nutzerrechten | Kein Plugin-System ohne Sandbox-Konzept und ADR |
| Netzwerk (Updates, Vorlagen, Cloud) | Manipulierte Updates, Datenabfluss | Opt-in, TLS, signierte Inhalte |
| Druck/Export an Laser/Plotter | Gerätebefehle aus Dateiinhalten | Export erzeugt nur Geometrie, nie durchgereichte Rohbefehle |
| Lieferkette | Kompromittierte Abhängigkeiten | Playbook 06 |
| Lokale Daten | Leak von Kundendaten, Pfaden, Nutzernamen in Dateien | Datensparsamkeit, Metadaten-Kontrolle |

Das Modell wird bei jeder neuen Angriffsfläche (neues Format, Netzwerkfunktion, Plugin) aktualisiert, bevor gebaut wird.

## Regeln

### Code

- **Speichersichere Sprache bevorzugen** für alles, was fremde Daten verarbeitet. Unsafe/native Code nur gekapselt, begründet und mit Review durch eine zweite Person.
- **Keine dynamische Codeausführung** aus Dateiinhalten: kein `eval`, keine eingebetteten Skripte, keine Makros, keine Deserialisierung beliebiger Typen.
- **Keine Shell-Aufrufe mit zusammengesetzten Strings.** Externe Programme nur mit Argumentlisten, festen Pfaden und ohne Shell.
- **Pfade validieren.** Dateinamen aus Dateiinhalten (z. B. verlinkte Bilder) nie ungeprüft verwenden; Path-Traversal (`../`), absolute Pfade, Symlinks und Gerätedateien abweisen.
- **Ressourcen begrenzen.** Jede Schleife über Eingabedaten hat eine Obergrenze (Anzahl Objekte, Punkte, Verschachtelungstiefe, Dekompressionsgröße, Laufzeit).
- **Fehler sicher behandeln.** Fehlschlag bedeutet Abbruch mit klarer Meldung, nie „irgendwie weitermachen" mit halbgültigen Daten.
- **Compiler-Warnungen und Linter sind Fehler** in CI. Sanitizer (ASan/UBSan oder Äquivalent) laufen in den Tests.

### Netzwerk und Privatsphäre

- Standard ist **komplett offline**. Jede Netzwerkfunktion ist Opt-in, einzeln abschaltbar und in den Einstellungen sichtbar.
- Keine Telemetrie ohne ausdrückliche Zustimmung; wenn eingeführt, dann anonym, dokumentiert und mit Einsicht in die gesendeten Daten.
- Nur TLS mit Zertifikatsprüfung. Keine Option zum Abschalten der Prüfung.
- Heruntergeladene Inhalte (Updates, Vorlagenbibliotheken) werden per Signatur geprüft, nicht nur per Transport.

### Secrets

- Keine Schlüssel, Tokens oder Passwörter im Repository, auch nicht in Tests. Secret-Scanning läuft in CI.
- Signaturschlüssel liegen in einem Secrets-Store bzw. auf Hardware, nie auf Entwicklerrechnern im Klartext.
- Falls lepaca je Zugangsdaten speichert (z. B. Cloud-Sync): nur im Schlüsselbund des Betriebssystems (Secret Service/KWallet, Windows Credential Manager, macOS Keychain).

### Rechte

- lepaca läuft **nie mit Admin-/Root-Rechten** und fordert sie nicht an. Installation über Systempakete/Flatpak regelt das.
- Sandboxing nutzen, wo die Plattform es anbietet (Flatpak-Portale, macOS App Sandbox). Dateizugriff über Datei-Dialoge/Portale statt Vollzugriff.

### Plugins und Erweiterungen

- Vorerst **kein Plugin-System**. Wird eines gewünscht, braucht es vorher ein ADR mit Sandbox-Konzept (z. B. WebAssembly mit expliziten Fähigkeiten), Signaturmodell und Rechteanzeige für Nutzer.

### Logging

- Logs enthalten keine Dateiinhalte, Kundendaten oder vollständigen Nutzerpfade in Release-Builds.
- Absturzberichte werden nur nach Zustimmung und mit Vorschau versendet.

## Sicherheitslücken melden

- `SECURITY.md` im Repository mit Kontaktweg (privat, nicht öffentliches Issue) und Reaktionszeitziel.
- Behobene Lücken bekommen einen Eintrag im Changelog und, wo sinnvoll, eine CVE.

## Security-Checkliste für jeden PR

- [ ] Verarbeitet der Code Daten aus Dateien, Netzwerk oder Zwischenablage? Dann: Grenzwerte, Validierung, Test mit kaputten Eingaben.
- [ ] Neue Abhängigkeit? Dann Playbook 06 befolgt.
- [ ] Neue Netzwerk- oder Dateisystemzugriffe? Dann Opt-in bzw. Begründung.
- [ ] Keine Secrets, keine sensiblen Daten in Logs.
- [ ] Bedrohungsmodell muss angepasst werden? Dann im selben PR.
