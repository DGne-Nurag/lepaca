# Playbook 03 – Architektur und Plattform

## Ziele

- Eine Codebasis für **Linux, Windows und macOS** (x86_64 und ARM64).
- Linux ist Erstplattform, nicht Portierung: Entwicklung und CI laufen dort zuerst.
- Klare Trennung zwischen Geometrie-Kern und Oberfläche, damit der Kern testbar und später auch ohne GUI nutzbar ist (CLI, Batch-Export).

## Schichten

```
┌──────────────────────────────────────────┐
│ UI (Fenster, Werkzeuge, Dialoge)         │  plattformnah, dünn
├──────────────────────────────────────────┤
│ Anwendung (Dokument, Undo, Befehle)      │  plattformunabhängig
├──────────────────────────────────────────┤
│ Kern (Geometrie, Nähte, Nesting, Maße)   │  rein, ohne I/O, voll getestet
├──────────────────────────────────────────┤
│ I/O (Format, Import/Export, Druck)       │  gehärtet, isolierbar
└──────────────────────────────────────────┘
```

Regeln:
- Der **Kern** hat keine Abhängigkeit zu UI, Dateisystem oder Netzwerk.
- Jede Nutzeraktion ist ein **Befehl** in der Anwendungsschicht. Damit sind Undo, Makro-Aufzeichnung und Tests einheitlich.
- **Parser** für fremde Formate sind eigene Module mit klarer Schnittstelle (Bytes rein, validiertes Modell oder Fehler raus), sodass sie später in einen separaten Prozess oder eine Sandbox verschoben werden können.
- Plattformspezifischer Code nur hinter Schnittstellen in einem eigenen Modul, nie verstreut über `#ifdef`/Plattformabfragen.

## Technologiewahl

Die Wahl von Sprache und GUI-Toolkit ist noch offen und wird per ADR entschieden. Kriterien in dieser Reihenfolge:

1. **Speichersicherheit** für Parser und Kern.
2. **Echte Plattformgleichheit** inklusive Linux (X11 und Wayland), HiDPI, Barrierefreiheit (AT-SPI, UIA, NSAccessibility).
3. **Präzise 2D-Grafik** (Bezierkurven, Antialiasing, Druck in exakter Größe, PDF/SVG-Ausgabe).
4. **Wartbarkeit und Reife** des Ökosystems, Lizenzverträglichkeit.
5. **Paketierbarkeit** (Flatpak/AppImage, MSI/MSIX, signiertes macOS-Bundle).

Kandidaten zur Bewertung im ADR (ohne Vorentscheidung): Rust mit Slint/egui/GTK-rs, C++ mit Qt 6, Kotlin/Compose Multiplatform. Elektron-artige Web-Stacks nur, wenn Performance und Druckgenauigkeit nachweislich reichen.

## Architecture Decision Records (ADR)

- Jede Entscheidung, die schwer rückgängig zu machen ist (Sprache, Toolkit, Dateiformat, Lizenz, Plugin-Modell, Netzwerkfunktion), bekommt ein ADR unter `docs/adr/NNNN-titel.md`.
- Aufbau: Kontext, Entscheidung, betrachtete Alternativen, Folgen, inklusive **Security- und Usability-Auswirkungen** als eigene Abschnitte.
- ADRs werden nicht gelöscht, sondern durch neue abgelöst.

## Plattformregeln

- CI baut und testet **jeden PR auf Linux, Windows und macOS**.
- Dateipfade nur über Pfad-APIs, nie String-Verkettung. Groß-/Kleinschreibung und Unicode-Normalisierung bedenken.
- Zeilenenden, Encodings (immer UTF-8 intern) und Locale-Formate explizit behandeln.
- Konfiguration an plattformüblichen Orten (XDG-Verzeichnisse unter Linux, `%APPDATA%`, `~/Library/Application Support`).
- Drucken über die native Druckschnittstelle (CUPS, Windows-Druck, macOS); Ausgabe zusätzlich als PDF in 1:1.

## Lizenz

- Lizenz des Projekts wird früh per ADR festgelegt. Jede Abhängigkeit muss lizenzverträglich sein; Prüfung automatisiert in CI.
