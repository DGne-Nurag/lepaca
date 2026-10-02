# ADR 0002: Oberfläche mit Tauri 2 und TypeScript (vorläufig)

- Status: vorläufig angenommen, bestätigt oder verworfen durch den Stresstest unten
- Datum: 2026-10-02
- Beteiligte: Nurag (Entscheidung, Review), Claude (Umsetzung)
- Bezug: ADR 0001 (Rust für Kern und Parser)

## Kontext

Die Oberfläche soll unter Linux (X11 und Wayland), Windows und macOS gleichwertig laufen, barrierefrei sein und Muster maßgenau drucken. Nurag soll die Oberfläche selbst reviewen können (TS/JS bekannt).

## Entscheidung

Die Oberfläche wird mit **Tauri 2** und **TypeScript** gebaut. Der Rust-Kern läuft im Tauri-Backend.

- Zeichenfläche als **Canvas/WebGL**, nicht als SVG-DOM mit einem Element pro Objekt.
- Der Kern berechnet alle Geometrie (Stichlöcher, Offsets, Kachelung); die Oberfläche zeichnet nur.
- Detailstufen beim Zoomen (z. B. Lochreihe als Linie bei kleinem Zoom).
- **Drucken:** Der Kern erzeugt ein 1:1-PDF, das über den nativen Systemdruck (CUPS, Windows, macOS) ausgegeben wird. Die Maßtreue hängt damit nicht an der Web-Engine.

## Stresstest (Bedingung für endgültige Annahme)

Erster GUI-Prototyp, bevor Funktionen auf der Oberfläche gebaut werden:
- ca. 50 Teile mit insgesamt 20.000 Stichlöchern,
- flüssiges Zoomen und Verschieben,
- Linux mit Wayland auf Intel/AMD und auf Nvidia, dazu Windows und macOS,
- Screenreader (Orca, NVDA, VoiceOver) erreicht Werkzeuge und Dialoge,
- gedrucktes Testmuster (100-mm-Quadrat) misst 100 mm.

Fällt der Test durch, wird ADR 0002 durch ein neues ADR für eine native GPU-Oberfläche (Slint oder egui) abgelöst. Der Kern bleibt unverändert.

## Betrachtete Alternativen

| Option | Pro | Contra |
|---|---|---|
| **Tauri 2 + TS** | Nurag kann UI reviewen, Barrierefreiheit über ARIA ausgereift, kleine Bundles | WebKitGTK unter Linux langsamer, bekannte Wayland/Nvidia-Probleme |
| Wails + TS | wie Tauri | Go-Backend, passt nicht zu ADR 0001; gleiche WebKitGTK-Schwäche |
| Slint | reines Rust, GPU-Rendering | Drucken fehlt, Barrierefreiheit unvollständig |
| egui | reines Rust, schnell | Immediate-Mode, kein Drucken, Barrierefreiheit über AccessKit begrenzt |
| Qt 6 (cxx-qt) | reifstes Toolkit | zweite Sprache/C++-Brücke, Lizenzfragen |
| Electron | Chromium überall gleich | groß, mehr Angriffsfläche, Playbook 03 skeptisch |

## Folgen

- Linux-Pakete (Flatpak/AppImage) bringen WebKitGTK mit; Windows nutzt WebView2, macOS WKWebView.
- Kein abhängigkeitsfreies Einzelbinary für die GUI (Kern-CLI bleibt statisch).
- Für bekannte WebKitGTK-Treiberprobleme werden Workarounds dokumentiert und getestet.

## Security-Auswirkungen

- Strikte Content-Security-Policy, keine Remote-Inhalte in der WebView, kein Netzwerk ohne Opt-in (Kernregel 3).
- Tauri-Capabilities minimal: die Oberfläche bekommt nur explizit freigegebene Kern-Befehle, keinen direkten Dateisystem- oder Shell-Zugriff.
- Dateien werden ausschließlich im Rust-Kern geparst, nie in der WebView.

## Usability-Auswirkungen

- Barrierefreiheit über ARIA und native WebView-Accessibility (AT-SPI, UIA, NSAccessibility), im Stresstest zu prüfen.
- HiDPI und Skalierung über die WebView.
- Maßtreue beim Druck über das vom Kern erzeugte PDF garantiert.
