# Playbook 04 – Dateiformate und Daten

Dateien sind die größte Angriffsfläche von lepaca und zugleich das Wertvollste für Nutzer:innen. Beides verlangt Sorgfalt.

## Eigenes Dateiformat

- **Offen und dokumentiert.** Spezifikation liegt im Repository; das Format ist ohne lepaca lesbar.
- **Versioniert.** Jede Datei trägt eine Formatversion. Neuere lepaca-Versionen lesen alle älteren Formate; ältere Versionen erkennen neuere Dateien und melden das verständlich, statt Daten still zu verwerfen.
- **Einfach und nicht ausführbar.** Bevorzugt ein textbasiertes, strikt schematisiertes Format (z. B. JSON oder ein ZIP-Container mit JSON und Bildern). Keine eingebetteten Skripte, keine Verweise auf Dateien außerhalb des Containers.
- **Metadaten bewusst.** Autor, Pfade und Rechnernamen werden nur gespeichert, wenn nötig und sichtbar. Beim Weitergeben („Für Weitergabe exportieren") werden persönliche Metadaten entfernt.
- **Atomar speichern.** In temporäre Datei schreiben, `fsync`, dann umbenennen. Vorherige Version optional als Backup behalten.

## Import und Export

Ziel-Formate (Priorität per Issue klären): SVG, DXF, PDF, Rasterbilder als Vorlage (PNG/JPEG), ggf. Formate anderer Leder-CAD-Programme.

### Parser-Härtung (Pflicht für jeden Parser)

1. **Grenzwerte** für Dateigröße, Anzahl Objekte, Punkte pro Pfad, Verschachtelungstiefe, Textlängen, Bildauflösung und dekomprimierte Größe (Schutz vor ZIP-/XML-Bomben). Grenzwerte zentral definiert und in den Einstellungen sichtbar.
2. **Keine externen Referenzen** auflösen: keine XML-Entities (XXE), keine externen Bilder/Fonts/URLs in SVG, keine Netzwerkzugriffe beim Import.
3. **Strikte Validierung** gegen ein internes Modell: Zahlen endlich (kein NaN/Inf), Koordinaten in plausiblen Bereichen, Indizes geprüft.
4. **Unbekanntes ignorieren und melden**, nicht raten. Nutzer:innen sehen nach dem Import eine Zusammenfassung („3 Elemente übersprungen: Text in unbekannter Schrift").
5. **Fuzzing**: Jeder Parser hat ein Fuzz-Target, das in CI regelmäßig läuft. Gefundene Abstürze werden als Regressionstest übernommen.
6. **Isolierbar**: Parser-Schnittstelle so gestalten, dass der Parser in einem eigenen, rechtearmen Prozess laufen kann.
7. Bevorzugt geprüfte, gepflegte Bibliotheken statt Eigenbau; wenn Bibliothek, dann deren Sicherheitshistorie prüfen (Playbook 06).

### Export

- Export erzeugt exakte Maße; jede Exportdatei wird in Tests zurückgelesen und auf Maßtreue geprüft.
- Für Laser/Plotter werden nur Geometrie und dokumentierte Parameter exportiert, keine aus Importdateien durchgereichten Rohbefehle.

## Nutzerdaten schützen

- Autosave in kurzen Abständen in ein Wiederherstellungsverzeichnis; nach Absturz wird Wiederherstellung angeboten.
- Keine Datei wird ohne Nachfrage überschrieben, die nicht von lepaca stammt oder ein neueres Format hat.
- Beim Öffnen eines älteren Formats wird beim Speichern darauf hingewiesen, dass ältere lepaca-Versionen die Datei danach ggf. nicht mehr lesen.

## Testdaten

- Sammlung realer und präparierter Testdateien unter `tests/fixtures/` (nur mit geklärten Rechten).
- Eigener Ordner `tests/fixtures/malicious/` mit bösartigen Beispielen (Bomben, Endlosrekursion, riesige Zahlen, Path-Traversal).
