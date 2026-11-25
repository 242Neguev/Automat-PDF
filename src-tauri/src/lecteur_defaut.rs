//! Automat-PDF, lecteur de PDF par défaut : savoir si c'est lui qui ouvre les PDF, et proposer de
//! le devenir. Le système demande toujours l'accord de l'utilisateur : Windows ne laisse aucune
//! application se déclarer d'office (elle ouvre sa fenêtre « Comment voulez-vous ouvrir les
//! fichiers .pdf ? », ou ses Paramètres) ; macOS demande de confirmer le changement.
//!
//! Windows : l'installateur inscrit Automat-PDF parmi les applications qui ouvrent les PDF
//! (`windows/clic-droit.nsh` : ProgID `AutomatPDF.Document`, « Ouvrir avec »,
//! RegisteredApplications). Mac : l'Info.plist déclare les PDF, au rang « Alternate » (jamais
//! choisi d'office).

use serde::Serialize;
use tauri::{AppHandle, WebviewWindow};

use crate::erreur::Erreur;

/// Où en est Automat-PDF comme lecteur de PDF du système.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtatLecteur {
    /// Le système permet de le proposer : Windows, avec Automat-PDF installé ; macOS 12 et plus.
    pub possible: bool,
    /// C'est déjà Automat-PDF qui ouvre les PDF.
    pub par_defaut: bool,
}

const IMPOSSIBLE: EtatLecteur = EtatLecteur {
    possible: false,
    par_defaut: false,
};

/// Ce que la demande a donné : l'état après coup ; `parametres` : la fenêtre de choix n'a pas pu
/// s'ouvrir, ce sont les Paramètres de Windows qui le sont (le choix s'y fait, plus tard).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Demande {
    pub etat: EtatLecteur,
    pub parametres: bool,
}

