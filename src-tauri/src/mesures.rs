//! Mesures du rendu par hayro pour le lecteur de PDF (étape 1 du plan du 25/11/2026) : temps
//! d'ouverture d'un gros PDF et de dessin d'une page à la taille d'un écran. Elles ne vérifient
//! rien et ne tournent pas avec les autres tests. À lancer à la main, en version optimisée :
//!
//!   cargo test --release --manifest-path src-tauri/Cargo.toml mesures -- --ignored --nocapture --test-threads 1
//!
//! Les PDF mesurés sont faits des fichiers d'essai de docs/test-windows/fichiers : des bulletins
//! de paie (texte, filets) et un courrier scanné (une grande image par page), répétés.

use std::sync::Arc;
use std::time::{Duration, Instant};

use hayro::hayro_interpret::InterpreterSettings;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{RenderCache, RenderSettings, render};

use crate::{pdf, rendu};

fn exemple(nom: &str) -> Vec<u8> {
    let chemin = format!(
        "{}/../docs/test-windows/fichiers/{nom}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&chemin).unwrap_or_else(|_| panic!("fichier d'essai introuvable : {chemin}"))
}

/// Le fichier d'essai `nom`, fusionné `fois` fois avec lui-même.
fn repete(nom: &str, fois: usize) -> Arc<Vec<u8>> {
    let octets = exemple(nom);
    let sources: Vec<(String, &[u8])> = (0..fois)
        .map(|i| (format!("{i} {nom}"), octets.as_slice()))
        .collect();
    Arc::new(pdf::fusionner(&sources).unwrap())
}

fn ms(duree: Duration) -> String {
    format!("{:.1} ms", duree.as_secs_f64() * 1000.0)
}

/// Médiane et 9e décile.
fn repartition(mut durees: Vec<Duration>) -> (Duration, Duration) {
    durees.sort();
    let rang = |q: f64| durees[((durees.len() - 1) as f64 * q).round() as usize];
    (rang(0.5), rang(0.9))
}

/// Une vingtaine de pages réparties dans tout le document.
fn echantillon(nombre: usize) -> Vec<usize> {
    (0..nombre).step_by((nombre / 20).max(1)).collect()
}

#[test]
#[ignore = "mesure, à lancer à la main en --release"]
fn mesure_le_rendu_d_un_gros_pdf() {
    let coeurs = std::thread::available_parallelism().map_or(1, |n| n.get());
    println!(
        "\n{coeurs} cœurs ; mode {}",
        if cfg!(debug_assertions) {
            "DÉBOGAGE (chiffres sans valeur)"
        } else {
            "optimisé"
        }
    );
    for (nom, fois) in [("Bulletins octobre.pdf", 33), ("Scan courrier.pdf", 50)] {
        let octets = repete(nom, fois);

        // Ouverture : lecture de la structure et taille de chaque page (ce que fait pages_pdf).
        let debut = Instant::now();
        let formats = rendu::formats(octets.clone(), nom).unwrap();
        let ouverture = debut.elapsed();
        let pages = echantillon(formats.len());
        println!(
            "\n{nom} × {fois} : {} pages, {:.1} Mo ; ouverture et tailles des pages : {}",
            formats.len(),
            octets.len() as f64 / 1e6,
            ms(ouverture)
        );

        // Ce que fait aujourd'hui vignette_pdf : document relu, page dessinée, JPEG (qualité 82).
        for largeur in [240u32, 800, 1200, 1600, 2400] {
            let mut durees = Vec::new();
            let mut octets_jpeg = 0;
            for &page in &pages {
                let debut = Instant::now();
                let jpeg = rendu::image_page(octets.clone(), nom, page, largeur, 82).unwrap();
                durees.push(debut.elapsed());
                octets_jpeg += jpeg.len();
            }
            let (mediane, decile) = repartition(durees);
            println!(
                "  {largeur:>4} px, JPEG : médiane {}, 9 pages sur 10 sous {} ; {} Ko par page",
                ms(mediane),
                ms(decile),
                octets_jpeg / pages.len() / 1000
            );
        }

        // Part du dessin seul (sans JPEG), et ce que rapporterait de garder le document ouvert et
        // le cache de hayro (polices, images) d'une page à l'autre, à 1 600 px de large.
        let echelle = 1600.0 / formats[0].largeur;
        let reglages = RenderSettings {
            x_scale: echelle,
            y_scale: echelle,
            bg_color: WHITE,
            ..Default::default()
        };
        let mut separes = Vec::new();
        for &page in &pages {
            let debut = Instant::now();
            let pdf = rendu::ouvrir(octets.clone(), nom).unwrap();
            let _ = render(
                &pdf.pages()[page],
                &RenderCache::new(),
                &InterpreterSettings::default(),
                &reglages,
            );
            separes.push(debut.elapsed());
        }
        let pdf = rendu::ouvrir(octets.clone(), nom).unwrap();
        let cache = RenderCache::new();
        let mut gardes = Vec::new();
        for &page in &pages {
            let debut = Instant::now();
            let _ = render(
                &pdf.pages()[page],
                &cache,
                &InterpreterSettings::default(),
                &reglages,
            );
            gardes.push(debut.elapsed());
        }
        let (m1, d1) = repartition(separes);
        let (m2, d2) = repartition(gardes);
        println!(
            "  1600 px, dessin seul, document relu à chaque page : médiane {}, 9 sur 10 sous {}",
            ms(m1),
            ms(d1)
        );
        println!(
            "  1600 px, dessin seul, document et cache gardés    : médiane {}, 9 sur 10 sous {}",
            ms(m2),
            ms(d2)
        );

        // Plusieurs pages à la fois, comme quand on fait défiler : 8 pages en parallèle.
        let lot: Vec<usize> = pages.iter().copied().take(8).collect();
        let debut = Instant::now();
        std::thread::scope(|portee| {
            for &page in &lot {
                let octets = octets.clone();
                portee.spawn(move || rendu::image_page(octets, nom, page, 1600, 82).unwrap());
            }
        });
        println!(
            "  8 pages à 1600 px en parallèle : {} au total",
            ms(debut.elapsed())
        );
    }
}
