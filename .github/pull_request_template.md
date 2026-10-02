## Vorher / Nachher

Vorher:

Nachher:

## Security-Checkliste (Playbook 01)

- [ ] Verarbeitet der Code Daten aus Dateien, Netzwerk oder Zwischenablage? Dann: Grenzwerte, Validierung, Test mit kaputten Eingaben.
- [ ] Neue Abhängigkeit? Dann Playbook 06 befolgt und hier begründet.
- [ ] Neue Netzwerk- oder Dateisystemzugriffe? Dann Opt-in bzw. Begründung.
- [ ] Keine Secrets, keine sensiblen Daten in Logs.
- [ ] Bedrohungsmodell muss angepasst werden? Dann im selben PR.

## Usability-Checkliste (Playbook 02, nur bei UI-Änderungen)

- [ ] Versteht eine Person ohne CAD-Erfahrung die Funktion?
- [ ] Rückgängig machbar?
- [ ] Per Tastatur bedienbar, Fokus sichtbar?
- [ ] Einheiten sichtbar und korrekt?
- [ ] Fehlermeldungen nach dem Was/Warum/Was-tun-Schema?
- [ ] Texte übersetzbar?
- [ ] Screenshot oder kurze Aufnahme angehängt?
