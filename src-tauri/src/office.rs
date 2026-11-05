//! Conversions avec Microsoft Office, piloté sans l'afficher par `scripts/office.ps1` (Windows) :
//! documents Word, classeurs Excel et présentations PowerPoint en PDF, LibreOffice prenant le
//! relais s'il est installé et qu'Office manque ; PDF en document Word modifiable (Word seulement).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::erreur::{Echec, Erreur};
use crate::libreoffice;
use crate::powershell;

const SCRIPT: &str = include_str!("../scripts/office.ps1");
/// Sans nouvelles d'Office pendant ce temps (une fenêtre attend une réponse…), tout est arrêté.
/// Un Office dont la licence a expiré peut mettre trois minutes à démarrer : on les lui laisse.
const SILENCE_MAX: Duration = Duration::from_secs(300);
/// Temps laissé pour arrêter les applications d'Office restées bloquées.
const ARRET_MAX: Duration = Duration::from_secs(60);

/// Ce que l'on fait d'un fichier, nommé comme dans le script (`sorte`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Sorte {
    Word,
    Excel,
    /// « powerpoint » : en kebab-case, ce serait « power-point », que le script ne connaît pas.
    #[serde(rename = "powerpoint")]
    PowerPoint,
    PdfWord,
}

impl Sorte {
    /// Document d'Office à mettre en PDF, d'après son extension.
    fn de(chemin: &Path) -> Option<Sorte> {
        let extension = chemin.extension()?.to_string_lossy().to_lowercase();
        match extension.as_str() {
            "doc" | "docx" | "docm" | "dot" | "dotx" | "rtf" | "odt" => Some(Sorte::Word),
            "xls" | "xlsx" | "xlsm" | "xlsb" | "ods" => Some(Sorte::Excel),
            "ppt" | "pptx" | "pptm" | "pps" | "ppsx" | "odp" => Some(Sorte::PowerPoint),
            _ => None,
        }
    }

    /// Application d'Office qui fait la conversion, nommée comme dans le script.
    fn application(self) -> &'static str {
        match self {
            Sorte::Word | Sorte::PdfWord => "Word",
            Sorte::Excel => "Excel",
            Sorte::PowerPoint => "PowerPoint",
        }
    }
}

/// Fichiers créés, et conversions qui ont échoué (rang dans la liste donnée).
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversions {
    pub crees: Vec<String>,
    pub echecs: Vec<Echec>,
}

/// Une ligne du compte rendu du script.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Ligne {
    /// Word, Excel ou PowerPoint va être lancé ; `avant` : ses processus déjà ouverts.
    Demarrage {
        demarrage: String,
        #[serde(default)]
        avant: Vec<u32>,
    },
    /// L'application lancée répond.
    Pret { pret: String },
    /// Une conversion, d'après son rang dans le lot confié au script.
    Conversion {
        rang: usize,
        ok: bool,
        #[serde(default)]
        absent: Option<String>,
        #[serde(default)]
        details: Option<String>,
    },
}

/// Ce que l'on sait d'une exécution du script, d'après ses lignes.
#[derive(Debug, Default)]
struct Suivi {
    /// Applications lancées, avec leurs processus déjà ouverts avant.
    lancees: Vec<(String, Vec<u32>)>,
    /// Application lancée qui n'a pas encore répondu.
    en_demarrage: Option<String>,
}

impl Suivi {
    /// Note une ligne du script ; rend le compte rendu d'une conversion (son rang dans le lot, son
    /// issue). Ce que PowerShell écrit d'autre (avertissements) est ignoré.
    fn noter(&mut self, ligne: &str) -> Option<(usize, Issue)> {
        match serde_json::from_str::<Ligne>(ligne).ok()? {
            Ligne::Demarrage { demarrage, avant } => {
                self.en_demarrage = Some(demarrage.clone());
                self.lancees.push((demarrage, avant));
                None
            }
            Ligne::Pret { pret } => {
                if self.en_demarrage.as_ref() == Some(&pret) {
                    self.en_demarrage = None;
                }
                None
            }
            Ligne::Conversion {
                rang,
                ok,
                absent,
                details,
            } => {
                // Une application absente, ou impossible à piloter, a fini de démarrer elle aussi.
                self.en_demarrage = None;
                let issue = match (ok, absent) {
                    (true, _) => Issue::Faite,
                    (false, Some(application)) => Issue::Absente(application),
                    (false, None) => Issue::Echouee(details.unwrap_or_default()),
                };
                Some((rang, issue))
            }
        }
    }
}

