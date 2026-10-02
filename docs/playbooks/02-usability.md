# Playbook 02 – Usability

## Für wen wir bauen

- **Hobby-Lederhandwerker:innen**, die kein CAD gelernt haben und ein Muster für eine Geldbörse zeichnen, anpassen und ausdrucken wollen.
- **Kleine Werkstätten**, die Muster wiederverwenden, skalieren, auf Leder nesten und an Laser/Plotter schicken.
- **Erfahrene Nutzer:innen**, die schnell und tastaturgesteuert arbeiten wollen.

Jede Funktion muss für die erste Gruppe verständlich sein, ohne die dritte auszubremsen.

## Prinzipien

### 1. Fachsprache des Handwerks

- Begriffe aus dem Lederhandwerk: Nahtlinie, Nahtzugabe, Stichabstand, Locheisen, Kante, Falz, Kleberand, Teil, Muster. Nicht: Offset-Polyline, Entity, Layer-Group.
- Wo CAD-Begriffe unvermeidbar sind, erklärt ein Tooltip sie in einem Satz.
- Werkzeuge spiegeln echte Arbeitsschritte: „Nahtlöcher entlang Kante setzen" statt „Punkte auf Pfad verteilen".

### 2. Präzision und Einheiten

- Einheit ist **immer sichtbar** (mm, cm, in). Interne Rechnung in einer festen Einheit (mm, Gleitkomma mit definierter Toleranz).
- Zahleneingaben akzeptieren Einheiten und einfache Rechnungen (`3,5 cm`, `1/4 in`, `40+2`). Dezimalkomma und -punkt werden je nach Locale korrekt verstanden.
- **Druck in 1:1 ist garantiert.** Jeder Ausdruck enthält ein Prüfquadrat/Lineal mit Maßangabe. Druckdialog warnt, wenn das System skaliert („An Seite anpassen").
- Fang- und Hilfsfunktionen (Raster, Objektfang, Maßketten) sind sichtbar an/aus.

### 3. Fehler verzeihen

- **Undo/Redo für jede Aktion**, unbegrenzt bzw. großzügig, auch über Speichern hinweg innerhalb einer Sitzung.
- **Autosave** und Wiederherstellung nach Absturz. Speichern ist atomar (temporäre Datei, dann umbenennen).
- Destruktive Aktionen (Löschen vieler Teile, Überschreiben) zeigen, was passiert, oder sind rückgängig machbar. Bevorzugt: rückgängig machbar statt Bestätigungsdialog.
- Keine modalen Dialogketten. Ein Dialog darf nie Arbeit blockieren, die woanders gespeichert werden müsste.

### 4. Gute Fehlermeldungen

Jede Fehlermeldung beantwortet drei Fragen in Alltagssprache:
1. **Was** ist passiert? („Die Datei konnte nicht geöffnet werden.")
2. **Warum**, soweit bekannt? („Sie enthält mehr als 1 Million Punkte, das Limit liegt bei 500.000.")
3. **Was kann ich tun?** („Datei in einem anderen Programm vereinfachen oder Limit in Einstellungen > Import erhöhen.")

Keine Stacktraces, Fehlercodes ohne Erklärung oder Schuldzuweisung an Nutzer:innen. Technische Details hinter „Details anzeigen".

### 5. Konsistenz und Plattformgefühl

- Plattformübliche Tastenkürzel (Strg ↔ Cmd), Dateidialoge und Menüs.
- Einheitliches Werkzeugverhalten: Esc bricht ab, Enter bestätigt, Rechtsklick zeigt Kontextmenü, Leertaste+Ziehen verschiebt die Ansicht.
- Werkzeuge zeigen in einer Statuszeile, was als Nächstes erwartet wird („Startpunkt der Naht wählen").

### 6. Erste Schritte

- Leerer Startbildschirm vermeiden: Vorlagen (z. B. Kartenetui) und „Neues Muster" direkt sichtbar.
- Interaktive Einführung optional, jederzeit überspringbar, später wieder aufrufbar.
- Beispielmuster werden mitgeliefert und sind offline verfügbar.

### 7. Performance ist Usability

- Ziel: Eingaben reagieren innerhalb von 100 ms, Zoom/Pan flüssig (60 fps) bei typischen Mustern.
- Lange Operationen (Nesting, großer Import) laufen im Hintergrund mit Fortschritt und Abbrechen-Knopf.

## Barrierefreiheit

- Alles per **Tastatur** erreichbar, sichtbarer Fokus.
- UI-Skalierung und HiDPI auf allen Plattformen; Schriftgröße folgt Systemeinstellungen.
- **Kontrast** mindestens WCAG AA. Hell- und Dunkelmodus. Information nie nur über Farbe (z. B. Naht vs. Schnittlinie zusätzlich über Linienart).
- Screenreader-Unterstützung für Menüs, Dialoge und Eigenschaftsfelder über die Accessibility-APIs des Toolkits.
- Farbschemata für Farbsehschwächen testen.

## Internationalisierung

- Alle Texte übersetzbar von Tag 1 an, keine zusammengesetzten Sätze im Code.
- Start mit Deutsch und Englisch. Zahlen-, Datums- und Einheitenformate locale-abhängig.
- Imperiale und metrische Einheiten gleichwertig.

## Usability-Checkliste für jeden PR mit UI-Änderung

- [ ] Versteht eine Person ohne CAD-Erfahrung die Funktion? (Begriffe, Tooltip, Statuszeile)
- [ ] Rückgängig machbar?
- [ ] Per Tastatur bedienbar, Fokus sichtbar?
- [ ] Einheiten sichtbar und korrekt?
- [ ] Fehlermeldungen nach dem Was/Warum/Was-tun-Schema?
- [ ] Texte übersetzbar?
- [ ] Screenshot oder kurze Aufnahme im PR, auf mindestens einer Plattform, bei größeren Änderungen auf Linux und einer weiteren.

## Validierung mit echten Nutzer:innen

- Vor größeren Funktionen: kurze Aufgabe mit 2 bis 3 Lederhandwerker:innen testen („Zeichne ein Kartenetui mit 4 mm Stichabstand und drucke es").
- Erkenntnisse als Notiz im zugehörigen Issue festhalten.
