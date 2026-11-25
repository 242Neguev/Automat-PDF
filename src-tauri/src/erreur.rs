//! Erreur transmise telle quelle à l'interface : un message, et le détail des problèmes.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Erreur {
    pub message: String,
    /// Une ligne par problème, quand il y en a plusieurs.
    pub details: Vec<String>,
    /// Ce que l'interface peut en faire, quand il y a quelque chose à faire : `MOT_DE_PASSE`, le PDF
    /// ne s'ouvre qu'avec son mot de passe (le lecteur le demande).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motif: Option<&'static str>,
}

/// Motif d'une erreur : le PDF demande son mot de passe d'ouverture.
pub const MOT_DE_PASSE: &str = "mot_de_passe";

impl Erreur {
    pub fn simple(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            details: Vec::new(),
            motif: None,
        }
    }

    /// Le PDF ne s'ouvre qu'avec son mot de passe.
    pub fn mot_de_passe(message: impl Into<String>) -> Self {
        Self {
            motif: Some(MOT_DE_PASSE),
            ..Self::simple(message)
        }
    }
}

/// Un élément d'une série (message, conversion…) qui n'a pas pu être traité : son rang dans la
/// série, et pourquoi.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Echec {
    pub rang: usize,
    pub message: String,
}
