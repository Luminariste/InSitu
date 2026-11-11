//! `insitu` : le moteur en ligne de commande.
//!
//! Chaque capacité du moteur doit rester appelable d'ici, pour que les agents
//! puissent la vérifier sans interface. Sous-commandes disponibles :
//!
//! - `insitu info` : carte d'identité du moteur, en JSON.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("info") => {
            let info = insitu_core::engine_info();
            println!(
                "{}",
                serde_json::to_string_pretty(&info).expect("EngineInfo est sérialisable")
            );
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") | None => {
            println!("{}", usage());
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("Commande inconnue : {other}\n\n{}", usage());
            ExitCode::from(2)
        }
    }
}

fn usage() -> &'static str {
    "Usage : insitu <commande>\n\nCommandes :\n  info    Affiche la carte d'identité du moteur (JSON)"
}
