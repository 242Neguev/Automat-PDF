//! LibreOffice, qui convertit les documents Office en PDF quand Microsoft Office manque ou ne
//! répond pas : trouvé là où il est installé, ou installé par son installateur officiel (winget,
//! sous Windows) ; à défaut, sa page de téléchargement est ouverte.

use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;

use crate::erreur::Erreur;

/// Paquet officiel de LibreOffice pour winget, publié par The Document Foundation.
const PAQUET: &str = "TheDocumentFoundation.LibreOffice";
/// Page de téléchargement officielle.
const PAGE: &str = "https://www.libreoffice.org/download/download-libreoffice/";
/// Codes de retour de winget qui ne sont pas des échecs : déjà installé, aucune mise à jour,
/// redémarrage demandé.
const CODES_ACCEPTES: [i32; 4] = [0, -1978335189, -1978335135, -1978334967];
/// winget ne trouve pas le paquet.
const PAQUET_INTROUVABLE: i32 = -1978335212;
/// L'accord de l'administrateur a été refusé (ERROR_CANCELLED de Windows).
const REFUSE: i32 = 1223;
/// Programme introuvable (ERROR_FILE_NOT_FOUND) : winget manque.
const INTROUVABLE: i32 = 2;

/// Ce qui permet de convertir les documents Office sur ce PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Convertisseurs {
    /// Microsoft Word est inscrit (il peut toutefois ne pas répondre) : PDF en Word.
    pub word: bool,
    /// Word, Excel ou PowerPoint est inscrit.
    pub office: bool,
    pub libreoffice: bool,
    /// LibreOffice peut être installé par l'application (winget est là).
    pub installable: bool,
}

/// Commande lancée sans fenêtre de console.
fn commande(programme: &str) -> Command {
    #[allow(unused_mut)]
    let mut commande = Command::new(programme);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const SANS_FENETRE: u32 = 0x0800_0000;
        commande.creation_flags(SANS_FENETRE);
    }
    commande
}

/// La valeur lue par `reg query` : ce qui suit son type, quelle que soit la langue de Windows.
fn valeur_du_registre(sortie: &str) -> Option<PathBuf> {
    sortie.lines().find_map(|ligne| {
        let (_, valeur) = ligne
            .split_once("REG_SZ")
            .or_else(|| ligne.split_once("REG_EXPAND_SZ"))?;
        let valeur = valeur.trim();
        (!valeur.is_empty()).then(|| PathBuf::from(valeur))
    })
}

/// Clé du registre existe-t-elle ? `vue` : `/reg:64` ou `/reg:32`.
fn lire_registre(cle: &str, vue: &str) -> Option<String> {
    let sortie = commande("reg")
        .args(["query", cle, "/ve", vue])
        .output()
        .ok()?;
    sortie
        .status
        .success()
        .then(|| String::from_utf8_lossy(&sortie.stdout).into_owned())
}

/// Emplacements où chercher LibreOffice, du plus sûr au moins sûr : celui que son installateur a
/// inscrit dans le registre, puis les dossiers des programmes.
fn candidats() -> Vec<PathBuf> {
    if cfg!(windows) {
        let inscrits = ["/reg:64", "/reg:32"].iter().filter_map(|vue| {
            lire_registre(r"HKLM\SOFTWARE\LibreOffice\UNO\InstallPath", vue)
                .and_then(|sortie| valeur_du_registre(&sortie))
                .map(|programme| programme.join("soffice.exe"))
        });
        let dossiers = ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"]
            .iter()
            .filter_map(std::env::var_os)
            .map(PathBuf::from)
            .chain([r"C:\Program Files".into(), r"C:\Program Files (x86)".into()])
            .map(|dossier| {
                dossier
                    .join("LibreOffice")
                    .join("program")
                    .join("soffice.exe")
            });
        inscrits.chain(dossiers).collect()
    } else if cfg!(target_os = "macos") {
        vec!["/Applications/LibreOffice.app/Contents/MacOS/soffice".into()]
    } else {
        vec!["/usr/bin/soffice".into(), "/usr/local/bin/soffice".into()]
    }
}

