//! PDF construits pour les tests : pages A4 dont la taille et les ressources sont héritées de
//! l'arbre des pages, chacune avec une ligne de texte « <texte> <numéro> » en Helvetica.

use lopdf::{Document, Object, ObjectId, dictionary};

fn construire(n: usize, texte: &str, lien_vers_la_derniere: bool) -> Vec<u8> {
    let mut document = Document::with_version("1.5");
    let arbre = document.new_object_id();
    let police = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut pages: Vec<ObjectId> = Vec::new();
    for page in 0..n {
        let contenu = format!("BT /F1 24 Tf 72 720 Td ({texte} {}) Tj ET", page + 1);
        let flux = document.add_object(lopdf::Stream::new(dictionary! {}, contenu.into_bytes()));
        pages.push(
            document.add_object(
                dictionary! { "Type" => "Page", "Parent" => arbre, "Contents" => flux },
            ),
        );
    }
    if lien_vers_la_derniere && n > 1 {
        // Lien de la première page vers la dernière, comme une table des matières.
        let destination = vec![
            Object::Reference(pages[n - 1]),
            "XYZ".into(),
            0.into(),
            842.into(),
            0.into(),
        ];
        let lien = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Link",
            "Rect" => vec![72.into(), 700.into(), 300.into(), 740.into()],
            "Dest" => destination,
        });
        if let Ok(premiere) = document.get_dictionary_mut(pages[0]) {
            premiere.set("Annots", vec![Object::Reference(lien)]);
        }
    }
    let enfants: Vec<Object> = pages.into_iter().map(Object::Reference).collect();
    document.objects.insert(
        arbre,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => enfants, "Count" => n as i64,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => police } },
        }),
    );
    let catalogue = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => arbre });
    document.trailer.set("Root", catalogue);
    let mut octets = Vec::new();
    document.save_to(&mut octets).unwrap();
    octets
}

/// PDF de `n` pages.
pub fn pdf_herite(n: usize, texte: &str) -> Vec<u8> {
    construire(n, texte, false)
}

/// PDF de `n` pages dont la première a un lien interne vers la dernière.
pub fn pdf_avec_lien(n: usize, texte: &str) -> Vec<u8> {
    construire(n, texte, true)
}

/// Écrit un fichier de test dans un dossier temporaire propre au test ; renvoie son chemin.
pub fn fichier_temporaire(dossier: &str, nom: &str, octets: &[u8]) -> String {
    let dossier = std::env::temp_dir().join("automat-pdf-tests").join(dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    let chemin = dossier.join(nom);
    std::fs::write(&chemin, octets).unwrap();
    chemin.to_string_lossy().into_owned()
}
