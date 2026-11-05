//! Erreur transmise telle quelle à l'interface : un message, et le détail des problèmes.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Erreur {
    pub message: String,
    /// Une ligne par problème, quand il y en a plusieurs.
    pub details: Vec<String>,
}

impl Erreur {
    pub fn simple(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            details: Vec::new(),
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
