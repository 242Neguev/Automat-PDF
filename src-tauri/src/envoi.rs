//! Ce qui est commun aux façons d'envoyer les fichiers par e-mail : par la messagerie de
//! l'utilisateur (`messagerie`, SMTP), par Outlook (`outlook`, Windows) ou par Mail (`mail`,
//! macOS).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::erreur::{Echec, Erreur};
use crate::pilotage::{Fin, NE_REPOND_PLUS};

/// Un message à envoyer, ou à préparer dans Outlook ou Mail.
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
        motif: None,
    })
}

/// Une ligne du compte rendu d'un script d'envoi (Outlook, Mail).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CompteRendu {
    Message {
        rang: usize,
        ok: bool,
        #[serde(default)]
        details: Option<String>,
    },
    Erreur {
        erreur: String,
        #[serde(default)]
        details: Option<String>,
    },
}

/// Compte rendu d'un script d'envoi (Outlook, Mail), suivi ligne par ligne.
pub struct Suivi<F: Fn(usize, usize)> {
    pub bilan: BilanEnvoi,
    pub erreur: Option<Erreur>,
    total: usize,
    progression: F,
    /// Message clair pour une erreur générale rapportée par le script (application fermée…).
    expliquer: fn(&str, Option<String>) -> Erreur,
}

impl<F: Fn(usize, usize)> Suivi<F> {
    /// `progression` reçoit le nombre de messages traités et le total.
    pub fn new(
        total: usize,
        progression: F,
        expliquer: fn(&str, Option<String>) -> Erreur,
    ) -> Self {
        Suivi {
            bilan: BilanEnvoi::default(),
            erreur: None,
            total,
            progression,
            expliquer,
        }
    }

    /// Note une ligne ; `false` quand une erreur générale (Outlook fermé…) arrête tout. Ce que le
    /// script écrit d'autre (avertissements) est ignoré.
    pub fn noter(&mut self, ligne: &str) -> bool {
        match serde_json::from_str::<CompteRendu>(ligne) {
            Ok(CompteRendu::Message { rang, ok, details }) => {
                if ok {
                    self.bilan.reussis += 1;
                } else {
                    self.bilan.echecs.push(Echec {
                        rang,
                        message: details.unwrap_or_default(),
                    });
                }
                (self.progression)(self.bilan.reussis + self.bilan.echecs.len(), self.total);
                true
            }
            Ok(CompteRendu::Erreur { erreur, details }) => {
                self.erreur = Some((self.expliquer)(&erreur, details));
                false
            }
            Err(_) => true,
        }
    }

    /// Bilan, une fois le script terminé : `echec` explique un script interrompu avant le premier
    /// message.
    pub fn conclure(self, fin: Fin, echec: &str) -> Result<BilanEnvoi, Erreur> {
        if fin.arrete {
            return Err(Erreur::simple(NE_REPOND_PLUS));
        }
        if let Some(erreur) = self.erreur {
            return Err(erreur);
        }
        if !fin.succes && self.bilan.reussis + self.bilan.echecs.len() == 0 {
            return Err(Erreur {
                message: echec.into(),
                details: fin.erreurs,
                motif: None,
            });
        }
        Ok(self.bilan)
    }
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