/// Deux chemins du même programme ? Windows ne tient compte ni des majuscules ni du sens des barres,
/// et peut écrire le chemin long (`\\?\C:\…`).
#[cfg(any(windows, test))]
fn meme_programme(a: &str, b: &str) -> bool {
    let normal = |chemin: &str| {
        chemin
            .trim()
            .trim_matches('"')
            .trim_start_matches(r"\\?\")
            .replace('/', "\\")
            .to_lowercase()
    };
    !a.trim().is_empty() && normal(a) == normal(b)
}

/// L'état, lu depuis le fil principal (les fonctions du système y sont chez elles).
pub fn etat(app: &AppHandle) -> EtatLecteur {
    let (envoi, reception) = std::sync::mpsc::channel();
    if app
        .run_on_main_thread(move || {
            let _ = envoi.send(systeme::etat());
        })
        .is_err()
    {
        return IMPOSSIBLE;
    }
    reception.recv().unwrap_or(IMPOSSIBLE)
}

/// Propose de faire d'Automat-PDF le lecteur des PDF, par la fenêtre du système.
pub fn devenir(app: &AppHandle, fenetre: &WebviewWindow) -> Result<Demande, Erreur> {
    let parametres = systeme::devenir(app, fenetre)?;
    Ok(Demande {
        etat: etat(app),
        parametres,
    })
}

#[cfg(windows)]
mod systeme {
    use std::os::windows::process::CommandExt;

    use tauri::{AppHandle, WebviewWindow};
    use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HWND};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CLASSES_ROOT, KEY_READ, RegCloseKey, RegOpenKeyExW,
    };
    use windows_sys::Win32::UI::Shell::{
        ASSOCF_INIT_IGNOREUNKNOWN, ASSOCSTR_EXECUTABLE, AssocQueryStringW, OAIF_ALLOW_REGISTRATION,
        OAIF_FORCE_REGISTRATION, OAIF_REGISTER_EXT, OPENASINFO, SHOpenWithDialog,
    };

    use super::{EtatLecteur, meme_programme};
    use crate::erreur::Erreur;

    /// La classe des PDF qu'inscrit l'installateur (`windows/clic-droit.nsh`).
    const CLASSE: &str = "AutomatPDF.Document";
    /// La page « Applications par défaut » des Paramètres, ouverte sur Automat-PDF (son nom dans
    /// RegisteredApplications) ; Windows 10 ouvre la page générale.
    const PARAMETRES: &str = "ms-settings:defaultapps?registeredAppUser=Automat-PDF";
    /// L'utilisateur a refermé la fenêtre sans choisir : HRESULT_FROM_WIN32(ERROR_CANCELLED).
    const ANNULE: i32 = 0x8007_04C7_u32 as i32;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn large(texte: &str) -> Vec<u16> {
        texte.encode_utf16().chain(Some(0)).collect()
    }

    /// Automat-PDF est-il inscrit parmi les applications qui ouvrent les PDF (version installée) ?
    fn inscrit() -> bool {
        let cle = large(CLASSE);
        let mut ouverte: HKEY = std::ptr::null_mut();
        // SAFETY : nom terminé par un zéro ; la clé ouverte est refermée.
        unsafe {
            if RegOpenKeyExW(HKEY_CLASSES_ROOT, cle.as_ptr(), 0, KEY_READ, &mut ouverte)
                != ERROR_SUCCESS
            {
                return false;
            }
            RegCloseKey(ouverte);
        }
        true
    }

    /// Le programme qui ouvre les PDF d'un double-clic, d'après Windows (choix de l'utilisateur compris).
    fn programme_des_pdf() -> Option<String> {
        let (extension, verbe) = (large(".pdf"), large("open"));
        let mut taille = 0u32;
        // SAFETY : chaînes terminées par un zéro ; sans tampon, Windows donne la taille nécessaire.
        unsafe {
            AssocQueryStringW(
                ASSOCF_INIT_IGNOREUNKNOWN,
                ASSOCSTR_EXECUTABLE,
                extension.as_ptr(),
                verbe.as_ptr(),
                std::ptr::null_mut(),
                &mut taille,
            );
        }
        if taille == 0 {
            return None;
        }
        let mut tampon = vec![0u16; taille as usize];
        // SAFETY : tampon de la taille annoncée.
        let resultat = unsafe {
            AssocQueryStringW(
                ASSOCF_INIT_IGNOREUNKNOWN,
                ASSOCSTR_EXECUTABLE,
                extension.as_ptr(),
                verbe.as_ptr(),
                tampon.as_mut_ptr(),
                &mut taille,
            )
        };
        if resultat < 0 {
            return None;
        }
        let fin = tampon.iter().position(|&c| c == 0).unwrap_or(tampon.len());
        Some(String::from_utf16_lossy(&tampon[..fin]))
    }

    pub fn etat() -> EtatLecteur {
        let nous = std::env::current_exe().ok();
        EtatLecteur {
            possible: inscrit(),
            par_defaut: programme_des_pdf()
                .zip(nous)
                .is_some_and(|(programme, nous)| {
                    meme_programme(&programme, &nous.to_string_lossy())
                }),
        }
    }

    /// La fenêtre de Windows « Comment voulez-vous ouvrir les fichiers .pdf ? », au-dessus de celle
    /// d'Automat-PDF ; si elle ne peut pas s'ouvrir, les Paramètres (renvoie `true`).
    pub fn devenir(app: &AppHandle, fenetre: &WebviewWindow) -> Result<bool, Erreur> {
        let hwnd = fenetre.hwnd().map(|h| h.0 as usize).unwrap_or(0);
        let (envoi, reception) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let fichier = large(".pdf");
            let infos = OPENASINFO {
                pcszFile: fichier.as_ptr(),
                pcszClass: std::ptr::null(),
                oaifInFlags: OAIF_ALLOW_REGISTRATION | OAIF_REGISTER_EXT | OAIF_FORCE_REGISTRATION,
            };
            // SAFETY : structure valide le temps de l'appel ; fenêtre de l'application ou aucune.
            let _ = envoi.send(unsafe { SHOpenWithDialog(hwnd as HWND, &infos) });
        })
        .map_err(|erreur| {
            Erreur::simple(format!(
                "La fenêtre de choix de Windows n'a pas pu s'ouvrir : {erreur}"
            ))
        })?;
        let resultat = reception.recv().unwrap_or(-1);
        if resultat >= 0 || resultat == ANNULE {
            return Ok(false);
        }
        std::process::Command::new("explorer.exe")
            .arg(PARAMETRES)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|erreur| {
                Erreur::simple(format!(
                    "Les Paramètres de Windows n'ont pas pu s'ouvrir : {erreur}"
                ))
            })?;
        Ok(true)
    }
}

#[cfg(target_os = "macos")]
mod systeme {
    use std::time::Duration;