/// Ce qu'il est advenu d'une conversion.
#[derive(Debug, Clone, PartialEq)]
enum Issue {
    Faite,
    /// L'application d'Office nécessaire n'est pas installée, ou ne se laisse pas piloter.
    Absente(String),
    /// L'application d'Office nécessaire n'a pas répondu à son lancement : elle a été arrêtée.
    Bloquee(String),
    Echouee(String),
}

/// Message pour une application d'Office qui n'a pas répondu à son lancement.
fn bloquee(application: &str) -> String {
    format!(
        "Microsoft {application} ne répond pas : une fenêtre attend sans doute une réponse (activation d'Office, premier lancement). Ouvrez {application} une fois à la main, fermez ses fenêtres, puis recommencez."
    )
}

struct Conversion {
    source: PathBuf,
    cible: PathBuf,
    sorte: Sorte,
}

/// Chemins des fichiers à créer : le nom de chaque source avec la nouvelle extension, sans écraser
/// un fichier existant ni un autre de la série (« Rapport (2).pdf »).
fn cibles(sources: &[PathBuf], dossier: &Path, extension: &str) -> Vec<PathBuf> {
    let mut pris = HashSet::new();
    sources
        .iter()
        .map(|source| {
            let base = source
                .file_stem()
                .map(|nom| nom.to_string_lossy().into_owned())
                .unwrap_or_else(|| "document".into());
            (1..)
                .map(|n| match n {
                    1 => dossier.join(format!("{base}.{extension}")),
                    _ => dossier.join(format!("{base} ({n}).{extension}")),
                })
                .find(|chemin| {
                    !chemin.exists() && pris.insert(chemin.to_string_lossy().to_lowercase())
                })
                .unwrap_or_default()
        })
        .collect()
}

/// Arrête les applications d'Office restées bloquées après un abandon : les processus apparus
/// depuis leur lancement par le script, sans fenêtre (jamais ceux de l'utilisateur). Le chien de
/// garde n'arrête que PowerShell : Office, lancé à part par Windows, continuerait sinon, caché.
fn arreter_les_restes(lancees: &[(String, Vec<u32>)]) {
    if lancees.is_empty() {
        return;
    }
    let donnees = json!({
        "arreter": lancees
            .iter()
            .map(|(application, avant)| json!({ "application": application, "avant": avant }))
            .collect::<Vec<_>>()
    })
    .to_string();
    let _ = powershell::lancer(SCRIPT, &donnees, ARRET_MAX, |_| true);
}

/// Issue sur laquelle on ne revient pas : ni application absente, ni application bloquée (un
/// relais peut alors convertir à sa place).
fn definitive(issue: &Issue) -> bool {
    !matches!(issue, Issue::Absente(_) | Issue::Bloquee(_))
}

