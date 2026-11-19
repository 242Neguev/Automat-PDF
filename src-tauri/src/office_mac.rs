//! Microsoft Office pour Mac (voir `office`) : Word, Excel et PowerPoint pilotés par AppleScript
//! (`scripts/macos`). Dans leur bac à sable, ils ne lisent et n'écrivent sans rien demander que dans
//! leur propre dossier, leur conteneur ; ailleurs, ils demanderaient pour chaque fichier où
//! l'enregistrer. Automat-PDF y dépose donc une copie de chaque document, puis range à sa place le
//! fichier créé.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use super::{Conversion, Issue, piloter};

const COMMUN: &str = include_str!("../scripts/macos/commun.applescript");
const CONDUITE: &str = include_str!("../scripts/macos/office.applescript");
const WORD: &str = include_str!("../scripts/macos/word.applescript");
const EXCEL: &str = include_str!("../scripts/macos/excel.applescript");
const POWERPOINT: &str = include_str!("../scripts/macos/powerpoint.applescript");

/// Partie du script propre à l'application, et son identifiant (le nom de son conteneur).
fn application(nom: &str) -> (&'static str, &'static str) {
    match nom {
        "Word" => (WORD, "com.microsoft.Word"),
        "Excel" => (EXCEL, "com.microsoft.Excel"),
        _ => (POWERPOINT, "com.microsoft.Powerpoint"),
    }
}

/// Le script qui pilote cette application : la partie commune, la sienne, puis la conduite des
/// conversions. Il ne nomme qu'elle : AppleScript demanderait sinon où trouver celles qui manquent.
pub fn script(nom: &str) -> String {
    format!("{COMMUN}\n{}\n{CONDUITE}", application(nom).0)
}

/// Word, Excel ou PowerPoint est installé : Office pour Mac s'installe dans « Applications ».
pub fn installee(nom: &str) -> bool {
    Path::new("/Applications")
        .join(format!("Microsoft {nom}.app"))
        .is_dir()
}

/// Dossier des fichiers temporaires de l'application, dans son conteneur : absent tant qu'elle n'a
/// jamais été ouverte.
fn conteneur(nom: &str) -> Option<PathBuf> {
    let dossier = PathBuf::from(std::env::var_os("HOME")?)
        .join("Library/Containers")
        .join(application(nom).1)
        .join("Data/tmp");
    dossier.is_dir().then_some(dossier)
}

/// Dossiers déposés dans les conteneurs, effacés quoi qu'il arrive.
struct Depots(Vec<PathBuf>);

impl Drop for Depots {
    fn drop(&mut self) {
        for dossier in &self.0 {
            let _ = std::fs::remove_dir_all(dossier);
        }
    }
}

/// Copie le document dans le conteneur de son application : la conversion telle qu'Office la fera.
/// Une application absente, ou jamais ouverte (son premier lancement, avec ses fenêtres, se fait à
/// la main), n'est pas pilotée : la conversion est laissée au relais.
fn deposer(
    conversion: &Conversion,
    rang: usize,
    lot: &str,
    depots: &mut Depots,
) -> Result<Conversion, Issue> {
    let nom = conversion.sorte.application();
    if !installee(nom) {
        return Err(Issue::Absente(nom.into()));
    }
    let Some(base) = conteneur(nom) else {
        return Err(Issue::Bloquee(nom.into()));
    };
    let dossier = base.join(lot);
    if !depots.0.contains(&dossier) {
        depots.0.push(dossier.clone());
    }
    let ici = dossier.join(rang.to_string());
    let echec = |erreur: std::io::Error| {
        Issue::Echouee(format!(
            "Le document n'a pas pu être confié à Microsoft {nom} : {erreur}"
        ))
    };
    std::fs::create_dir_all(&ici).map_err(echec)?;
    let source = ici.join(conversion.source.file_name().unwrap_or_default());
    std::fs::copy(&conversion.source, &source).map_err(echec)?;
    // Un nom propre à Automat-PDF : Word donne au document converti le nom du fichier enregistré,
    // et le retrouve ainsi sans le confondre avec un document de l'utilisateur.
    let extension = conversion
        .cible
        .extension()
        .unwrap_or_default()
        .to_string_lossy();
    Ok(Conversion {
        source,
        cible: ici.join(format!("{lot}-{rang}.{extension}")),
        sorte: conversion.sorte,
    })
}

/// Range le fichier créé par Office à sa place.
fn ranger(cree: &Path, cible: &Path) -> Issue {
    let range = std::fs::rename(cree, cible).or_else(|_| std::fs::copy(cree, cible).map(|_| ()));
    match range {
        Ok(()) => Issue::Faite,
        Err(erreur) => Issue::Echouee(format!(
            "Le fichier converti n'a pas pu être enregistré : {erreur}"
        )),
    }
}