    use block2::RcBlock;
    use objc2::available;
    use objc2::rc::Retained;
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSBundle, NSError, NSURL, ns_string};
    use objc2_uniform_type_identifiers::UTType;
    use tauri::{AppHandle, WebviewWindow};

    use super::{EtatLecteur, IMPOSSIBLE};
    use crate::erreur::Erreur;

    fn type_pdf() -> Option<Retained<UTType>> {
        UTType::typeWithIdentifier(ns_string!("com.adobe.pdf"))
    }

    /// L'application elle-même (un paquet `.app`) ; rien quand elle tourne hors de son paquet (essais).
    fn application() -> Option<Retained<NSURL>> {
        let adresse = NSBundle::mainBundle().bundleURL();
        let chemin = adresse.path()?.to_string();
        chemin.ends_with(".app").then_some(adresse)
    }

    fn chemin(adresse: &NSURL) -> Option<String> {
        adresse
            .path()
            .map(|chemin| chemin.to_string().trim_end_matches('/').to_string())
    }

    /// L'application qui ouvre les PDF d'un double-clic (son chemin), d'après macOS 12 et plus.
    pub(super) fn lecteur_des_pdf() -> Option<String> {
        if !available!(macos = 12.0) {
            return None;
        }
        let pdf = type_pdf()?;
        NSWorkspace::sharedWorkspace()
            .URLForApplicationToOpenContentType(&pdf)
            .and_then(|adresse| chemin(&adresse))
    }

    pub fn etat() -> EtatLecteur {
        if !available!(macos = 12.0) {
            return IMPOSSIBLE;
        }
        let Some(nous) = application().and_then(|nous| chemin(&nous)) else {
            return IMPOSSIBLE;
        };
        EtatLecteur {
            possible: true,
            par_defaut: lecteur_des_pdf().is_some_and(|actuel| actuel == nous),
        }
    }

    /// macOS demande à l'utilisateur de confirmer (« Utiliser Automat-PDF ») ; on attend sa réponse.
    pub fn devenir(app: &AppHandle, _fenetre: &WebviewWindow) -> Result<bool, Erreur> {
        if !available!(macos = 12.0) {
            return Err(Erreur::simple(
                "Choisissez Automat-PDF dans le Finder : « Lire les informations » d'un PDF, « Ouvrir avec », puis « Tout modifier ».",
            ));
        }
        let (envoi, reception) = std::sync::mpsc::channel::<bool>();
        app.run_on_main_thread(move || {
            let (Some(nous), Some(pdf)) = (application(), type_pdf()) else {
                let _ = envoi.send(false);
                return;
            };
            let fin = RcBlock::new(move |erreur: *mut NSError| {
                let _ = envoi.send(erreur.is_null());
            });
            NSWorkspace::sharedWorkspace()
                .setDefaultApplicationAtURL_toOpenContentType_completionHandler(
                    &nous,
                    &pdf,
                    Some(&fin),
                );
        })
        .map_err(|erreur| {
            Erreur::simple(format!(
                "macOS n'a pas pu changer le lecteur de PDF : {erreur}"
            ))
        })?;
        // La réponse de l'utilisateur ; sans réponse au bout de dix minutes, on n'attend plus.
        let _ = reception.recv_timeout(Duration::from_secs(600));
        Ok(false)
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod systeme {
    use tauri::{AppHandle, WebviewWindow};

    use super::{EtatLecteur, IMPOSSIBLE};
    use crate::erreur::Erreur;

    pub fn etat() -> EtatLecteur {
        IMPOSSIBLE
    }

    pub fn devenir(_app: &AppHandle, _fenetre: &WebviewWindow) -> Result<bool, Erreur> {
        Err(Erreur::simple(
            "Ce système ne permet pas de choisir le lecteur de PDF depuis Automat-PDF.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_le_meme_programme_ecrit_autrement() {
        let nous = r"C:\Users\Awa\AppData\Local\Automat-PDF\Automat-PDF.exe";
        assert!(meme_programme(
            r"c:\users\awa\appdata\local\automat-pdf\AUTOMAT-PDF.EXE",
            nous
        ));
        assert!(meme_programme(
            r"\\?\C:\Users\Awa\AppData\Local\Automat-PDF\Automat-PDF.exe",
            nous
        ));
        assert!(meme_programme(
            "\"C:/Users/Awa/AppData/Local/Automat-PDF/Automat-PDF.exe\"",
            nous
        ));
        assert!(!meme_programme(
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            nous
        ));
        assert!(!meme_programme("", ""));
    }

    /// Lecture seule : l'application qui ouvre les PDF sur ce Mac (Aperçu, d'ordinaire).
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "dépend du Mac qui fait l'essai"]
    fn lit_le_lecteur_des_pdf_du_mac() {
        let lecteur = systeme::lecteur_des_pdf().expect("une application ouvre les PDF");
        println!("Les PDF s'ouvrent avec : {lecteur}");
        assert!(lecteur.ends_with(".app"));
    }

    #[test]
    fn dit_a_l_interface_ce_qu_il_en_est() {
        let demande = Demande {
            etat: EtatLecteur {
                possible: true,
                par_defaut: false,
            },
            parametres: true,
        };
        assert_eq!(
            serde_json::to_value(demande).unwrap(),
            serde_json::json!({ "etat": { "possible": true, "parDefaut": false }, "parametres": true })
        );
    }
}
