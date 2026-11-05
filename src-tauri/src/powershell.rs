//! Scripts PowerShell (Windows) : Outlook et Office se pilotent par automatisation COM. Le script,
//! fixe, est écrit dans un dossier temporaire et lancé sans fenêtre ; ses données passent par un
//! fichier, jamais par la ligne de commande ; il rend compte par des lignes JSON, une par élément.

use std::time::Duration;

use crate::erreur::Erreur;

/// Message pour un script arrêté parce qu'il ne disait plus rien.
pub const NE_REPOND_PLUS: &str = "L'application pilotée ne répond plus (une fenêtre attend peut-être une réponse, ou un mot de passe) : le traitement a été arrêté.";

/// Ce que le script a laissé en sortant.
#[derive(Debug)]
pub struct Fin {
    pub succes: bool,
    /// Arrêté par le chien de garde : il ne disait plus rien depuis trop longtemps.
    pub arrete: bool,
    /// Premières lignes de ses erreurs.
    pub erreurs: Vec<String>,
}

/// Lance `script` avec le fichier de données `donnees` (paramètre `-Donnees`) ; chaque ligne écrite
/// par le script est passée à `ligne`, qui renvoie `false` pour cesser de lire. Si le script ne dit
/// plus rien pendant `silence_max` (une fenêtre attend une réponse, un mot de passe…), il est arrêté
/// (`Fin::arrete`) ; les applications qu'il pilotait, lancées hors de lui, restent à arrêter.
#[cfg(windows)]
pub fn lancer(
    script: &str,
    donnees: &str,
    silence_max: Duration,
    mut ligne: impl FnMut(&str) -> bool,
) -> Result<Fin, Erreur> {
    use std::io::{BufRead, BufReader, Read};
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    /// Le dossier de travail (script, données) est effacé quoi qu'il arrive.
    struct Travail(std::path::PathBuf);
    impl Drop for Travail {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    const SANS_FENETRE: u32 = 0x0800_0000;
    static COMPTEUR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    let echec =
        |contexte: &str, erreur: std::io::Error| Erreur::simple(format!("{contexte} : {erreur}"));
    let travail = Travail(std::env::temp_dir().join(format!(
        "automat-pdf-{}-{}",
        std::process::id(),
        COMPTEUR.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(&travail.0)
        .map_err(|e| echec("Dossier de travail impossible à créer", e))?;
    let fichier_script = travail.0.join("script.ps1");
    let fichier_donnees = travail.0.join("donnees.json");
    // Marque UTF-8 en tête : sans elle, PowerShell 5.1 lirait le script dans le codage de Windows.
    std::fs::write(&fichier_script, format!("\u{feff}{script}"))
        .map_err(|e| echec("Script impossible à écrire", e))?;
    std::fs::write(&fichier_donnees, donnees)
        .map_err(|e| echec("Données impossibles à écrire", e))?;

    let mut processus = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Sta",
            "-File",
        ])
        .arg(&fichier_script)
        .arg("-Donnees")
        .arg(&fichier_donnees)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(SANS_FENETRE)
        .spawn()
        .map_err(|e| echec("PowerShell n'a pas pu être lancé", e))?;
    let identifiant = processus.id();

    // Les erreurs sont lues à part : un flux plein ne doit jamais bloquer l'autre.
    let erreurs = processus.stderr.take().map(|mut flux| {
        std::thread::spawn(move || {
            let mut texte = Vec::new();
            let _ = flux.read_to_end(&mut texte);
            String::from_utf8_lossy(&texte).into_owned()
        })
    });
    // Chien de garde : sans nouvelles depuis trop longtemps, le script est arrêté.
    let derniere_nouvelle = Arc::new(Mutex::new(Instant::now()));
    let fini = Arc::new(AtomicBool::new(false));
    let arrete = Arc::new(AtomicBool::new(false));
    let garde = {
        let (derniere_nouvelle, fini, arrete) =
            (derniere_nouvelle.clone(), fini.clone(), arrete.clone());
        std::thread::spawn(move || {
            while !fini.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(500));
                let silence = derniere_nouvelle
                    .lock()
                    .map(|instant| instant.elapsed())
                    .unwrap_or_default();
                if silence > silence_max && !fini.load(Ordering::Relaxed) {
                    arrete.store(true, Ordering::Relaxed);
                    let _ = Command::new("taskkill")
                        .args(["/PID", &identifiant.to_string(), "/T", "/F"])
                        .creation_flags(SANS_FENETRE)
                        .output();
                    return;
                }
            }
        })
    };
    if let Some(sortie) = processus.stdout.take() {
        // Décodage sans échec possible : un caractère mal codé n'interrompt pas le compte rendu.
        for octets in BufReader::new(sortie).split(b'\n').map_while(Result::ok) {
            if let Ok(mut instant) = derniere_nouvelle.lock() {
                *instant = Instant::now();
            }
            if !ligne(String::from_utf8_lossy(&octets).trim()) {
                break;
            }
        }
    }
    let etat = processus
        .wait()
        .map_err(|e| echec("PowerShell s'est arrêté", e))?;
    fini.store(true, Ordering::Relaxed);
    let _ = garde.join();
    let erreurs = erreurs.and_then(|fil| fil.join().ok()).unwrap_or_default();
    Ok(Fin {
        succes: etat.success(),
        arrete: arrete.load(Ordering::Relaxed),
        erreurs: erreurs
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(5)
            .map(str::to_string)
            .collect(),
    })
}

/// Hors de Windows, ni Outlook ni Office ne se pilotent.
#[cfg(not(windows))]
pub fn lancer(_: &str, _: &str, _: Duration, _: impl FnMut(&str) -> bool) -> Result<Fin, Erreur> {
    Err(Erreur::simple(
        "Cette fonction n'est disponible que sous Windows.",
    ))
}
