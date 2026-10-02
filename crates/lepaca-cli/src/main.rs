//! Kommandozeile für lepaca. Bisher nur Versionsausgabe; Export folgt.

fn main() {
    println!("lepaca {}", env!("CARGO_PKG_VERSION"));
}
