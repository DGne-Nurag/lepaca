# Playbook 05 – Entwicklungs-Workflow

## Git

- `main` ist immer releasefähig und geschützt: kein direkter Push, kein Force-Push.
- Arbeit in kurzlebigen Branches (`feat/…`, `fix/…`, `docs/…`, `sec/…`), Merge per Pull Request.
- Commit-Nachrichten im Format Conventional Commits (`feat: Nahtlöcher entlang Kurve`), damit Changelog automatisch entsteht.
- Signierte Commits (SSH- oder GPG-Signatur) für alle Maintainer.

## Pull Requests

- Klein und fokussiert, eine Sache pro PR.
- Beschreibung: Vorher/Nachher in Worten, bei UI ein Screenshot, Verweis auf Issue.
- PR-Vorlage enthält die Security-Checkliste (Playbook 01) und die Usability-Checkliste (Playbook 02).
- Mindestens ein Review. Zweites Review für: Parser, Netzwerkcode, Krypto/Signaturen, `unsafe`/nativen Code, Build- und Release-Konfiguration.
- CI muss grün sein. Tests werden nie deaktiviert, um grün zu werden.

## Tests

| Ebene | Was | Wann |
|---|---|---|
| Unit | Geometrie-Kern, Maßberechnung, Befehle | jeder PR |
| Eigenschaften (property-based) | Geometrie-Invarianten (Offset-Abstand, Lochabstand, Rundungsfehler) | jeder PR |
| Roundtrip | Speichern → Laden → identisch; Export → Import → maßgleich | jeder PR |
| Fuzzing | Alle Parser | täglich, kurz auch im PR |
| Visuelle Regression | Rendering von Referenzmustern | jeder PR mit Rendering-Änderung |
| Plattform | Build + Tests auf Linux, Windows, macOS | jeder PR |
| Manuell/Nutzer | Aufgabentests mit Handwerker:innen | vor größeren Releases |

- Fehlerbehebung beginnt mit einem Test, der den Fehler reproduziert.
- Toleranzen für Gleitkommavergleiche sind zentral definiert, nicht pro Test erfunden.

## Definition of Done

Eine Änderung ist fertig, wenn:
- [ ] Code reviewt und CI auf allen Plattformen grün ist,
- [ ] Tests die neue Funktion bzw. den Fehler abdecken,
- [ ] Security- und Usability-Checkliste abgehakt sind,
- [ ] Texte übersetzbar sind und deutsche + englische Texte vorliegen,
- [ ] Nutzer-Dokumentation und Changelog aktualisiert sind,
- [ ] bei schwer umkehrbaren Entscheidungen ein ADR existiert.

## Arbeiten mit Claude

- Claude folgt diesen Playbooks wie alle anderen. Die Playbooks werden im Repository aus `CLAUDE.md` verlinkt.
- Claude öffnet PRs als Entwurf, treibt CI auf grün und fordert Review an; Merge entscheidet ein Mensch.
- Keine Secrets in Prompts, Issues oder Projektdateien.
