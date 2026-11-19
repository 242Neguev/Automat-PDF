//! Applications pilotées par un script fixe : Outlook et Office par PowerShell sous Windows
//! (`powershell`), Mail et Office par AppleScript sous macOS (`applescript`). Le script rend compte
//! par des lignes JSON, une par élément ; un chien de garde l'arrête s'il ne dit plus rien.

/// Message pour un script arrêté parce qu'il ne disait plus rien.
pub const NE_REPOND_PLUS: &str = "L'application pilotée ne répond plus (une fenêtre attend peut-être une réponse, ou un mot de passe) : le traitement a été arrêté.";

/// Ce que le script a laissé en sortant.
#[derive(Debug)]
pub struct Fin {
    pub succes: bool,
    /// Arrêté par le chien de garde : il ne disait plus rien depuis trop longtemps (ou, sous
    /// macOS, l'application pilotée n'a pas répondu à temps).
    pub arrete: bool,
    /// Premières lignes de ses erreurs.
    pub erreurs: Vec<String>,
}
