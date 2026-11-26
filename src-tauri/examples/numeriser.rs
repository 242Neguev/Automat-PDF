//! Numériser depuis le terminal, sans l'interface : pour essayer un scanner, et pour le banc de
//! captures (le cœur du banc ne tourne pas sur le fil principal, où macOS répond).
//!
//! ```text
//! cargo run --example numeriser -- lister
//! cargo run --example numeriser -- numeriser '{"scanner":"…","source":"vitre","couleur":"gris","resolution":300}' dossier
//! cargo run --example numeriser -- numeriser '{…}' dossier 3   # arrêter après 3 pages
//! ```
//!
//! Le résultat s'écrit en JSON sur la sortie standard ; l'avancement, sur la sortie d'erreur.

use std::path::Path;
use std::sync::Arc;

use automat_pdf_lib::numerisation;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    // Le travail se fait sur un autre fil : sur Mac, le fil principal fait tourner sa boucle, où
    // arrivent les réponses des scanners.
    std::thread::spawn(move || {
        let resultat = match arguments.first().map(String::as_str) {
            Some("lister") => numerisation::scanners().map(|liste| serde_json::json!(liste)),
            Some("numeriser") if (3..=4).contains(&arguments.len()) => {
                let arret: Option<usize> = arguments.get(3).and_then(|n| n.parse().ok());
                match serde_json::from_str::<numerisation::Reglages>(&arguments[1]) {
                    Ok(reglages) => numerisation::numeriser(
                        &reglages,
                        Path::new(&arguments[2]),
                        Arc::new(move |pages| {
                            eprintln!("pages numérisées : {pages}");
                            if arret == Some(pages) {
                                numerisation::annuler();
                            }
                        }),
                    )
                    .map(|numerisation| serde_json::json!(numerisation)),
                    Err(erreur) => {
                        eprintln!("Réglages illisibles : {erreur}");
                        std::process::exit(2);
                    }
                }
            }
            _ => {
                eprintln!(
                    "Usage : numeriser lister | numeriser numeriser '<réglages JSON>' <dossier> [arrêt après N pages]"
                );
                std::process::exit(2);
            }
        };
        match resultat {
            Ok(json) => {
                println!("{json}");
                std::process::exit(0);
            }
            Err(erreur) => {
                println!("{}", serde_json::json!({ "erreur": erreur.message }));
                std::process::exit(1);
            }
        }
    });
    #[cfg(target_os = "macos")]
    objc2_foundation::NSRunLoop::mainRunLoop().run();
    #[cfg(not(target_os = "macos"))]
    loop {
        std::thread::park();
    }
}
