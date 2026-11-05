//! Fichiers choisis par l'utilisateur : lecture contrôlée (PDF, liste) et écriture d'un dossier de
//! résultats (PDF séparés…), sans jamais sortir de ce dossier.

use std::path::Path;

use serde::Deserialize;

use crate::erreur::Erreur;

/// Ce que l'on s'attend à lire : chaque nature a ses formats et sa taille maximale.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Nature {
    Liste,
    Pdf,
    Image,
}

impl Nature {
    fn extensions(self) -> &'static [&'static str] {
        match self {
            Nature::Liste => &["xlsx", "xlsm", "xls", "ods", "csv", "tsv", "txt"],
            Nature::Pdf => &["pdf"],
            Nature::Image => &["jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "gif"],
        }
    }

    fn taille_max(self) -> u64 {
        match self {
            Nature::Liste => 50 * 1024 * 1024,
            Nature::Pdf => 500 * 1024 * 1024,
            Nature::Image => 100 * 1024 * 1024,
        }
    }

    fn refus(self, extension: &str) -> String {
        match self {
            Nature::Liste => format!(
                "Format de liste non pris en charge (.{extension}) : utilisez un fichier Excel, CSV ou ODS."
            ),
            Nature::Pdf => format!("Format non pris en charge (.{extension}) : choisissez un PDF."),
            Nature::Image => format!(
                "Format d'image non pris en charge (.{extension}) : utilisez un JPG, un PNG, un WEBP, un TIFF, un BMP ou un GIF."
            ),
        }
    }
}

/// Contenu d'un fichier, après contrôle de son format et de sa taille.
pub fn lire(chemin: &str, nature: Nature) -> Result<Vec<u8>, Erreur> {
    let extension = Path::new(chemin)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();
    if !nature.extensions().contains(&extension.as_str()) {
        return Err(Erreur::simple(nature.refus(&extension)));
    }
    let illisible = |erreur: std::io::Error| {
        Erreur::simple(format!("Impossible de lire « {chemin} » : {erreur}"))
    };
    let taille = std::fs::metadata(chemin).map_err(illisible)?.len();
    if taille > nature.taille_max() {
        return Err(Erreur::simple(format!(
            "Le fichier dépasse {} Mo.",
            nature.taille_max() / (1024 * 1024)
        )));
    }
    std::fs::read(chemin).map_err(illisible)
}

/// Un fichier à créer : un texte ou des octets (PDF…).
#[derive(Debug, Deserialize)]
pub struct FichierAEcrire {
    pub nom: String,
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub octets: Option<Vec<u8>>,
}

/// Nom de fichier sans risque : ni séparateur, ni caractère interdit par Windows, ni « .. ».
pub fn nom_sur(nom: &str) -> Option<String> {
    let propre: String = nom
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|".contains(c) || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    // Les points finaux sont retirés (Windows les ignore) : « . » et « .. » deviennent vides.
    let propre = propre.trim().trim_end_matches('.').to_string();
    (!propre.is_empty()).then_some(propre)
}

/// Chemin relatif sans risque (« bulletins/001.pdf ») : chaque élément est un nom sûr ; les « . »
/// et « .. » sont écartés, on ne sort jamais du dossier.
fn chemin_sur(nom: &str) -> Option<std::path::PathBuf> {
    let elements: Vec<String> = nom.split(['/', '\\']).filter_map(nom_sur).collect();
    (!elements.is_empty()).then(|| elements.iter().collect())
}

/// Chemin qu'aura le fichier `nom` une fois écrit dans `dossier` par [`ecrire`].
pub fn chemin_dans(dossier: &str, nom: &str) -> Option<String> {
    Some(
        Path::new(dossier)
            .join(chemin_sur(nom)?)
            .to_string_lossy()
            .into_owned(),
    )
}

/// Crée (ou complète) le dossier `sous_dossier` dans `dossier` et y écrit les fichiers ; renvoie son chemin.
/// Un nom de fichier peut contenir des sous-dossiers (« bulletins/001.pdf »).
pub fn ecrire(
    dossier: &str,
    sous_dossier: &str,
    fichiers: &[FichierAEcrire],
) -> Result<String, Erreur> {
    let nom_dossier =
        nom_sur(sous_dossier).ok_or_else(|| Erreur::simple("Nom de dossier invalide."))?;
    let cible = Path::new(dossier).join(nom_dossier);
    let echec = |erreur: std::io::Error| {
        Erreur::simple(format!(
            "Impossible d'écrire dans « {} » : {erreur}",
            cible.display()
        ))
    };
    std::fs::create_dir_all(&cible).map_err(echec)?;
    for fichier in fichiers {
        let nom = chemin_sur(&fichier.nom)
            .ok_or_else(|| Erreur::simple(format!("Nom de fichier invalide : {}", fichier.nom)))?;
        if let Some(parent) = cible.join(&nom).parent() {
            std::fs::create_dir_all(parent).map_err(echec)?;
        }
        let contenu = match (&fichier.texte, &fichier.octets) {
            (Some(texte), _) => texte.as_bytes(),
            (None, Some(octets)) => octets.as_slice(),
            (None, None) => &[],
        };
        std::fs::write(cible.join(nom), contenu).map_err(echec)?;
    }
    Ok(cible.to_string_lossy().into_owned())
}

/// Enregistre un fichier choisi par l'utilisateur (boîte « Enregistrer sous »).
pub fn enregistrer(chemin: &str, octets: &[u8]) -> Result<(), Erreur> {
    std::fs::write(chemin, octets).map_err(|erreur| {
        Erreur::simple(format!("Impossible d'enregistrer « {chemin} » : {erreur}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier_de_test(nom: &str) -> std::path::PathBuf {
        let dossier = std::env::temp_dir().join("automat-pdf-tests").join(nom);
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(&dossier).unwrap();
        dossier
    }

    #[test]
    fn controle_le_format_et_la_taille() {
        let dossier = dossier_de_test("lecture");
        let pdf = dossier.join("courrier.pdf");
        std::fs::write(&pdf, b"%PDF-1.7").unwrap();
        assert_eq!(
            lire(pdf.to_str().unwrap(), Nature::Pdf).unwrap(),
            b"%PDF-1.7"
        );
        let erreur = lire(pdf.to_str().unwrap(), Nature::Liste).unwrap_err();
        assert!(erreur.message.contains("Excel"));
    }

    #[test]
    fn ecrit_sans_sortir_du_dossier() {
        let dossier = dossier_de_test("ecriture");
        let fichiers = [
            FichierAEcrire {
                nom: "Aïssatou DIALLO.pdf".into(),
                texte: None,
                octets: Some(vec![1, 2, 3]),
            },
            FichierAEcrire {
                nom: "../../évasion.pdf".into(),
                texte: Some("non".into()),
                octets: None,
            },
            FichierAEcrire {
                nom: "bulletins/001.pdf".into(),
                texte: Some("ok".into()),
                octets: None,
            },
        ];
        let cible = ecrire(dossier.to_str().unwrap(), "Paie : mars", &fichiers).unwrap();
        assert!(cible.ends_with("Paie - mars"));
        let cible = Path::new(&cible);
        assert_eq!(
            std::fs::read(cible.join("Aïssatou DIALLO.pdf")).unwrap(),
            [1, 2, 3]
        );
        // « ../../évasion.pdf » reste dans le dossier : les « .. » sont écartés.
        assert!(cible.join("évasion.pdf").exists());
        assert!(!dossier.join("évasion.pdf").exists());
        assert!(cible.join("bulletins").join("001.pdf").exists());
    }
}