/// Conversions par Office pour Mac, dans les conteneurs (voir `office::piloter`) ; `progression`
/// reçoit le nombre de conversions terminées.
pub fn avec_office(conversions: &[Conversion], progression: &impl Fn(usize)) -> Vec<Issue> {
    static COMPTEUR: AtomicU32 = AtomicU32::new(0);
    let lot = format!(
        "automat-pdf-{}-{}",
        std::process::id(),
        COMPTEUR.fetch_add(1, Ordering::Relaxed)
    );
    let mut depots = Depots(Vec::new());
    let mut issues: Vec<Option<Issue>> = Vec::with_capacity(conversions.len());
    let mut deposees = Vec::new();
    for (rang, conversion) in conversions.iter().enumerate() {
        match deposer(conversion, rang, &lot, &mut depots) {
            Ok(deposee) => {
                deposees.push((rang, deposee));
                issues.push(None);
            }
            Err(issue) => issues.push(Some(issue)),
        }
    }
    // Les documents qui n'ont pas pu être copiés sont terminés : comptés d'emblée.
    let terminees = issues
        .iter()
        .flatten()
        .filter(|issue| super::definitive(issue))
        .count();
    if terminees > 0 {
        progression(terminees);
    }
    let a_piloter: Vec<Conversion> = deposees
        .iter()
        .map(|(_, deposee)| Conversion {
            source: deposee.source.clone(),
            cible: deposee.cible.clone(),
            sorte: deposee.sorte,
        })
        .collect();
    let par_office = piloter(&a_piloter, &|faites| progression(terminees + faites));
    for ((rang, deposee), issue) in deposees.iter().zip(par_office) {
        issues[*rang] = Some(match issue {
            Issue::Faite => ranger(&deposee.cible, &conversions[*rang].cible),
            autre => autre,
        });
    }
    issues
        .into_iter()
        .map(|issue| issue.unwrap_or_else(|| Issue::Echouee(String::new())))
        .collect()
}

/// Arrête les applications d'Office restées bloquées après un abandon : les processus apparus
/// depuis leur lancement par le script, jamais ceux de l'utilisateur (ouverts avant).
pub fn arreter_les_restes(lancees: &[(String, Vec<u32>)]) {
    for (nom, avant) in lancees {
        let Ok(sortie) = Command::new("/usr/bin/pgrep")
            .args(["-x", &format!("Microsoft {nom}")])
            .output()
        else {
            continue;
        };
        let identifiants = String::from_utf8_lossy(&sortie.stdout)
            .lines()
            .filter_map(|ligne| ligne.trim().parse::<u32>().ok())
            .filter(|identifiant| !avant.contains(identifiant))
            .collect::<Vec<_>>();
        for identifiant in identifiants {
            let _ = Command::new("/bin/kill")
                .args(["-9", &identifiant.to_string()])
                .output();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::office::Sorte;

    #[test]
    fn chaque_script_ne_nomme_que_son_application() {
        for (nom, autres) in [
            ("Word", ["Microsoft Excel", "Microsoft PowerPoint"]),
            ("Excel", ["Microsoft Word", "Microsoft PowerPoint"]),
            ("PowerPoint", ["Microsoft Word", "Microsoft Excel"]),
        ] {
            let script = script(nom);
            assert!(script.is_ascii());
            assert!(script.starts_with("-- Commun"), "{nom}");
            assert!(script.contains(&format!("tell application \"Microsoft {nom}\"")));
            assert!(script.contains(&format!("property nomApplication : \"{nom}\"")));
            for autre in autres {
                assert!(!script.contains(autre), "{nom} : {autre}");
            }
            // La conduite appelle chaque gestionnaire de la partie de l'application.
            for gestionnaire in [
                "demarrer",
                "convertir",
                "documentsOuverts",
                "documentsModifies",
                "quitter",
            ] {
                assert!(
                    script.contains(&format!("on {gestionnaire}(")),
                    "{nom} : {gestionnaire}"
                );
                assert!(
                    script.contains(&format!("my {gestionnaire}(")),
                    "{nom} : {gestionnaire}"
                );
            }
        }
        // Chaque sorte est convertie par l'application dont le script la connaît.
        assert!(script("Word").contains("\"pdf-word\""));
        assert_eq!(Sorte::PdfWord.application(), "Word");
    }

    #[test]
    fn range_le_fichier_cree_a_sa_place() {
        let cree =
            crate::essais::fichier_temporaire("office-mac", "automat-pdf-1-0-0.pdf", b"%PDF");
        let cible = Path::new(&cree).with_file_name("Note de service été.pdf");
        let _ = std::fs::remove_file(&cible);
        assert_eq!(ranger(Path::new(&cree), &cible), Issue::Faite);
        assert_eq!(std::fs::read(&cible).unwrap(), b"%PDF");
        assert!(!Path::new(&cree).exists());
        assert!(matches!(
            ranger(Path::new("/nulle/part.pdf"), &cible),
            Issue::Echouee(_)
        ));
    }
}