/// Conversions faites par Office ; `progression` reçoit le nombre de conversions terminées (les
/// conversions laissées à un relais sont comptées par lui). Une application qui ne répond pas à son
/// lancement, ou qui y interrompt le script, est abandonnée (`Bloquee`) et les autres conversions
/// reprises sans elle ; un document qui bloque Office (fenêtre, mot de passe) ou l'interrompt est
/// seul abandonné, les suivants sont repris.
fn avec_office(conversions: &[Conversion], progression: &impl Fn(usize)) -> Vec<Issue> {
    if !cfg!(windows) {
        return conversions
            .iter()
            .map(|conversion| Issue::Absente(conversion.sorte.application().into()))
            .collect();
    }
    let mut issues: Vec<Option<Issue>> = vec![None; conversions.len()];
    let mut traitees = 0;
    loop {
        let lot: Vec<usize> = (0..conversions.len())
            .filter(|&rang| issues[rang].is_none())
            .collect();
        if lot.is_empty() {
            break;
        }
        let donnees = json!({
            "conversions": lot
                .iter()
                .map(|&rang| {
                    let c = &conversions[rang];
                    json!({ "source": c.source, "cible": c.cible, "sorte": c.sorte })
                })
                .collect::<Vec<_>>()
        })
        .to_string();
        let mut suivi = Suivi::default();
        let resultat = powershell::lancer(SCRIPT, &donnees, SILENCE_MAX, |ligne| {
            if let Some((rang, issue)) = suivi.noter(ligne)
                && let Some(&rang) = lot.get(rang)
                && issues[rang].is_none()
            {
                if definitive(&issue) {
                    traitees += 1;
                    progression(traitees);
                }
                issues[rang] = Some(issue);
            }
            true
        });
        let fin = match resultat {
            Ok(fin) => fin,
            Err(erreur) => {
                for &rang in &lot {
                    issues[rang].get_or_insert_with(|| Issue::Echouee(erreur.message.clone()));
                }
                break;
            }
        };
        // Terminé normalement : tout ce que le script pouvait faire est fait.
        if !fin.arrete && fin.succes {
            break;
        }
        // Arrêté parce qu'il ne répondait plus, ou interrompu par une erreur : les applications
        // qu'il a lancées sont arrêtées, ce qui l'a bloqué est mis de côté, et le reste est repris.
        arreter_les_restes(&suivi.lancees);
        let abandon = if fin.arrete {
            "Ce document a bloqué Office (une fenêtre attend peut-être une réponse, ou un mot de passe) : il a été abandonné.".to_string()
        } else {
            match fin.erreurs.first() {
                Some(erreur) => format!("La conversion s'est interrompue : {erreur}"),
                None => "La conversion s'est interrompue.".to_string(),
            }
        };
        let restantes: Vec<usize> = lot
            .into_iter()
            .filter(|&rang| issues[rang].is_none())
            .collect();
        let abandonnees: Vec<(usize, Issue)> = match &suivi.en_demarrage {
            // L'application ne démarre pas : ses conversions sont abandonnées.
            Some(application) => restantes
                .iter()
                .filter(|&&rang| conversions[rang].sorte.application() == application)
                .map(|&rang| (rang, Issue::Bloquee(application.clone())))
                .collect(),
            // Le document en cours bloquait l'application : lui seul est abandonné.
            None => restantes
                .first()
                .map(|&rang| (rang, Issue::Echouee(abandon.clone())))
                .into_iter()
                .collect(),
        };
        if abandonnees.is_empty() {
            // Rien à mettre de côté (arrêt pendant la fermeture d'Office) : on ne recommence pas.
            for rang in restantes {
                issues[rang] = Some(Issue::Echouee(abandon.clone()));
            }
            break;
        }
        for (rang, issue) in abandonnees {
            if definitive(&issue) {
                traitees += 1;
                progression(traitees);
            }
            issues[rang] = Some(issue);
        }
    }
    issues
        .into_iter()
        .map(|issue| {
            issue.unwrap_or_else(|| {
                Issue::Echouee("Office n'a pas rendu compte de cette conversion.".into())
            })
        })
        .collect()
}

