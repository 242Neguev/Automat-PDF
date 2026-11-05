//! Ce qui est commun aux deux façons d'envoyer les fichiers par e-mail : par la messagerie de
//! l'utilisateur (`messagerie`, SMTP) ou par Outlook (`outlook`, Windows).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::erreur::{Echec, Erreur};

/// Un message à envoyer, ou à préparer dans Outlook.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Destinataire(s), séparés par des points-virgules.
    pub a: String,
    pub objet: String,
    pub texte: String,
    /// Chemins complets des pièces jointes.
    pub pieces: Vec<String>,
}

/// Ce que l'envoi a fait : messages réussis, et ceux qui ont échoué (rang, raison).
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanEnvoi {
    pub reussis: usize,
    pub echecs: Vec<Echec>,
}

/// Messages vérifiés avant l'envoi : une adresse, des pièces jointes qui existent.
pub fn verifier(messages: &[Message]) -> Result<(), Erreur> {
    let mut problemes = Vec::new();
    for (rang, message) in messages.iter().enumerate() {
        if !message.a.contains('@') {
            problemes.push(format!(
                "Message {} : adresse invalide « {} ».",
                rang + 1,
                message.a
            ));
        }
        for piece in &message.pieces {
            if !Path::new(piece).is_file() {
                problemes.push(format!(
                    "Message {} : « {piece} » est introuvable.",
                    rang + 1
                ));
            }
        }
    }
    if problemes.is_empty() {
        return Ok(());
    }
    Err(Erreur {
        message: "Certains messages ne peuvent pas être préparés.".into(),
        details: problemes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::fichier_temporaire;

    fn message(a: &str, pieces: Vec<String>) -> Message {
        Message {
            a: a.into(),
            objet: "Votre bulletin d'octobre".into(),
            texte: "Bonjour,\nci-joint votre bulletin.".into(),
            pieces,
        }
    }

    #[test]
    fn refuse_une_adresse_invalide_ou_une_piece_introuvable() {
        let piece = fichier_temporaire("envoi", "bulletin.pdf", b"%PDF-1.7");
        assert!(verifier(&[message("rh@exemple.com", vec![piece.clone()])]).is_ok());
        let erreur = verifier(&[
            message("sans arobase", vec![piece]),
            message("x@y.z", vec!["/nulle/part.pdf".into()]),
        ])
        .unwrap_err();
        assert_eq!(erreur.details.len(), 2, "{:?}", erreur.details);
        assert!(erreur.details[0].contains("Message 1"));
        assert!(erreur.details[1].contains("introuvable"));
    }
}
