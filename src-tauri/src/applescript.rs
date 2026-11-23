//! Scripts AppleScript (macOS) : Mail et Microsoft Office se pilotent par des événements Apple,
//! comme Outlook et Office par PowerShell sous Windows (`powershell`). Le script, fixe, est écrit
//! dans un dossier temporaire et lancé par `osascript` ; ses données passent par un fichier, jamais
//! par la ligne de commande ; il rend compte par des lignes JSON, une par élément.

use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::erreur::Erreur;
pub use crate::pilotage::Fin;

/// Ce qu'écrit `osascript` quand l'application pilotée n'a pas répondu à temps (« AppleEvent timed
/// out ») : une fenêtre y attend une réponse. Le script s'arrête alors, comme arrêté par le chien
/// de garde.
const DELAI_DEPASSE: &str = "(-1712)";

/// Le dossier de travail (script, données) est effacé quoi qu'il arrive.
struct Travail(PathBuf);

impl Drop for Travail {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Lance `script` avec le fichier de données `donnees` (premier argument du script) ; chaque ligne
/// écrite par le script est passée à `ligne`, qui renvoie `false` pour cesser de lire. Si le script
/// ne dit plus rien pendant `silence_max`, il est arrêté (`Fin::arrete`) ; les applications qu'il
/// pilotait, lancées hors de lui, restent à arrêter.
pub fn lancer(
    script: &str,
    donnees: &str,
    silence_max: Duration,
    mut ligne: impl FnMut(&str) -> bool,
) -> Result<Fin, Erreur> {
    static COMPTEUR: AtomicU32 = AtomicU32::new(0);

    let echec =
        |contexte: &str, erreur: std::io::Error| Erreur::simple(format!("{contexte} : {erreur}"));
    let travail = Travail(std::env::temp_dir().join(format!(
        "automat-pdf-{}-{}",
        std::process::id(),
        COMPTEUR.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(&travail.0)
        .map_err(|e| echec("Dossier de travail impossible à créer", e))?;
    let fichier_script = travail.0.join("script.applescript");
    let fichier_donnees = travail.0.join("donnees.json");
    std::fs::write(&fichier_script, script).map_err(|e| echec("Script impossible à écrire", e))?;
    std::fs::write(&fichier_donnees, donnees)
        .map_err(|e| echec("Données impossibles à écrire", e))?;

    let mut processus = Command::new("/usr/bin/osascript")
        .arg(&fichier_script)
        .arg(&fichier_donnees)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| echec("AppleScript n'a pas pu être lancé", e))?;
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
                    let _ = Command::new("/bin/kill")
                        .args(["-9", &identifiant.to_string()])
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
        .map_err(|e| echec("AppleScript s'est arrêté", e))?;
    fini.store(true, Ordering::Relaxed);
    let _ = garde.join();
    let erreurs = erreurs.and_then(|fil| fil.join().ok()).unwrap_or_default();
    Ok(Fin {
        succes: etat.success(),
        arrete: arrete.load(Ordering::Relaxed) || erreurs.contains(DELAI_DEPASSE),
        erreurs: lignes_d_erreur(&erreurs),
    })
}

/// Premières lignes des erreurs d'`osascript`, sans le chemin du script (« script.applescript:
/// 812:930: execution error: … ») : seule l'explication intéresse l'utilisateur.
fn lignes_d_erreur(erreurs: &str) -> Vec<String> {
    erreurs
        .lines()
        .map(|ligne| {
            ligne
                .split_once("execution error: ")
                .map_or(ligne, |(_, explication)| explication)
                .trim()
        })
        .filter(|ligne| !ligne.is_empty())
        .take(5)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rend_compte_ligne_par_ligne_avec_les_donnees_du_fichier() {
        // Script et données en UTF-8, chemins accentués : rien ne passe par la ligne de commande.
        let script = r#"use AppleScript version "2.5"
use scripting additions
use framework "Foundation"
on run argv
	set donnees to current application's NSData's dataWithContentsOfFile:(item 1 of argv)
	set texte to (current application's NSString's alloc()'s initWithData:donnees encoding:(current application's NSUTF8StringEncoding)) as text
	set sortie to current application's NSFileHandle's fileHandleWithStandardOutput()
	repeat with ligne in {"{\"rang\":0}", texte}
		set octets to ((current application's NSString's stringWithString:((ligne as text) & linefeed))'s dataUsingEncoding:(current application's NSUTF8StringEncoding))
		sortie's writeData:octets
	end repeat
end run
"#;
        let mut lues = Vec::new();
        let fin = lancer(script, "Aïssatou été", Duration::from_secs(60), |ligne| {
            lues.push(ligne.to_string());
            true
        })
        .unwrap();
        assert!(fin.succes, "{:?}", fin.erreurs);
        assert!(!fin.arrete);
        assert_eq!(lues, [r#"{"rang":0}"#, "Aïssatou été"]);
    }

    #[test]
    fn arrete_un_script_qui_ne_dit_plus_rien() {
        let debut = Instant::now();
        let fin = lancer("delay 30", "{}", Duration::from_secs(1), |_| true).unwrap();
        assert!(fin.arrete);
        assert!(!fin.succes);
        assert!(debut.elapsed() < Duration::from_secs(20));
    }

    #[test]
    fn garde_l_explication_des_erreurs() {
        let fin = lancer(
            r#"error "Fichier protégé" number 5"#,
            "{}",
            Duration::from_secs(60),
            |_| true,
        )
        .unwrap();
        assert!(!fin.succes);
        assert!(!fin.arrete);
        assert_eq!(fin.erreurs, ["Fichier protégé (5)"]);
        assert_eq!(
            lignes_d_erreur(
                "/tmp/a/script.applescript:12:40: execution error: Microsoft Word got an error: AppleEvent timed out. (-1712)\n"
            ),
            ["Microsoft Word got an error: AppleEvent timed out. (-1712)"]
        );
    }
}
