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
        },
    );
    entrees.truncate(FICHIERS_GARDES);
    Ok(octets)
}