/// Une conversion en PDF par LibreOffice, dans un dossier provisoire (il écraserait sinon un
/// fichier du même nom), puis mise à sa place.
fn avec_libreoffice(soffice: &Path, conversion: &Conversion) -> Issue {
    let provisoire =
        std::env::temp_dir().join(format!("automat-pdf-libreoffice-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&provisoire);
    // Profil à part : la conversion fonctionne même si LibreOffice est déjà ouvert.
    let profil = format!(
        "-env:UserInstallation=file:///{}",
        provisoire
            .join("profil")
            .to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches('/')
    );
    let sortie = Command::new(soffice)
        .args([
            profil.as_str(),
            "--headless",
            "--norestore",
            "--convert-to",
            "pdf",
            "--outdir",
        ])
        .arg(&provisoire)
        .arg(&conversion.source)
        .output();
    let produit = provisoire.join(
        conversion
            .source
            .with_extension("pdf")
            .file_name()
            .unwrap_or_default(),
    );
    let issue = match sortie {
        Ok(_) if produit.is_file() => match std::fs::copy(&produit, &conversion.cible) {
            Ok(_) => Issue::Faite,
            Err(erreur) => Issue::Echouee(format!("Le PDF n'a pas pu être enregistré : {erreur}")),
        },
        Ok(sortie) => Issue::Echouee(format!(
            "LibreOffice n'a pas pu convertir ce document. {}",
            String::from_utf8_lossy(&sortie.stderr).trim()
        )),
        Err(erreur) => Issue::Echouee(format!("LibreOffice n'a pas pu être lancé : {erreur}")),
    };
    let _ = std::fs::remove_dir_all(&provisoire);
    issue
}

/// Bilan des conversions, dans l'ordre des sources.
fn bilan(
    conversions: &[Option<Conversion>],
    issues: Vec<Issue>,
    absente: impl Fn(&str) -> String,
) -> Conversions {
    let mut resultat = Conversions::default();
    for (rang, (conversion, issue)) in conversions.iter().zip(issues).enumerate() {
        match (conversion, issue) {
            (Some(conversion), Issue::Faite) => {
                resultat
                    .crees
                    .push(conversion.cible.to_string_lossy().into_owned());
            }
            (_, Issue::Absente(application)) => resultat.echecs.push(Echec {
                rang,
                message: absente(&application),
            }),
            (_, Issue::Bloquee(application)) => resultat.echecs.push(Echec {
                rang,
                message: bloquee(&application),
            }),
            (_, Issue::Echouee(message)) => resultat.echecs.push(Echec { rang, message }),
            (None, Issue::Faite) => {}
        }
    }
    resultat
}

/// Conversions prévues, dans l'ordre des sources ; `None` pour un fichier qui ne s'y prête pas.
fn preparer(
    sources: &[String],
    dossier: &str,
    extension: &str,
    sorte: impl Fn(&Path) -> Option<Sorte>,
) -> Result<(Vec<Option<Conversion>>, Vec<Issue>), Erreur> {
    std::fs::create_dir_all(dossier).map_err(|erreur| {
        Erreur::simple(format!("Impossible d'écrire dans « {dossier} » : {erreur}"))
    })?;
    let chemins: Vec<PathBuf> = sources.iter().map(PathBuf::from).collect();
    let cibles = cibles(&chemins, Path::new(dossier), extension);
    let mut issues = Vec::new();
    let conversions = chemins
        .into_iter()
        .zip(cibles)
        .map(|(source, cible)| match sorte(&source) {
            _ if !source.is_file() => {
                issues.push(Issue::Echouee("Fichier introuvable.".into()));
                None
            }
            Some(sorte) => {
                issues.push(Issue::Faite);
                Some(Conversion {
                    source,
                    cible,
                    sorte,
                })
            }
            None => {
                issues.push(Issue::Echouee(
                    "Ce genre de fichier ne se convertit pas ici.".into(),
                ));
                None
            }
        })
        .collect();
    Ok((conversions, issues))
}

/// Convertit par Office les conversions prévues (marquées `Faite` d'avance) et range les issues.
fn convertir(
    conversions: &[Option<Conversion>],
    issues: &mut [Issue],
    progression: &impl Fn(usize, usize),
) {
    let rangs: Vec<usize> = (0..conversions.len())
        .filter(|&rang| conversions[rang].is_some() && issues[rang] == Issue::Faite)
        .collect();
    let a_faire: Vec<&Conversion> = rangs
        .iter()
        .filter_map(|&rang| conversions[rang].as_ref())
        .collect();
    let par_office = avec_office(
        &a_faire
            .iter()
            .map(|c| Conversion {
                source: c.source.clone(),
                cible: c.cible.clone(),
                sorte: c.sorte,
            })
            .collect::<Vec<_>>(),
        &|faites| progression(faites, rangs.len()),
    );
    for (rang, issue) in rangs.into_iter().zip(par_office) {
        issues[rang] = issue;
    }
}

/// Les conversions qu'Office n'a pas pu faire (application absente ou bloquée), confiées une à une
/// à `relais` ; l'avancement reprend où Office l'a laissé.
fn relayer(
    conversions: &[Option<Conversion>],
    issues: &mut [Issue],
    progression: &impl Fn(usize, usize),
    relais: impl Fn(&Conversion) -> Issue,
) {
    let total = conversions.iter().flatten().count();
    let mut faites = conversions
        .iter()
        .zip(issues.iter())
        .filter(|(conversion, issue)| conversion.is_some() && definitive(issue))
        .count();
    for (conversion, issue) in conversions.iter().zip(issues.iter_mut()) {
        if let (Some(conversion), false) = (conversion, definitive(issue)) {
            *issue = relais(conversion);
            faites += 1;
            progression(faites, total);
        }
    }
}

/// Documents Word, classeurs Excel et présentations PowerPoint en PDF, dans `dossier`.
pub fn en_pdf(
    sources: &[String],
    dossier: &str,
    progression: impl Fn(usize, usize),
) -> Result<Conversions, Erreur> {
    let (conversions, mut issues) = preparer(sources, dossier, "pdf", Sorte::de)?;
    convertir(&conversions, &mut issues, &progression);
    // Office manque, ne se laisse pas piloter ou ne répond pas : LibreOffice prend le relais, s'il
    // est installé.
    if let Some(soffice) = libreoffice::trouver() {
        relayer(&conversions, &mut issues, &progression, |conversion| {
            avec_libreoffice(&soffice, conversion)
        });
    }
    Ok(bilan(&conversions, issues, |application| {
        format!("Microsoft {application} n'est pas installé sur cet ordinateur, ni LibreOffice.")
    }))
}

/// PDF en documents Word modifiables (.docx), dans `dossier` : par Microsoft Word, ou à défaut par
/// Automat-PDF lui-même.
pub fn pdf_en_word(
    sources: &[String],
    dossier: &str,
    progression: impl Fn(usize, usize),
) -> Result<Conversions, Erreur> {
    let sorte = |chemin: &Path| {
        chemin
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
            .then_some(Sorte::PdfWord)
    };
    let (conversions, mut issues) = preparer(sources, dossier, "docx", sorte)?;
    convertir(&conversions, &mut issues, &progression);
    // Word manque, ne se laisse pas piloter ou ne répond pas : Automat-PDF convertit lui-même,
    // avec une mise en page plus simple.
    relayer(&conversions, &mut issues, &progression, sans_word);
    // Plus rien n'attend Word : chaque PDF qu'il n'a pas pu convertir est passé au relais.
    Ok(bilan(&conversions, issues, |application| {
        format!("Microsoft {application} n'a pas pu convertir ce PDF.")
    }))
}

/// Un PDF en document Word par Automat-PDF lui-même, sans Word.
fn sans_word(conversion: &Conversion) -> Issue {
    let nom = conversion
        .source
        .file_name()
        .map_or_else(String::new, |nom| nom.to_string_lossy().into_owned());
    let resultat = std::fs::read(&conversion.source)
        .map_err(|erreur| format!("« {nom} » n'a pas pu être lu : {erreur}"))
        .and_then(|octets| {
            crate::pdf_en_word::convertir(Arc::new(octets), &nom).map_err(|erreur| erreur.message)
        })
        .and_then(|docx| {
            std::fs::write(&conversion.cible, docx)
                .map_err(|erreur| format!("Le document Word n'a pas pu être enregistré : {erreur}"))
        });
    match resultat {
        Ok(()) => Issue::Faite,
        Err(message) => Issue::Echouee(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::fichier_temporaire;

    #[test]
    fn le_relais_reprend_l_avancement_ou_office_l_a_laisse() {
        let conversion = |nom: &str| {
            Some(Conversion {
                source: PathBuf::from(format!("{nom}.pdf")),
                cible: PathBuf::from(format!("{nom}.docx")),
                sorte: Sorte::PdfWord,
            })
        };
        // Word absent pour « a », fait « b », bloqué sur « c » ; le quatrième fichier manquait.
        let conversions = [conversion("a"), conversion("b"), conversion("c"), None];
        let mut issues = vec![
            Issue::Absente("Word".into()),
            Issue::Faite,
            Issue::Bloquee("Word".into()),
            Issue::Echouee("Fichier introuvable.".into()),
        ];
        let avancement = std::cell::RefCell::new(Vec::new());
        relayer(
            &conversions,
            &mut issues,
            &|fait, total| avancement.borrow_mut().push((fait, total)),
            |conversion| {
                if conversion.source.ends_with("c.pdf") {
                    Issue::Echouee("Illisible.".into())
                } else {
                    Issue::Faite
                }
            },
        );
        assert_eq!(
            issues,
            [
                Issue::Faite,
                Issue::Faite,
                Issue::Echouee("Illisible.".into()),
                Issue::Echouee("Fichier introuvable.".into())
            ]
        );
        assert_eq!(avancement.into_inner(), [(2, 3), (3, 3)]);
    }

    #[test]
    fn reconnait_les_documents_d_office() {
        assert_eq!(Sorte::de(Path::new("Rapport.DOCX")), Some(Sorte::Word));
        assert_eq!(Sorte::de(Path::new("Budget 2026.xlsx")), Some(Sorte::Excel));
        assert_eq!(
            Sorte::de(Path::new("Présentation.pptx")),
            Some(Sorte::PowerPoint)
        );
        assert_eq!(Sorte::de(Path::new("photo.jpg")), None);
        assert_eq!(Sorte::de(Path::new("sans extension")), None);
    }

    #[test]
    fn n_ecrase_ni_un_fichier_existant_ni_un_autre_de_la_serie() {
        let existant = fichier_temporaire("office-cibles", "Rapport.pdf", b"%PDF");
        let dossier = Path::new(&existant).parent().unwrap().to_path_buf();
        let _ = std::fs::remove_file(dossier.join("Rapport (2).pdf"));
        let noms: Vec<String> = cibles(
            &[
                PathBuf::from("/a/Rapport.docx"),
                PathBuf::from("/b/rapport.xlsx"),
                PathBuf::from("/c/Budget.xlsx"),
            ],
            &dossier,
            "pdf",
        )
        .iter()
        .map(|chemin| chemin.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
        assert_eq!(noms, ["Rapport (2).pdf", "rapport (3).pdf", "Budget.pdf"]);
    }

    #[test]
    fn explique_ce_qui_manque() {
        let document = fichier_temporaire("office", "Note de service.docx", b"PK");
        let pdf = fichier_temporaire("office", "Contrat.pdf", b"%PDF");
        let image = fichier_temporaire("office", "photo.jpg", b"jpg");
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("office-sortie");
        let dossier = dossier.to_str().unwrap();
        if cfg!(windows) || libreoffice::trouver().is_some() {
            return; // poste équipé : la conversion serait réellement lancée
        }
        let conversions = en_pdf(
            &[document.clone(), image, "/nulle/part.docx".into()],
            dossier,
            |_, _| {},
        )
        .unwrap();
        assert!(conversions.crees.is_empty());
        let messages: Vec<&str> = conversions
            .echecs
            .iter()
            .map(|e| e.message.as_str())
            .collect();
        assert_eq!(
            messages,
            [
                "Microsoft Word n'est pas installé sur cet ordinateur, ni LibreOffice.",
                "Ce genre de fichier ne se convertit pas ici.",
                "Fichier introuvable."
            ]
        );
        // Sans Word, Automat-PDF convertit lui-même : ce faux PDF ne se lit pas.
        let word = pdf_en_word(&[pdf, document], dossier, |_, _| {}).unwrap();
        assert_eq!(
            word.echecs[0].message,
            "« Contrat.pdf » n'a pas pu être lu."
        );
        assert_eq!(
            word.echecs[1],
            Echec {
                rang: 1,
                message: "Ce genre de fichier ne se convertit pas ici.".into()
            }
        );
    }

    #[test]
    fn le_script_ne_contient_que_de_l_ascii() {
        assert!(SCRIPT.is_ascii());
        assert!(SCRIPT.contains("param([Parameter(Mandatory = $true)][string]$Donnees)"));
    }

    #[test]
    fn le_script_connait_chaque_sorte() {
        // Une sorte inconnue du script l'interromprait avant même de lancer Office (PowerPoint,
        // transmis en « power-point », n'était jamais converti) ; une sorte sans branche de
        // conversion serait annoncée faite sans l'avoir été.
        for sorte in [Sorte::Word, Sorte::Excel, Sorte::PowerPoint, Sorte::PdfWord] {
            let valeur = serde_json::to_value(sorte).unwrap();
            let nom = valeur.as_str().unwrap();
            let application = format!("'{nom}' = '{}'", sorte.application());
            assert!(
                SCRIPT.contains(&application),
                "{application} : absent du script"
            );
            let branche = format!("'{nom}' {{");
            assert!(SCRIPT.contains(&branche), "{branche} : absent du script");
        }
    }

    #[test]
    fn suit_le_compte_rendu_du_script() {
        let mut suivi = Suivi::default();
        assert_eq!(suivi.noter(r#"{"demarrage":"Word","avant":[]}"#), None);
        assert_eq!(suivi.en_demarrage.as_deref(), Some("Word"));
        assert_eq!(suivi.noter("AVERTISSEMENT : sans importance"), None);
        assert_eq!(suivi.noter(r#"{"pret":"Word"}"#), None);
        assert_eq!(suivi.en_demarrage, None);
        assert_eq!(
            suivi.noter(r#"{"ok":true,"rang":0}"#),
            Some((0, Issue::Faite))
        );
        assert_eq!(
            suivi.noter(r#"{"rang":1,"details":"Mot de passe incorrect.","ok":false}"#),
            Some((1, Issue::Echouee("Mot de passe incorrect.".into())))
        );
        // PowerPoint déjà ouvert par l'utilisateur (processus 4242), mais impossible à piloter.
        assert_eq!(
            suivi.noter(r#"{"avant":[4242],"demarrage":"PowerPoint"}"#),
            None
        );
        assert_eq!(
            suivi.noter(r#"{"rang":2,"ok":false,"absent":"PowerPoint"}"#),
            Some((2, Issue::Absente("PowerPoint".into())))
        );
        assert_eq!(suivi.en_demarrage, None);
        // Excel lancé ne répond pas : il reste « en démarrage », ce qui désigne le coupable.
        assert_eq!(
            suivi.noter(r#"{"demarrage":"Excel","avant":[17,23]}"#),
            None
        );
        assert_eq!(suivi.en_demarrage.as_deref(), Some("Excel"));
        assert_eq!(
            suivi.lancees,
            [
                ("Word".to_string(), vec![]),
                ("PowerPoint".into(), vec![4242]),
                ("Excel".into(), vec![17, 23])
            ]
        );
    }

    #[test]
    fn explique_qu_une_application_ne_repond_pas() {
        let conversion = |source: &str, sorte| {
            Some(Conversion {
                source: PathBuf::from(source),
                cible: PathBuf::from(source).with_extension("pdf"),
                sorte,
            })
        };
        let resultat = bilan(
            &[
                conversion("Note.docx", Sorte::Word),
                conversion("Budget.xlsx", Sorte::Excel),
            ],
            vec![Issue::Bloquee("Word".into()), Issue::Faite],
            |_| String::new(),
        );
        assert_eq!(resultat.crees, ["Budget.pdf"]);
        assert_eq!(resultat.echecs.len(), 1);
        assert_eq!(resultat.echecs[0].rang, 0);
        assert!(
            resultat.echecs[0]
                .message
                .starts_with("Microsoft Word ne répond pas"),
            "{}",
            resultat.echecs[0].message
        );
    }

    /// Essai réel, sur un PC Windows avec Office (ou LibreOffice) : `cargo test -- --ignored`.
    #[cfg(windows)]
    #[test]
    #[ignore = "pilote Office ou LibreOffice pour de vrai : plusieurs minutes"]
    fn convertit_les_fichiers_d_essai() {
        let essais = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("docs")
            .join("test-windows")
            .join("fichiers");
        let essai = |nom: &str| essais.join(nom).to_string_lossy().into_owned();
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("office réel été");
        let _ = std::fs::remove_dir_all(&dossier);
        let dossier = dossier.to_str().unwrap();
        let en_pdf = en_pdf(
            &[
                essai("Convocation formation.docx"),
                essai("Liste des agents.xlsx"),
            ],
            dossier,
            |_, _| {},
        )
        .unwrap();
        assert!(en_pdf.echecs.is_empty(), "{:?}", en_pdf.echecs);
        let en_word = pdf_en_word(&[essai("Bulletins octobre.pdf")], dossier, |_, _| {}).unwrap();
        assert!(en_word.echecs.is_empty(), "{:?}", en_word.echecs);
        for cree in en_pdf.crees.iter().chain(&en_word.crees) {
            let debut = if cree.ends_with(".pdf") { "%PDF" } else { "PK" };
            assert!(
                std::fs::read(cree).unwrap().starts_with(debut.as_bytes()),
                "{cree}"
            );
        }
        assert_eq!(en_pdf.crees.len() + en_word.crees.len(), 3);
    }
}
