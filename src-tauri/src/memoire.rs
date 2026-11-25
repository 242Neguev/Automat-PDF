//! Contenu des derniers PDF ouverts, gardé en mémoire : les vignettes d'un même fichier ne le
//! relisent pas sur le disque à chaque page. Un fichier modifié depuis est relu.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use crate::erreur::Erreur;
use crate::fichiers::{self, Nature};

/// Nombre de fichiers gardés en mémoire, du plus récent au plus ancien. Un PDF déverrouillé (ouvert
/// avec son mot de passe, dans un onglet du lecteur) passe après les autres : il n'est oublié
/// qu'au-delà du double, pour ne pas redemander son mot de passe en revenant à son onglet.
const FICHIERS_GARDES: usize = 8;

struct Entree {
    chemin: String,
    modifie: Option<SystemTime>,
    taille: u64,
    octets: Arc<Vec<u8>>,
    /// Contenu déchiffré avec le mot de passe donné dans le lecteur, jamais écrit sur le disque.
    deverrouille: bool,
    /// Contenu d'avant les retouches du lecteur (commentaires du PDF supprimés ou changés) : les
    /// retouches se refont toujours depuis lui, et il revient quand elles sont toutes défaites.
    origine: Option<Arc<Vec<u8>>>,
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
            origine: None,
        },
    );
    limiter(&mut entrees);
    Ok(octets)
}

/// Oublie les fichiers en trop : les plus anciens PDF ordinaires d'abord (voir `FICHIERS_GARDES`) ;
/// un PDF retouché dans le lecteur passe, comme un PDF déverrouillé, après les autres.
fn limiter(entrees: &mut Vec<Entree>) {
    while entrees.len() > FICHIERS_GARDES {
        match entrees
            .iter()
            .rposition(|e| !e.deverrouille && e.origine.is_none())
        {
            Some(position) => drop(entrees.remove(position)),
            None if entrees.len() > 2 * FICHIERS_GARDES => drop(entrees.pop()),
            None => break,
        }
    }
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

/// Oublie ce que le lecteur a fait d'un PDF quand son onglet se ferme : son contenu déchiffré
/// (comme à sa première ouverture, il redemandera son mot de passe) et ses retouches.
pub fn reverrouiller(chemin: &str) {
    let mut entrees = entrees()
        .lock()
        .unwrap_or_else(|empoisonne| empoisonne.into_inner());
    entrees.retain(|e| !(e.chemin == chemin && e.deverrouille));
    for entree in entrees.iter_mut().filter(|e| e.chemin == chemin) {
        if let Some(origine) = entree.origine.take() {
            entree.octets = origine;
        }
    }
}

/// Le contenu d'un PDF tel qu'il était avant les retouches du lecteur (sinon, tel quel).
pub fn origine(chemin: &str) -> Result<Arc<Vec<u8>>, Erreur> {
    let actuel = pdf(chemin)?;
    let entrees = entrees()
        .lock()
        .unwrap_or_else(|empoisonne| empoisonne.into_inner());
    Ok(entrees
        .iter()
        .find(|e| e.chemin == chemin && Arc::ptr_eq(&e.octets, &actuel))
        .and_then(|e| e.origine.clone())
        .unwrap_or(actuel))
}

/// Retouche, en mémoire seulement, le contenu d'un PDF : `retoucher` reçoit son contenu
/// d'origine et rend le contenu retouché, ou rien quand il n'y a plus de retouche (l'origine
/// revient). Rien n'est écrit sur le disque.
pub fn retoucher(
    chemin: &str,
    retoucher: impl FnOnce(&[u8]) -> Result<Option<Vec<u8>>, Erreur>,
) -> Result<(), Erreur> {
    let origine = origine(chemin)?;
    let retouche = retoucher(&origine)?;
    let mut entrees = entrees()
        .lock()
        .unwrap_or_else(|empoisonne| empoisonne.into_inner());
    if let Some(entree) = entrees.iter_mut().find(|e| e.chemin == chemin) {
        match retouche {
            Some(octets) => {
                entree.origine.get_or_insert(origine);
                entree.octets = Arc::new(octets);
            }
            None => {
                if let Some(origine) = entree.origine.take() {
                    entree.octets = origine;
                }
            }
        }
    }
    Ok(())
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
        // Son onglet fermé : il redemande son mot de passe.
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

    #[test]
    fn retouche_en_memoire_puis_revient_a_l_origine() {
        let chemin =
            crate::essais::fichier_temporaire("memoire-retouches", "Reçu.pdf", b"%PDF origine");
        assert_eq!(pdf(&chemin).unwrap().as_slice(), b"%PDF origine");
        retoucher(&chemin, |origine| {
            assert_eq!(origine, b"%PDF origine");
            Ok(Some(b"%PDF retouche 1".to_vec()))
        })
        .unwrap();
        assert_eq!(pdf(&chemin).unwrap().as_slice(), b"%PDF retouche 1");
        // Une nouvelle retouche repart de l'origine, pas de la précédente.
        retoucher(&chemin, |origine| {
            assert_eq!(origine, b"%PDF origine");
            Ok(Some(b"%PDF retouche 2".to_vec()))
        })
        .unwrap();
        assert_eq!(origine(&chemin).unwrap().as_slice(), b"%PDF origine");
        // Plus de retouche : l'origine revient ; l'onglet fermé l'efface aussi.
        retoucher(&chemin, |_| Ok(None)).unwrap();
        assert_eq!(pdf(&chemin).unwrap().as_slice(), b"%PDF origine");
        retoucher(&chemin, |_| Ok(Some(b"%PDF retouche 3".to_vec()))).unwrap();
        reverrouiller(&chemin);
        assert_eq!(pdf(&chemin).unwrap().as_slice(), b"%PDF origine");
        // Le fichier, lui, n'a jamais changé.
        assert_eq!(std::fs::read(&chemin).unwrap(), b"%PDF origine");
    }

    #[test]
    fn garde_les_pdf_deverrouilles_plus_longtemps_que_les_autres() {
        let entree = |chemin: String, deverrouille: bool| Entree {
            chemin,
            modifie: None,
            taille: 0,
            octets: Arc::new(Vec::new()),
            deverrouille,
            origine: None,
        };
        // Le PDF déverrouillé est le plus ancien : les PDF ordinaires les plus anciens partent avant lui.
        let mut entrees: Vec<Entree> = (0..9)
            .map(|i| entree(format!("ordinaire {i}"), false))
            .collect();
        entrees.push(entree("protégé".into(), true));
        limiter(&mut entrees);
        let gardes: Vec<&str> = entrees.iter().map(|e| e.chemin.as_str()).collect();
        assert_eq!(gardes.len(), FICHIERS_GARDES);
        assert_eq!(gardes[0], "ordinaire 0");
        assert_eq!(gardes.last(), Some(&"protégé"));
        // Seulement des PDF déverrouillés : gardés jusqu'au double, les plus récents.
        let mut entrees: Vec<Entree> = (0..20)
            .map(|i| entree(format!("protégé {i}"), true))
            .collect();
        limiter(&mut entrees);
        assert_eq!(entrees.len(), 2 * FICHIERS_GARDES);
        assert_eq!(entrees.last().unwrap().chemin, "protégé 15");
    }
}
