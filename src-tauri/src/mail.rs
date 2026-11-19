//! Envoi de fichiers par Mail (macOS), comme par Outlook sous Windows (`outlook`) : chaque message
//! (destinataires, objet, texte, pièces jointes) est préparé dans les Brouillons, ou envoyé tout de
//! suite, depuis le compte réglé dans Mail. Mail est piloté par le script
//! `scripts/macos/mail.applescript` (événements Apple).

use std::time::Duration;

use serde_json::json;

use crate::applescript;
use crate::envoi::{BilanEnvoi, Message, Suivi, verifier};
use crate::erreur::Erreur;
use crate::outlook::ModeEnvoi;

const COMMUN: &str = include_str!("../scripts/macos/commun.applescript");
const SCRIPT: &str = include_str!("../scripts/macos/mail.applescript");
/// Sans nouvelles de Mail pendant ce temps, l'envoi est arrêté ; macOS peut d'abord demander à
/// l'utilisateur s'il autorise Automat-PDF à piloter Mail : on lui en laisse le temps.
const SILENCE_MAX: Duration = Duration::from_secs(300);

/// Les adresses d'un champ « Destinataire » (séparées par « ; » ou « , ») : Mail les veut une par
/// une.
fn adresses(champ: &str) -> Vec<&str> {
    champ
        .split([';', ','])
        .map(str::trim)
        .filter(|adresse| !adresse.is_empty())
        .collect()
}

/// Contenu du fichier lu par le script : les messages, avec des fins de ligne du Mac.
fn preparer(messages: &[Message], mode: ModeEnvoi) -> String {
    let messages: Vec<serde_json::Value> = messages
        .iter()
        .map(|message| {
            json!({
                "a": adresses(&message.a),
                "objet": message.objet,
                "texte": message.texte.replace("\r\n", "\n"),
                "pieces": message.pieces,
            })
        })
        .collect();
    json!({ "envoyer": mode == ModeEnvoi::Direct, "messages": messages }).to_string()
}

/// Message clair pour une erreur générale rapportée par le script.
fn erreur_du_script(code: &str, details: Option<String>) -> Erreur {
    let message = match code {
        "refus" => {
            "macOS ne permet pas à Automat-PDF de piloter Mail. Dans Réglages Système › Confidentialité et sécurité › Automatisation, autorisez « Mail » pour Automat-PDF, puis recommencez."
        }
        "sans-compte" => {
            "Aucun compte n'est réglé dans Mail : ajoutez votre adresse dans Mail (Mail › Réglages › Comptes), ou choisissez « Ma messagerie »."
        }
        "mail-absent" => {
            "Mail ne répond pas. Ouvrez Mail : il faut y avoir réglé votre adresse (s'il propose d'ajouter un compte, faites-le, ou choisissez plutôt « Ma messagerie »), et accepter que macOS laisse Automat-PDF piloter Mail. Puis recommencez."
        }
        _ => "Les messages n'ont pas pu être transmis à Mail.",
    };
    Erreur {
        message: message.into(),
        details: details.into_iter().collect(),
    }
}

/// Prépare (ou envoie) les messages dans Mail ; `progression` reçoit le nombre de messages traités
/// et le total.
pub fn envoyer(
    messages: &[Message],
    mode: ModeEnvoi,
    progression: impl Fn(usize, usize),
) -> Result<BilanEnvoi, Erreur> {
    verifier(messages)?;
    let mut suivi = Suivi::new(messages.len(), progression, erreur_du_script);
    let fin = applescript::lancer(
        &format!("{COMMUN}\n{SCRIPT}"),
        &preparer(messages, mode),
        SILENCE_MAX,
        |ligne| suivi.noter(ligne),
    )?;
    suivi.conclure(fin, "Les messages n'ont pas pu être transmis à Mail.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_les_messages_pour_le_script() {
        let donnees = preparer(
            &[Message {
                a: "a.diallo@exemple.com ; rh@exemple.com,".into(),
                objet: "Votre bulletin d'octobre".into(),
                texte: "Bonjour Aïssatou,\r\nci-joint votre bulletin.\n\nLe service RH".into(),
                pieces: vec!["/Users/rh/Paie été/Bulletin DIALLO.pdf".into()],
            }],
            ModeEnvoi::Brouillons,
        );
        let lu: serde_json::Value = serde_json::from_str(&donnees).unwrap();
        assert_eq!(lu["envoyer"], false);
        assert_eq!(
            lu["messages"][0]["a"],
            json!(["a.diallo@exemple.com", "rh@exemple.com"])
        );
        // Fins de ligne du Mac, accents intacts.
        assert_eq!(
            lu["messages"][0]["texte"],
            "Bonjour Aïssatou,\nci-joint votre bulletin.\n\nLe service RH"
        );
        assert_eq!(
            lu["messages"][0]["pieces"][0],
            "/Users/rh/Paie été/Bulletin DIALLO.pdf"
        );
    }

    #[test]
    fn explique_ce_qui_empeche_l_envoi() {
        assert!(
            erreur_du_script("refus", None)
                .message
                .contains("Confidentialité et sécurité › Automatisation")
        );
        assert!(
            erreur_du_script("sans-compte", None)
                .message
                .starts_with("Aucun compte n'est réglé dans Mail")
        );
        let inconnue = erreur_du_script("donnees", Some("JSON illisible".into()));
        assert_eq!(inconnue.details, ["JSON illisible"]);
    }

    #[test]
    fn les_scripts_ne_contiennent_que_de_l_ascii() {
        assert!(COMMUN.is_ascii());
        assert!(SCRIPT.is_ascii());
        // Chaque erreur générale du script a son explication.
        for code in ["refus", "sans-compte", "mail-absent", "donnees"] {
            assert!(SCRIPT.contains(&format!("erreur:\"{code}\"")), "{code}");
        }
    }
}
