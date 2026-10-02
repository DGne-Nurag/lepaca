fn main() {
    // Eigene Befehle werden ausdrücklich registriert; die Oberfläche darf sie
    // nur nutzen, wenn eine Capability sie freigibt.
    let attribute = tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["teil_geometrie", "pdf_exportieren"]),
    );
    if let Err(e) = tauri_build::try_build(attribute) {
        panic!("tauri-build fehlgeschlagen: {e:#}");
    }
}
