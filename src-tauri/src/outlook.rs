//! Envoi de fichiers par Outlook (Windows) : chaque message (destinataire, objet, texte, pièces
//! jointes) est préparé dans les Brouillons, ou envoyé tout de suite. Outlook « classique » est
//! piloté par le script `scripts/outlook.ps1` (automatisation COM).

use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::envoi::{BilanEnvoi, Message, verifier};
use crate::erreur::{Echec, Erreur};
use crate::powershell;

const SCRIPT: &str = include_str!("../scripts/outlook.ps1");
/// Sans nouvelles d'Outlook pendant ce temps, l'envoi est arrêté ; Outlook peut demander l'accord
/// de l'utilisateur par une fenêtre de sécurité : on lui en laisse le temps.
const SILENCE_MAX: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModeEnvoi {
    /// Dans les Brouillons d'Outlook, à vérifier puis envoyer.
    Brouillons,
    /// Envoyés aussitôt.
    Direct,
}

/// Une ligne du compte rendu du script.
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

/// Contenu du fichier lu par le script : les messages, avec des fins de ligne Windows.
pub fn preparer(messages: &[Message], mode: ModeEnvoi) -> String {
    let messages: Vec<Message> = messages
        .iter()
        .map(|message| Message {
            texte: message.texte.replace("\r\n", "\n").replace('\n', "\r\n"),
            ..message.clone()
        })
        .collect();
    json!({ "envoyer": mode == ModeEnvoi::Direct, "messages": messages }).to_string()
}

/// Message clair pour une erreur générale rapportée par le script.
fn erreur_du_script(code: &str, details: Option<String>) -> Erreur {
    let message = match code {
        "outlook-ferme" => {
            "Outlook n'est pas ouvert : ouvrez Outlook (la version classique, pas le « nouvel Outlook »), puis recommencez."
        }
        "outlook-absent" => {
            "Outlook (version classique) est introuvable ou ne se laisse pas piloter. Le « nouvel Outlook » ne le permet pas : revenez à la version classique."
        }
        _ => "Les messages n'ont pas pu être transmis à Outlook.",
    };
    Erreur {
        message: message.into(),
        details: details.into_iter().collect(),
    }
}

/// Compte rendu du script, suivi ligne par ligne.
struct Suivi<F: Fn(usize, usize)> {
    bilan: BilanEnvoi,
    erreur: Option<Erreur>,
    total: usize,
    progression: F,
}

impl<F: Fn(usize, usize)> Suivi<F> {
    /// Note une ligne ; `false` quand une erreur générale (Outlook fermé…) arrête tout. Ce que
    /// PowerShell écrit d'autre (avertissements) est ignoré.
    fn noter(&mut self, ligne: &str) -> bool {
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
                self.erreur = Some(erreur_du_script(&erreur, details));
                false
            }
            Err(_) => true,
        }
    }
}

/// Prépare (ou envoie) les messages dans Outlook ; `progression` reçoit le nombre de messages
/// traités et le total.
pub fn envoyer(
    messages: &[Message],
    mode: ModeEnvoi,
    progression: impl Fn(usize, usize),
) -> Result<BilanEnvoi, Erreur> {
    verifier(messages)?;
    let mut suivi = Suivi {
        bilan: BilanEnvoi::default(),
        erreur: None,
        total: messages.len(),
        progression,
    };
    let fin = powershell::lancer(SCRIPT, &preparer(messages, mode), SILENCE_MAX, |ligne| {
        suivi.noter(ligne)
    })?;
    if fin.arrete {
        return Err(Erreur::simple(powershell::NE_REPOND_PLUS));
    }
    if let Some(erreur) = suivi.erreur {
        return Err(erreur);
    }
    if !fin.succes && suivi.bilan.reussis + suivi.bilan.echecs.len() == 0 {
        return Err(Erreur {
            message: "Les messages n'ont pas pu être transmis à Outlook.".into(),
            details: fin.erreurs,
        });
    }
    Ok(suivi.bilan)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    fn message(a: &str, pieces: Vec<String>) -> Message {
        Message {
            a: a.into(),
            objet: "Votre bulletin d'octobre".into(),
            texte: "Bonjour Aïssatou,\nci-joint votre bulletin.\n\nLe service RH".into(),
            pieces,
        }
    }

    #[test]
    fn prepare_les_messages_pour_le_script() {
        let donnees = preparer(
            &[message("a.diallo@exemple.com", vec![])],
            ModeEnvoi::Brouillons,
        );
        let lu: serde_json::Value = serde_json::from_str(&donnees).unwrap();
        assert_eq!(lu["envoyer"], false);
        assert_eq!(lu["messages"][0]["a"], "a.diallo@exemple.com");
        // Fins de ligne Windows, accents intacts.
        assert_eq!(
            lu["messages"][0]["texte"],
            "Bonjour Aïssatou,\r\nci-joint votre bulletin.\r\n\r\nLe service RH"
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&preparer(&[], ModeEnvoi::Direct)).unwrap()["envoyer"],
            true
        );
    }

    #[test]
    fn suit_le_compte_rendu_du_script() {
        let etapes = RefCell::new(Vec::new());
        let mut suivi = Suivi {
            bilan: BilanEnvoi::default(),
            erreur: None,
            total: 3,
            progression: |fait, total| etapes.borrow_mut().push((fait, total)),
        };
        for ligne in [
            "AVERTISSEMENT : sans importance",
            r#"{"ok":true,"rang":0}"#,
            r#"{"rang":1,"details":"Adresse refusée","ok":false}"#,
            r#"{"ok":true,"rang":2}"#,
        ] {
            assert!(suivi.noter(ligne));
        }
        assert_eq!(suivi.bilan.reussis, 2);
        assert_eq!(
            suivi.bilan.echecs,
            [Echec {
                rang: 1,
                message: "Adresse refusée".into()
            }]
        );
        assert_eq!(*etapes.borrow(), [(1, 3), (2, 3), (3, 3)]);
        assert!(!suivi.noter(r#"{"erreur":"outlook-ferme"}"#));
        assert!(
            suivi
                .erreur
                .unwrap()
                .message
                .contains("Outlook n'est pas ouvert")
        );
    }

    #[test]
    fn le_script_ne_contient_que_de_l_ascii() {
        // Lu par PowerShell 5.1 : rien qui dépende du codage du fichier.
        assert!(SCRIPT.is_ascii());
        assert!(SCRIPT.contains("param([Parameter(Mandatory = $true)][string]$Donnees)"));
    }

    #[cfg(not(windows))]
    #[test]
    fn explique_que_l_envoi_demande_windows() {
        let piece = crate::essais::fichier_temporaire("outlook", "b.pdf", b"%PDF-1.7");
        let erreur = envoyer(
            &[message("rh@exemple.com", vec![piece])],
            ModeEnvoi::Brouillons,
            |_, _| {},
        )
        .unwrap_err();
        assert!(erreur.message.contains("Windows"));
    }
}
