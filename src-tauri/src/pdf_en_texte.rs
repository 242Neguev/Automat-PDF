//! PDF en texte : le texte de chaque PDF, page après page, dans un fichier texte (« nom.txt ») que
//! lisent tous les éditeurs (Bloc-notes, TextEdit). Les lignes sont celles de la page, les colonnes
//! d'un tableau séparées par des tabulations, les pages par une ligne vide. Le texte des pages
//! numérisées est reconnu par le moteur du système si on le demande (`ocr`), sans modifier le PDF.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::erreur::{Echec, Erreur};
use crate::office::{self, Conversions};
use crate::{fichiers, memoire, ocr, texte, volume};

/// En dessous de ce nombre de caractères (hors espaces), une page compte comme sans texte.
const TEXTE_MIN: usize = 3;

fn caracteres(texte: &str) -> usize {
    texte.chars().filter(|c| !c.is_whitespace()).count()
}

/// Le texte de chaque PDF dans « nom.txt », dans `dossier`, sans écraser un fichier existant.
/// `avancement(fichiers faits, fichiers, pages lues du fichier en cours, ses pages à lire)` : les
/// pages sont celles que lit le moteur de reconnaissance.
pub fn pdf_en_texte(
    fichiers: &[String],
    dossier: &str,
    reconnaitre: bool,
    avancement: impl Fn(usize, usize, usize, usize) + Sync,
) -> Result<Conversions, Erreur> {
    volume::autorise("PDF en texte", fichiers.len())?;
    std::fs::create_dir_all(dossier).map_err(|erreur| {
        Erreur::simple(format!("Impossible d'écrire dans « {dossier} » : {erreur}"))
    })?;
    let sources: Vec<PathBuf> = fichiers.iter().map(PathBuf::from).collect();
    let cibles = office::cibles(&sources, Path::new(dossier), "txt");
    let mut resultat = Conversions::default();
    for (rang, (source, cible)) in fichiers.iter().zip(cibles).enumerate() {
        avancement(rang, fichiers.len(), 0, 0);
        let nom = crate::operations::nom_de(source);
        let fait = memoire::pdf(source).and_then(|octets| {
            let texte = texte_du_pdf(octets, &nom, reconnaitre, &|lues, pages| {
                avancement(rang, fichiers.len(), lues, pages)
            })?;
            let cible = cible.to_string_lossy().into_owned();
            fichiers::enregistrer(&cible, &en_fichier(&texte))?;
            Ok(cible)
        });
        match fait {
            Ok(cible) => resultat.crees.push(cible),
            Err(erreur) => resultat.echecs.push(Echec {
                rang,
                message: erreur.message,
            }),
        }
    }
    avancement(fichiers.len(), fichiers.len(), 0, 0);
    Ok(resultat)
}

/// Le texte du PDF, ses pages séparées par une ligne vide. Celui des pages numérisées est reconnu
/// si `reconnaitre` ; `avancement(pages lues, pages à lire)`.
pub fn texte_du_pdf(
    octets: Arc<Vec<u8>>,
    nom: &str,
    reconnaitre: bool,
    avancement: &(dyn Fn(usize, usize) + Sync),
) -> Result<String, Erreur> {
    let mut pages = texte::textes(octets.clone(), nom)?;
    let sans_texte: Vec<usize> = pages
        .iter()
        .enumerate()
        .filter(|(_, texte)| caracteres(texte) < TEXTE_MIN)
        .map(|(page, _)| page)
        .collect();
    let tout_numerise = sans_texte.len() == pages.len();
    if !sans_texte.is_empty() && reconnaitre {
        let lues =
            ocr::textes_reconnus(&octets, nom, &sans_texte, avancement).map_err(|erreur| {
                Erreur::simple(if tout_numerise {
                    format!("Son texte n'a pas pu être reconnu : {}", erreur.message)
                } else {
                    format!(
                        "Le texte de ses pages numérisées n'a pas pu être reconnu : {} Sans « \
                     Reconnaître le texte des pages numérisées », seul le texte des autres pages \
                     est enregistré.",
                        erreur.message
                    )
                })
            })?;
        for (page, texte) in lues {
            if let Some(place) = pages.get_mut(page) {
                *place = texte;
            }
        }
    } else if tout_numerise && !pages.is_empty() && !reconnaitre {
        return Err(Erreur::simple(
            "Ce PDF est numérisé : ses pages n'ont pas de texte. Cochez « Reconnaître le texte des \
             pages numérisées » pour le lire.",
        ));
    }
    let texte = pages
        .iter()
        .map(|page| page.trim_end())
        .collect::<Vec<_>>()
        .join("\n\n");
    if caracteres(&texte) < TEXTE_MIN {
        return Err(Erreur::simple(
            "Aucun texte n'a été trouvé dans ce PDF : ses pages n'ont que des images ou des dessins.",
        ));
    }
    Ok(texte)
}

/// Le fichier texte, en UTF-8. Sous Windows, avec sa marque (BOM) et des fins de ligne CRLF, que
/// reconnaissent tous les logiciels de Windows.
fn en_fichier(texte: &str) -> Vec<u8> {
    let texte = format!("{}\n", texte.trim_end());
    if cfg!(windows) {
        let mut octets = vec![0xEF, 0xBB, 0xBF];
        octets.extend_from_slice(texte.replace('\n', "\r\n").as_bytes());
        octets
    } else {
        texte.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::{fichier_temporaire, pdf_herite, scan};

    #[test]
    fn ecrit_le_texte_des_pages_dans_un_fichier_texte() {
        let chemin = fichier_temporaire("pdf-en-texte", "Rapport.pdf", &pdf_herite(2, "Bonjour"));
        let dossier = Path::new(&chemin).parent().unwrap().join("textes");
        let _ = std::fs::remove_dir_all(&dossier);
        let dossier = dossier.to_string_lossy().into_owned();
        let source = vec![chemin.clone()];
        let resultat = pdf_en_texte(&source, &dossier, false, |_, _, _, _| {}).unwrap();
        assert_eq!(resultat.crees.len(), 1, "{:?}", resultat.echecs);
        assert!(resultat.crees[0].ends_with("Rapport.txt"));
        let ecrit = std::fs::read_to_string(&resultat.crees[0]).unwrap();
        assert_eq!(
            ecrit.trim_start_matches('\u{feff}').replace("\r\n", "\n"),
            "Bonjour 1\n\nBonjour 2\n"
        );
        // Le fichier qui existe n'est pas écrasé.
        let encore = pdf_en_texte(&source, &dossier, false, |_, _, _, _| {}).unwrap();
        assert!(
            encore.crees[0].ends_with("Rapport (2).txt"),
            "{:?}",
            encore.crees
        );
        let _ = std::fs::remove_dir_all(&dossier);
    }

    #[test]
    fn un_scan_demande_la_reconnaissance_du_texte() {
        let erreur =
            texte_du_pdf(Arc::new(scan("Attestation")), "scan.pdf", false, &|_, _| {}).unwrap_err();
        assert!(erreur.message.contains("numérisé"), "{}", erreur.message);
    }

    /// Vision, pour de vrai : le texte d'un scan est reconnu, sans toucher au PDF.
    #[cfg(target_os = "macos")]
    #[test]
    fn reconnait_le_texte_d_un_scan() {
        let octets = Arc::new(scan("Bulletin de paie"));
        let texte = texte_du_pdf(octets.clone(), "scan.pdf", true, &|_, _| {}).unwrap();
        assert!(texte.contains("Bulletin de paie"), "{texte:?}");
        assert!(ocr::semble_numerise(octets, "scan.pdf").unwrap());
    }
}
