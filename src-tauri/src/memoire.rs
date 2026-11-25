//! Contenu des derniers PDF ouverts, gardé en mémoire : les vignettes d'un même fichier ne le
//! relisent pas sur le disque à chaque page. Un fichier modifié depuis est relu.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use crate::erreur::Erreur;
use crate::fichiers::{self, Nature};

/// Nombre de fichiers gardés en mémoire, du plus récent au plus ancien.
const FICHIERS_GARDES: usize = 8;

struct Entree {
    chemin: String,
    modifie: Option<SystemTime>,
    taille: u64,
    octets: Arc<Vec<u8>>,
    /// Contenu déchiffré avec le mot de passe donné dans le lecteur, jamais écrit sur le disque.
    deverrouille: bool,
}

fn entrees() -> &'static Mutex<Vec<Entree>> {
    static ENTREES: OnceLock<Mutex<Vec<Entree>>> = OnceLock::new();
    ENTREES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Contenu d'un PDF choisi par l'utilisateur (contrôlé comme une lecture ordinaire).
pub fn pdf(chemin: &str) -> Result<Arc<Vec<u8>>, Erreur> {
    let infos = std::fs::metadata(chemin)
        .map_err(|erreur| Erreur::simple(format!("Impossible de lire « {chemin} » : {erreur}")))?;
    let (modifie, taille) = (infos.modified().ok(), infos.len());
    let mut entrees = entrees()
        .lock()
        .unwrap_or_else(|empoisonne| empoisonne.into_inner());
    if let Some(position) = entrees
        .iter()
        .position(|e| e.chemin == chemin && e.modifie == modifie && e.taille == taille)
    {
        let entree = entrees.remove(position);
        let octets = entree.octets.clone();
        entrees.insert(0, entree);
        return Ok(octets);
    }
    let octets = Arc::new(fichiers::lire(chemin, Nature::Pdf)?);
    entrees.retain(|e| e.chemin != chemin);
    entrees.insert(
        0,
        Entree {
            chemin: chemin.to_string(),
            modifie,
            taille,
            octets: octets.clone(),
            deverrouille: false,
        },
    );
    entrees.truncate(FICHIERS_GARDES);
    Ok(octets)
}

/// Déverrouille un PDF qui demande son mot de passe d'ouverture : son contenu déchiffré remplace,
/// en mémoire seulement, le contenu chiffré, et le lecteur le lit ensuite comme un PDF ordinaire.
/// Rien n'est écrit sur le disque. Un fichier modifié depuis, ou sorti de la mémoire
/// (`FICHIERS_GARDES`), est relu chiffré et redemande son mot de passe.
pub fn deverrouiller(chemin: &str, mot_de_passe: &str, nom: &str) -> Result<(), Erreur> {
    let lu = pdf(chemin)?;
    let deja = entrees()
        .lock()
        .unwrap_or_else(|empoisonne| empoisonne.into_inner())
        .iter()
        .any(|e| e.chemin == chemin && e.deverrouille && Arc::ptr_eq(&e.octets, &lu));
    if deja {
        return Ok(());
    }
    let clair = Arc::new(crate::securite::deverrouiller(&lu, nom, mot_de_passe)?);
    let mut entrees = entrees()
        .lock()
        .unwrap_or_else(|empoisonne| empoisonne.into_inner());
    if let Some(entree) = entrees
        .iter_mut()
        .find(|e| e.chemin == chemin && Arc::ptr_eq(&e.octets, &lu))
    {
        entree.octets = clair;
        entree.deverrouille = true;
    }
    Ok(())
}

/// Oublie le contenu déchiffré d'un PDF quand le lecteur passe à un autre document : comme à sa
/// première ouverture, il redemandera son mot de passe.
pub fn reverrouiller(chemin: &str) {
    entrees()
        .lock()
        .unwrap_or_else(|empoisonne| empoisonne.into_inner())
        .retain(|e| !(e.chemin == chemin && e.deverrouille));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erreur::MOT_DE_PASSE;
    use crate::essais::pdf_herite;
    use crate::securite::{OptionsProtection, proteger};

    #[test]
    fn deverrouille_en_memoire_un_pdf_protege_par_un_mot_de_passe() {
        let document = lopdf::Document::load_mem(&pdf_herite(2, "Confidentiel")).unwrap();
        let options = OptionsProtection {
            mot_de_passe: "Mars 2026".into(),
            proprietaire: String::new(),
            impression: true,
            copie: true,
            modification: true,
        };
        let protege = proteger(document, &options).unwrap();
        let dossier =
            std::env::temp_dir().join(format!("automat-pdf-memoire-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let fichier = dossier.join("Bulletin protégé.pdf");
        std::fs::write(&fichier, &protege).unwrap();
        let chemin = fichier.to_str().unwrap();
        let nom = "Bulletin protégé.pdf";

        // Sans son mot de passe, le lecteur ne l'ouvre pas, et l'interface sait pourquoi.
        let erreur = crate::rendu::ouvrir(pdf(chemin).unwrap(), nom)
            .err()
            .unwrap();
        assert_eq!(erreur.motif, Some(MOT_DE_PASSE));

        assert_eq!(
            deverrouiller(chemin, "faux", nom).unwrap_err().message,
            "Mot de passe incorrect."
        );
        deverrouiller(chemin, "Mars 2026", nom).unwrap();
        assert_eq!(
            crate::rendu::ouvrir(pdf(chemin).unwrap(), nom)
                .unwrap()
                .pages()
                .len(),
            2
        );
        deverrouiller(chemin, "Mars 2026", nom).unwrap();
        // Un autre document ouvert : il redemande son mot de passe.
        reverrouiller(chemin);
        assert_eq!(
            crate::rendu::ouvrir(pdf(chemin).unwrap(), nom)
                .err()
                .unwrap()
                .motif,
            Some(MOT_DE_PASSE)
        );
        // Le fichier, lui, reste protégé.
        assert_eq!(std::fs::read(chemin).unwrap(), protege);
        std::fs::remove_dir_all(&dossier).ok();
    }
}
