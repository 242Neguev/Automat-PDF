//! Montrer ce que l'application vient de créer : ouvrir le fichier dans l'application habituelle
//! (lecteur de PDF, Word, visionneuse d'images), ou l'afficher, sélectionné, dans son dossier.

use std::path::Path;
use std::process::Command;

use crate::erreur::Erreur;

/// Formats que l'application crée, les seuls qu'elle ouvre : jamais un programme ni un script.
const OUVRABLES: &[&str] = &["pdf", "docx", "png", "jpg", "jpeg"];

/// Le fichier est-il d'un format que l'application crée ?
fn ouvrable(chemin: &Path) -> bool {
    chemin
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| OUVRABLES.contains(&extension.to_lowercase().as_str()))
}

fn introuvable(chemin: &str) -> Erreur {
    Erreur::simple(format!(
        "« {chemin} » est introuvable : il a peut-être été déplacé, renommé ou supprimé."
    ))
}

/// Ouvre un fichier créé par l'application dans l'application habituelle, ou un dossier.
pub fn ouvrir(chemin: &str) -> Result<(), Erreur> {
    let cible = Path::new(chemin);
    if !cible.exists() {
        return Err(introuvable(chemin));
    }
    if !cible.is_dir() && !ouvrable(cible) {
        return Err(Erreur::simple(format!(
            "Automat-PDF n'ouvre que les PDF, les documents Word et les images qu'il crée : « {chemin} » n'en est pas un."
        )));
    }
    lancer(commande_ouvrir(chemin), chemin)
}

/// Affiche un fichier dans son dossier, sélectionné ; un dossier, lui, est simplement ouvert.
pub fn montrer(chemin: &str) -> Result<(), Erreur> {
    let cible = Path::new(chemin);
    if !cible.exists() {
        return Err(introuvable(chemin));
    }
    if cible.is_dir() {
        return lancer(commande_ouvrir(chemin), chemin);
    }
    lancer(commande_montrer(chemin), chemin)
}

/// Ouvre dans le navigateur une page choisie par l'application, jamais une adresse reçue.
pub fn ouvrir_page(page: &'static str) -> Result<(), Erreur> {
    let programme = if cfg!(windows) {
        "explorer.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut commande = Command::new(programme);
    commande.arg(page);
    lancer(commande, page)
}

/// Lance la commande sans l'attendre ; elle est récupérée à sa fin, sans bloquer l'application.
fn lancer(mut commande: Command, chemin: &str) -> Result<(), Erreur> {
    let mut lancee = commande.spawn().map_err(|erreur| {
        Erreur::simple(format!("« {chemin} » n'a pas pu être ouvert : {erreur}"))
    })?;
    std::thread::spawn(move || {
        let _ = lancee.wait();
    });
    Ok(())
}

/// L'Explorateur ne comprend que les barres « \ ».
#[cfg_attr(not(windows), allow(dead_code))]
fn chemin_windows(chemin: &str) -> String {
    chemin.replace('/', "\\")
}

/// Argument de l'Explorateur qui ouvre le dossier et y sélectionne le fichier. Un chemin Windows
/// ne contient jamais de guillemet : il n'y a rien à échapper.
#[cfg_attr(not(windows), allow(dead_code))]
fn argument_selection(chemin: &str) -> String {
    format!("/select,\"{}\"", chemin_windows(chemin))
}

#[cfg(windows)]
fn commande_ouvrir(chemin: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut commande = Command::new("explorer.exe");
    commande.raw_arg(format!("\"{}\"", chemin_windows(chemin)));
    commande
}

#[cfg(windows)]
fn commande_montrer(chemin: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut commande = Command::new("explorer.exe");
    // Tel quel : la bibliothèque standard mettrait des guillemets autour de « /select,… ».
    commande.raw_arg(argument_selection(chemin));
    commande
}

#[cfg(target_os = "macos")]
fn commande_ouvrir(chemin: &str) -> Command {
    let mut commande = Command::new("open");
    commande.arg(chemin);
    commande
}

#[cfg(target_os = "macos")]
fn commande_montrer(chemin: &str) -> Command {
    let mut commande = Command::new("open");
    commande.arg("-R").arg(chemin);
    commande
}

#[cfg(not(any(windows, target_os = "macos")))]
fn commande_ouvrir(chemin: &str) -> Command {
    let mut commande = Command::new("xdg-open");
    commande.arg(chemin);
    commande
}

#[cfg(not(any(windows, target_os = "macos")))]
fn commande_montrer(chemin: &str) -> Command {
    let dossier = Path::new(chemin).parent().unwrap_or(Path::new("."));
    let mut commande = Command::new("xdg-open");
    commande.arg(dossier);
    commande
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn n_ouvre_que_les_formats_crees_par_l_application() {
        assert!(ouvrable(Path::new("C:\\Paie\\Bulletin DUPONT.pdf")));
        assert!(ouvrable(Path::new("/tmp/Convocation.DOCX")));
        assert!(ouvrable(Path::new("page 1.jpeg")));
        assert!(!ouvrable(Path::new("C:\\Windows\\notepad.exe")));
        assert!(!ouvrable(Path::new("installer.ps1")));
        assert!(!ouvrable(Path::new("raccourci.pdf.lnk")));
        assert!(!ouvrable(Path::new("sans extension")));
    }

    #[test]
    fn refuse_un_programme_et_un_fichier_absent() {
        let dossier =
            std::env::temp_dir().join(format!("automat-pdf-ouverture-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let programme = dossier.join("outil.exe");
        std::fs::write(&programme, b"MZ").unwrap();
        let refus = ouvrir(programme.to_str().unwrap()).unwrap_err();
        assert!(refus.message.contains("n'ouvre que"), "{}", refus.message);
        let absent = dossier.join("absent.pdf");
        let absent = absent.to_str().unwrap();
        assert!(ouvrir(absent).unwrap_err().message.contains("introuvable"));
        assert!(montrer(absent).unwrap_err().message.contains("introuvable"));
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn selectionne_le_fichier_dans_l_explorateur() {
        assert_eq!(
            argument_selection("C:/Paie/Octobre/Bulletin DUPONT Jean.pdf"),
            "/select,\"C:\\Paie\\Octobre\\Bulletin DUPONT Jean.pdf\""
        );
    }
}