/// LibreOffice installé sur ce PC, s'il l'est.
pub fn trouver() -> Option<PathBuf> {
    candidats().into_iter().find(|chemin| chemin.is_file())
}

/// L'application d'Office (Word, Excel ou PowerPoint) est inscrite sur ce PC.
fn inscrite(application: &str) -> bool {
    cfg!(windows)
        && lire_registre(
            &format!(r"HKCR\{application}.Application\CurVer"),
            "/reg:64",
        )
        .is_some()
}

fn winget_disponible() -> bool {
    cfg!(windows)
        && commande("winget")
            .arg("--version")
            .output()
            .is_ok_and(|sortie| sortie.status.success())
}

/// Ce qui permet de convertir les documents Office sur ce PC.
pub fn convertisseurs() -> Convertisseurs {
    let libreoffice = trouver().is_some();
    let word = inscrite("Word");
    Convertisseurs {
        word,
        office: word || inscrite("Excel") || inscrite("PowerPoint"),
        libreoffice,
        installable: !libreoffice && winget_disponible(),
    }
}

/// Explication d'un échec de l'installation, d'après le code de retour de winget (ou de Windows).
fn explication(code: i32) -> String {
    match code {
        REFUSE => "L'accord de l'administrateur a été refusé : LibreOffice n'a pas été installé.".into(),
        INTROUVABLE => "winget, l'installateur de Windows, est introuvable : téléchargez LibreOffice depuis sa page.".into(),
        PAQUET_INTROUVABLE => "Le paquet de LibreOffice est introuvable : téléchargez LibreOffice depuis sa page.".into(),
        _ if CODES_ACCEPTES.contains(&code) => "LibreOffice semble installé, mais reste introuvable : fermez puis rouvrez Automat-PDF.".into(),
        _ => format!(
            "L'installation de LibreOffice n'a pas abouti (code {code:#x}) : vérifiez la connexion à internet, puis réessayez ; vous pouvez aussi le télécharger depuis sa page."
        ),
    }
}

/// Commande PowerShell qui lance winget avec les droits d'administrateur, après l'accord demandé
/// par Windows, attend sa fin et rend son code de retour ; un refus rend le code d'erreur de
/// Windows (1223). Lancé sans ces droits, le programme d'installation de LibreOffice, prévu pour
/// tout l'ordinateur, échoue (erreur 1730) : en mode silencieux, il ne demande pas l'accord.
fn script_d_installation() -> String {
    let arguments = format!(
        "install --id {PAQUET} --exact --source winget --silent --accept-package-agreements --accept-source-agreements --disable-interactivity"
    );
    format!(
        "try {{ $p = Start-Process -FilePath 'winget' -ArgumentList '{arguments}' -Verb RunAs -Wait -PassThru -WindowStyle Hidden; exit $p.ExitCode }} catch {{ $e = $_.Exception; while ($e -and -not ($e -is [System.ComponentModel.Win32Exception])) {{ $e = $e.InnerException }}; if ($e) {{ exit $e.NativeErrorCode }} else {{ exit 1 }} }}"
    )
}

/// Installe LibreOffice par son installateur officiel (winget) : Windows demande d'abord l'accord
/// d'un administrateur, puis environ 350 Mo sont téléchargés et installés. Rend l'emplacement de
/// LibreOffice installé.
pub fn installer() -> Result<PathBuf, Erreur> {
    if !cfg!(windows) {
        return Err(Erreur::simple(
            "LibreOffice ne s'installe d'ici que sous Windows : téléchargez-le depuis sa page.",
        ));
    }
    let sortie = commande("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script_d_installation(),
        ])
        .output()
        .map_err(|erreur| {
            Erreur::simple(format!(
                "L'installation de LibreOffice n'a pas pu être lancée ({erreur}) : téléchargez-le depuis sa page."
            ))
        })?;
    let code = sortie.status.code().unwrap_or(-1);
    // Le code de winget ne suffit pas : il annonce parfois un échec quand Windows Installer a
    // réussi. Ce qui compte, c'est LibreOffice trouvé à la fin ; le code explique un échec.
    trouver().ok_or_else(|| Erreur::simple(explication(code)))
}

