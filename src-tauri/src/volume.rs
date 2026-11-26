//! Volume traité à la fois (facturation au volume, proposition du 26/11/2026, dans
//! `docs/edition-payante` d'Automat-PDF-main) : toutes les fonctions sont gratuites ; seul le
//! nombre de documents traités en une seule opération pourra demander une licence.
//!
//! Chaque opération qui traite un lot de documents passe par `autorise`. Tant qu'Eau sèche n'a pas
//! fixé le plafond gratuit et les paliers, il n'y a pas de plafond : rien n'est limité.

use crate::erreur::Erreur;

/// Documents qu'une opération peut traiter à la fois sans licence. `None` : pas de plafond (en
/// attente de la décision d'Eau sèche).
const PLAFOND_GRATUIT: Option<usize> = None;

/// L'opération `operation` (son nom, pour le message) peut-elle traiter `nombre` documents d'un coup ?
pub fn autorise(operation: &str, nombre: usize) -> Result<(), Erreur> {
    verifier(PLAFOND_GRATUIT, operation, nombre)
}

fn verifier(plafond: Option<usize>, operation: &str, nombre: usize) -> Result<(), Erreur> {
    match plafond {
        Some(plafond) if nombre > plafond => Err(Erreur::simple(format!(
            "{operation} : l'édition gratuite traite {plafond} documents à la fois. Retirez-en \
             {} de la liste, ou traitez-les en plusieurs fois.",
            nombre - plafond
        ))),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rien_n_est_limite_tant_que_le_plafond_n_est_pas_fixe() {
        assert!(autorise("Reconnaître le texte", 10_000).is_ok());
        assert!(verifier(Some(10), "Compresser", 10).is_ok());
        let erreur = verifier(Some(10), "Compresser", 12).unwrap_err();
        assert!(
            erreur.message.contains("10 documents à la fois"),
            "{}",
            erreur.message
        );
        assert!(
            erreur.message.contains("Retirez-en 2"),
            "{}",
            erreur.message
        );
    }
}
