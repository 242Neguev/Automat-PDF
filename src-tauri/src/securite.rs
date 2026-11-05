//! Mot de passe : protéger un PDF (chiffrement AES-256, droits d'impression, de copie et de
//! modification), ou retirer le mot de passe d'un PDF dont on le connaît.

use std::collections::BTreeMap;
use std::sync::Arc;

use lopdf::encryption::crypt_filters::{Aes256CryptFilter, CryptFilter};
use lopdf::{Document, EncryptionState, EncryptionVersion, LoadOptions, Permissions};
use serde::Deserialize;

use crate::erreur::Erreur;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsProtection {
    /// Mot de passe pour ouvrir le document.
    pub mot_de_passe: String,
    /// Mot de passe du propriétaire (tous les droits) ; vide : tiré au hasard.
    #[serde(default)]
    pub proprietaire: String,
    pub impression: bool,
    pub copie: bool,
    pub modification: bool,
}

fn aleatoire(octets: &mut [u8]) -> Result<(), Erreur> {
    getrandom::fill(octets)
        .map_err(|erreur| Erreur::simple(format!("Le tirage aléatoire a échoué : {erreur}")))
}

/// Le document chiffré (AES-256), avec les droits choisis.
pub fn proteger(mut document: Document, options: &OptionsProtection) -> Result<Vec<u8>, Erreur> {
    if options.mot_de_passe.is_empty() {
        return Err(Erreur::simple("Choisissez un mot de passe."));
    }
    let proprietaire = if options.proprietaire.is_empty() {
        let mut octets = [0u8; 18];
        aleatoire(&mut octets)?;
        octets.iter().map(|octet| format!("{octet:02x}")).collect()
    } else {
        options.proprietaire.clone()
    };
    let mut cle = [0u8; 32];
    aleatoire(&mut cle)?;
    // L'accès des outils d'accessibilité reste toujours permis.
    let mut droits = Permissions::COPYABLE_FOR_ACCESSIBILITY;
    if options.impression {
        droits |= Permissions::PRINTABLE | Permissions::PRINTABLE_IN_HIGH_QUALITY;
    }
    if options.copie {
        droits |= Permissions::COPYABLE;
    }
    if options.modification {
        droits |= Permissions::MODIFIABLE
            | Permissions::ANNOTABLE
            | Permissions::FILLABLE
            | Permissions::ASSEMBLABLE;
    }
    let filtre: Arc<dyn CryptFilter> = Arc::new(Aes256CryptFilter);
    let version = EncryptionVersion::V5 {
        encrypt_metadata: true,
        crypt_filters: BTreeMap::from([(b"StdCF".to_vec(), filtre)]),
        file_encryption_key: &cle,
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password: &proprietaire,
        user_password: &options.mot_de_passe,
        permissions: droits,
    };
    let etat = EncryptionState::try_from(version).map_err(|erreur| {
        Erreur::simple(format!("Le chiffrement n'a pas pu être préparé : {erreur}"))
    })?;
    document.compress();
    document
        .encrypt(&etat)
        .map_err(|erreur| Erreur::simple(format!("Le PDF n'a pas pu être chiffré : {erreur}")))?;
    let mut octets = Vec::new();
    document
        .save_to(&mut octets)
        .map_err(|erreur| Erreur::simple(format!("Le PDF n'a pas pu être écrit : {erreur}")))?;
    Ok(octets)
}

/// Le PDF déchiffré avec son mot de passe, enregistré sans protection. Un PDF qui n'a que des
/// restrictions (impression, copie…) s'ouvre sans mot de passe : elles sont simplement retirées.
pub fn deverrouiller(octets: &[u8], nom: &str, mot_de_passe: &str) -> Result<Vec<u8>, Erreur> {
    let illisible =
        |erreur: lopdf::Error| Erreur::simple(format!("« {nom} » n'a pas pu être lu : {erreur}"));
    let mut document = match Document::load_mem_with_options(octets, LoadOptions::with_password(""))
    {
        Ok(document) if !document.was_encrypted() => {
            return Err(Erreur::simple(format!(
                "« {nom} » n'est pas protégé par un mot de passe."
            )));
        }
        Ok(document) => document,
        Err(lopdf::Error::InvalidPassword) => {
            Document::load_mem_with_options(octets, LoadOptions::with_password(mot_de_passe))
                .map_err(|erreur| match erreur {
                    lopdf::Error::InvalidPassword => Erreur::simple("Mot de passe incorrect."),
                    autre => illisible(autre),
                })?
        }
        Err(autre) => return Err(illisible(autre)),
    };
    let mut sortie = Vec::new();
    document
        .save_to(&mut sortie)
        .map_err(|erreur| Erreur::simple(format!("Le PDF n'a pas pu être écrit : {erreur}")))?;
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::pdf_herite;
    use crate::pdf::charger;

    fn options(mot_de_passe: &str) -> OptionsProtection {
        OptionsProtection {
            mot_de_passe: mot_de_passe.into(),
            proprietaire: String::new(),
            impression: true,
            copie: false,
            modification: false,
        }
    }

    #[test]
    fn protege_par_mot_de_passe_puis_le_retire() {
        let document = Document::load_mem(&pdf_herite(2, "Confidentiel")).unwrap();
        let protege = proteger(document, &options("Mars 2026")).unwrap();
        // Sans le mot de passe, le PDF ne s'ouvre pas ; avec, si.
        let erreur = charger(&protege, "bulletin.pdf").unwrap_err();
        assert!(
            erreur.message.contains("mot de passe"),
            "{}",
            erreur.message
        );
        assert!(
            Document::load_mem_with_options(&protege, LoadOptions::with_password("faux")).is_err()
        );
        let ouvert =
            Document::load_mem_with_options(&protege, LoadOptions::with_password("Mars 2026"))
                .unwrap();
        assert!(ouvert.was_encrypted());
        let page = ouvert.get_pages()[&1];
        assert!(
            String::from_utf8_lossy(&ouvert.get_page_content(page)).contains("(Confidentiel 1)")
        );
        // Mot de passe retiré : le PDF s'ouvre directement, et s'affiche.
        assert_eq!(
            deverrouiller(&protege, "b.pdf", "faux")
                .unwrap_err()
                .message,
            "Mot de passe incorrect."
        );
        let libre = deverrouiller(&protege, "b.pdf", "Mars 2026").unwrap();
        let document = charger(&libre, "b.pdf").unwrap();
        assert!(!document.trailer.has(b"Encrypt") && document.get_pages().len() == 2);
        let image =
            crate::rendu::image_page(std::sync::Arc::new(libre.clone()), "b.pdf", 1, 200, 90)
                .unwrap();
        let image = image::load_from_memory(&image).unwrap().to_luma8();
        assert!(image.pixels().filter(|p| p.0[0] < 100).count() > 20);
        assert!(
            deverrouiller(&libre, "b.pdf", "x")
                .unwrap_err()
                .message
                .contains("n'est pas protégé")
        );
    }

    #[test]
    fn refuse_un_mot_de_passe_vide() {
        let document = Document::load_mem(&pdf_herite(1, "T")).unwrap();
        assert_eq!(
            proteger(document, &options("")).unwrap_err().message,
            "Choisissez un mot de passe."
        );
    }
}