/// Ouvre la page de téléchargement officielle de LibreOffice dans le navigateur.
pub fn ouvrir_page() -> Result<(), Erreur> {
    let ouverture = if cfg!(windows) {
        commande("explorer").arg(PAGE).spawn()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(PAGE).spawn()
    } else {
        Command::new("xdg-open").arg(PAGE).spawn()
    };
    ouverture.map(|_| ()).map_err(|erreur| {
        Erreur::simple(format!(
            "La page de LibreOffice n'a pas pu être ouverte ({erreur}) : allez sur {PAGE}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_l_emplacement_inscrit_dans_le_registre() {
        let francais = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\LibreOffice\\UNO\\InstallPath\r\n    (par défaut)    REG_SZ    C:\\Program Files\\LibreOffice\\program\r\n\r\n";
        assert_eq!(
            valeur_du_registre(francais),
            Some(PathBuf::from(r"C:\Program Files\LibreOffice\program"))
        );
        let anglais = "HKEY_LOCAL_MACHINE\\SOFTWARE\\LibreOffice\\UNO\\InstallPath\n    (Default)    REG_EXPAND_SZ    D:\\Outils\\LibreOffice\\program\n";
        assert_eq!(
            valeur_du_registre(anglais),
            Some(PathBuf::from(r"D:\Outils\LibreOffice\program"))
        );
        assert_eq!(valeur_du_registre("    (par défaut)    REG_SZ    \n"), None);
        assert_eq!(valeur_du_registre(""), None);
    }

    #[test]
    fn lance_winget_avec_les_droits_d_administrateur() {
        let script = script_d_installation();
        assert!(script.contains("-Verb RunAs -Wait -PassThru"), "{script}");
        assert!(script.contains("exit $p.ExitCode"));
        // Les arguments de winget, entre apostrophes, n'en contiennent aucune : rien ne s'en échappe.
        let (_, reste) = script.split_once("-ArgumentList '").unwrap();
        let (arguments, _) = reste.split_once('\'').unwrap();
        assert_eq!(
            arguments,
            "install --id TheDocumentFoundation.LibreOffice --exact --source winget --silent --accept-package-agreements --accept-source-agreements --disable-interactivity"
        );
        assert!(script.is_ascii());
    }

    #[test]
    fn explique_un_echec_de_l_installation() {
        assert_eq!(
            explication(REFUSE),
            "L'accord de l'administrateur a été refusé : LibreOffice n'a pas été installé."
        );
        assert!(
            explication(INTROUVABLE)
                .starts_with("winget, l'installateur de Windows, est introuvable")
        );
        assert!(explication(-1978335226).contains("(code 0x8a150006)"));
        assert!(
            explication(PAQUET_INTROUVABLE).starts_with("Le paquet de LibreOffice est introuvable")
        );
        assert!(explication(0).contains("rouvrez Automat-PDF"));
    }

    #[test]
    fn cherche_dans_les_dossiers_des_programmes() {
        let candidats = candidats();
        if cfg!(windows) {
            assert!(candidats.contains(&PathBuf::from(
                r"C:\Program Files\LibreOffice\program\soffice.exe"
            )));
        } else if cfg!(target_os = "macos") {
            assert_eq!(
                candidats,
                [PathBuf::from(
                    "/Applications/LibreOffice.app/Contents/MacOS/soffice"
                )]
            );
        }
    }
}
