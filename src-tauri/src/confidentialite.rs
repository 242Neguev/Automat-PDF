//! Réglages de confidentialité et statistiques anonymes d'utilisation.
//!
//! En dehors des travaux demandés par l'utilisateur, Automat-PDF ne se connecte à internet que
//! pour deux choses, réglables dans « À propos » :
//! - la recherche des mises à jour (greffon updater de Tauri, mené par l'interface), activée
//!   d'emblée : elle ne fait que lire la page des versions publiées sur GitHub ;
//! - les statistiques anonymes, envoyées seulement après un « Oui » à la question posée au premier
//!   lancement : un signal à chaque ouverture, avec un numéro tiré au hasard pour l'installation,
//!   la version d'Automat-PDF, celle du système, la langue et l'édition. Jamais un fichier, un nom
//!   ni une adresse.
//!
//! L'adresse qui reçoit les statistiques est fixée à la compilation (`AUTOMAT_PDF_STATISTIQUES`,
//! secret de GitHub Actions). Sans elle, comme dans les copies compilées par d'autres, la question
//! n'est pas posée et rien n'est envoyé.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::erreur::Erreur;

/// Adresse qui reçoit les statistiques, si cette copie d'Automat-PDF en a une.
pub fn adresse() -> Option<&'static str> {
    option_env!("AUTOMAT_PDF_STATISTIQUES")
        .map(str::trim)
        .filter(|adresse| adresse_valable(adresse))
}

/// Seule une adresse chiffrée est acceptée ; en développement, aussi un serveur sur ce PC.
fn adresse_valable(adresse: &str) -> bool {
    adresse.starts_with("https://")
        || (cfg!(debug_assertions) && adresse.starts_with("http://127.0.0.1:"))
}

/// Édition d'Automat-PDF, pour distinguer plus tard l'édition libre de l'édition payante.
const EDITION: &str = "libre";

/// Ce qui est enregistré dans le fichier des réglages de confidentialité.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Reglages {
    /// Chercher les nouvelles versions au démarrage, puis chaque jour.
    pub mises_a_jour: bool,
    /// Statistiques anonymes acceptées ; `None` tant que la question n'a pas reçu de réponse.
    pub statistiques: Option<bool>,
    /// Numéro tiré au hasard pour cette installation, au premier « Oui ».
    pub installation: Option<String>,
    /// Le premier signal (« installation ») a été reçu.
    pub installation_signalee: bool,
}

impl Default for Reglages {
    fn default() -> Self {
        Self {
            mises_a_jour: true,
            statistiques: None,
            installation: None,
            installation_signalee: false,
        }
    }
}

/// Ce que l'interface sait des réglages de confidentialité.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Confidentialite {
    pub mises_a_jour: bool,
    pub statistiques: Option<bool>,
    /// Cette copie d'Automat-PDF peut envoyer des statistiques : sinon, la question n'est pas posée.
    pub statistiques_possibles: bool,
}

/// Le signal envoyé, tel quel, en JSON.
#[derive(Debug, Serialize)]
pub struct Signal {
    pub installation: String,
    /// « installation » pour le premier signal reçu, « ouverture » ensuite.
    pub evenement: &'static str,
    pub version: &'static str,
    pub systeme: String,
    pub langue: String,
    pub edition: &'static str,
}

impl Reglages {
    /// Réglages lus dans leur fichier ; ce qui est illisible reprend sa valeur par défaut.
    pub fn lire(contenu: &str) -> Self {
        let mut reglages: Self = serde_json::from_str(contenu).unwrap_or_default();
        reglages.installation = reglages
            .installation
            .filter(|numero| numero_valable(numero));
        if reglages.installation.is_none() {
            reglages.installation_signalee = false;
        }
        reglages
    }

    /// Contenu du fichier.
    pub fn texte(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn etat(&self, statistiques_possibles: bool) -> Confidentialite {
        Confidentialite {
            mises_a_jour: self.mises_a_jour,
            statistiques: self.statistiques,
            statistiques_possibles,
        }
    }

    /// Applique les réglages donnés ; au premier « Oui », un numéro est tiré pour l'installation.
    /// Le numéro est gardé après un « Non » : en cas de nouveau « Oui », l'installation n'est pas
    /// comptée deux fois.
    pub fn regler(
        &mut self,
        mises_a_jour: Option<bool>,
        statistiques: Option<bool>,
    ) -> Result<(), Erreur> {
        if let Some(mises_a_jour) = mises_a_jour {
            self.mises_a_jour = mises_a_jour;
        }
        if let Some(statistiques) = statistiques {
            self.statistiques = Some(statistiques);
        }
        if self.statistiques == Some(true) && self.installation.is_none() {
            self.installation = Some(numero_au_hasard()?);
            self.installation_signalee = false;
        }
        Ok(())
    }

    /// Le signal à envoyer à l'ouverture, si l'utilisateur a accepté les statistiques.
    pub fn signal(&self, langue: &str, systeme: &str) -> Option<Signal> {
        if self.statistiques != Some(true) {
            return None;
        }
        Some(Signal {
            installation: self.installation.clone()?,
            evenement: if self.installation_signalee {
                "ouverture"
            } else {
                "installation"
            },
            version: env!("CARGO_PKG_VERSION"),
            systeme: nettoyer(systeme),
            langue: langue_valable(langue),
            edition: EDITION,
        })
    }
}

/// Numéro d'installation tiré au hasard (UUID version 4) : il ne dit rien de l'ordinateur ni de
/// la personne.
fn numero_au_hasard() -> Result<String, Erreur> {
    let mut octets = [0u8; 16];
    getrandom::fill(&mut octets)
        .map_err(|erreur| Erreur::simple(format!("Le tirage aléatoire a échoué : {erreur}")))?;
    octets[6] = (octets[6] & 0x0f) | 0x40;
    octets[8] = (octets[8] & 0x3f) | 0x80;
    let hexa: String = octets.iter().map(|octet| format!("{octet:02x}")).collect();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hexa[..8],
        &hexa[8..12],
        &hexa[12..16],
        &hexa[16..20],
        &hexa[20..]
    ))
}

fn numero_valable(numero: &str) -> bool {
    let groupes: Vec<&str> = numero.split('-').collect();
    groupes
        .iter()
        .map(|groupe| groupe.len())
        .eq([8, 4, 4, 4, 12])
        && groupes.iter().all(|groupe| {
            groupe
                .bytes()
                .all(|o| matches!(o, b'0'..=b'9' | b'a'..=b'f'))
        })
        && numero.as_bytes()[14] == b'4'
        && matches!(numero.as_bytes()[19], b'8' | b'9' | b'a' | b'b')
}

/// Langue de l'interface (« fr-FR », « en ») ; tout le reste devient « inconnue ».
fn langue_valable(langue: &str) -> String {
    let langue = langue.trim();
    let mut parties = langue.split('-');
    let langue_seule = parties.next().is_some_and(|code| {
        (2..=3).contains(&code.len()) && code.bytes().all(|o| o.is_ascii_alphabetic())
    });
    let precisions: Vec<&str> = parties.collect();
    let precisions_valables = precisions.len() <= 3
        && precisions.iter().all(|precision| {
            (1..=8).contains(&precision.len())
                && precision.bytes().all(|o| o.is_ascii_alphanumeric())
        });
    if langue_seule && precisions_valables {
        langue.to_string()
    } else {
        "inconnue".to_string()
    }
}

/// Description du système réduite à des caractères sans risque (60 au plus).
fn nettoyer(texte: &str) -> String {
    let propre: String = texte
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || " ().,_-".contains(*c))
        .take(60)
        .collect();
    let propre = propre.trim();
    if propre.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        propre.to_string()
    } else {
        "inconnu".to_string()
    }
}

/// Valeur d'une ligne de `reg query` (« Nom    REG_SZ    valeur »), quelle que soit la langue de
/// Windows.
fn valeur_du_registre<'a>(sortie: &'a str, nom: &str) -> Option<&'a str> {
    sortie.lines().find_map(|ligne| {
        let mut morceaux = ligne.split_whitespace();
        (morceaux.next()? == nom).then_some(())?;
        morceaux.next()?.starts_with("REG_").then_some(())?;
        morceaux.next()
    })
}

/// « Windows 11 24H2 (26100) », d'après la clé CurrentVersion du registre.
fn decrire_windows(sortie: &str) -> String {
    let compilation: Option<u32> =
        valeur_du_registre(sortie, "CurrentBuild").and_then(|valeur| valeur.parse().ok());
    let nom = match compilation {
        Some(numero) if numero >= 22000 => "Windows 11",
        Some(_) => "Windows 10",
        None => "Windows",
    };
    let mut description = nom.to_string();
    if let Some(edition) = valeur_du_registre(sortie, "DisplayVersion") {
        description.push(' ');
        description.push_str(edition);
    }
    if let Some(numero) = compilation {
        description.push_str(&format!(" ({numero})"));
    }
    nettoyer(&description)
}

/// Système de ce PC : version de Windows lue dans le registre.
#[cfg(windows)]
pub fn systeme() -> String {
    use std::os::windows::process::CommandExt;
    const SANS_FENETRE: u32 = 0x0800_0000;
    std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        ])
        .creation_flags(SANS_FENETRE)
        .output()
        .ok()
        .filter(|sortie| sortie.status.success())
        .map_or_else(
            || "Windows".to_string(),
            |sortie| decrire_windows(&String::from_utf8_lossy(&sortie.stdout)),
        )
}

#[cfg(not(windows))]
pub fn systeme() -> String {
    match std::env::consts::OS {
        "macos" => "macOS".to_string(),
        autre => nettoyer(autre),
    }
}

/// Envoie le signal. Le service qui le reçoit répond `{"ok":true}` : toute autre réponse (une page
/// de connexion de Google, par exemple, si le service est mal déployé) est un échec.
pub fn envoyer(adresse: &str, signal: &Signal) -> Result<(), Erreur> {
    let echec = |raison: String| {
        Erreur::simple(format!(
            "Les statistiques n'ont pas été envoyées : {raison}"
        ))
    };
    let corps = serde_json::to_string(signal).map_err(|erreur| echec(erreur.to_string()))?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("Automat-PDF/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|erreur| echec(erreur.to_string()))?;
    // Google Apps Script répond à l'envoi par une redirection, suivie avec une simple lecture.
    let reponse = client
        .post(adresse)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(corps)
        .send()
        .map_err(|erreur| echec(erreur.to_string()))?;
    let statut = reponse.status();
    if !statut.is_success() {
        return Err(echec(format!("réponse {statut}")));
    }
    let texte = reponse.text().map_err(|erreur| echec(erreur.to_string()))?;
    let accepte = serde_json::from_str::<serde_json::Value>(&texte)
        .is_ok_and(|lu| lu.get("ok") == Some(&serde_json::Value::Bool(true)));
    if accepte {
        Ok(())
    } else {
        Err(echec("réponse inattendue du service".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::essais::serveur_local;

    #[test]
    fn reglages_par_defaut_et_relus() {
        let neufs = Reglages::lire("");
        assert_eq!(neufs, Reglages::default());
        assert!(
            neufs.mises_a_jour,
            "les mises à jour sont cherchées d'emblée"
        );
        assert_eq!(neufs.statistiques, None, "la question n'a pas été posée");
        assert!(neufs.signal("fr-FR", "Windows 11").is_none());

        let mut reglages = neufs.clone();
        reglages.regler(Some(false), Some(true)).unwrap();
        let numero = reglages.installation.clone().unwrap();
        assert!(numero_valable(&numero), "{numero}");
        let relus = Reglages::lire(&reglages.texte());
        assert_eq!(relus, reglages);
        assert_eq!(
            relus.etat(true),
            Confidentialite {
                mises_a_jour: false,
                statistiques: Some(true),
                statistiques_possibles: true
            }
        );

        // Un « Non » garde le numéro : un nouveau « Oui » ne compte pas une installation de plus.
        reglages.installation_signalee = true;
        reglages.regler(None, Some(false)).unwrap();
        assert!(reglages.signal("fr-FR", "Windows 11").is_none());
        reglages.regler(None, Some(true)).unwrap();
        assert_eq!(reglages.installation.as_deref(), Some(numero.as_str()));
        assert!(reglages.installation_signalee);
        assert!(!reglages.mises_a_jour, "réglage laissé tel quel");
    }

    #[test]
    fn ecarte_ce_qui_est_illisible() {
        assert_eq!(Reglages::lire("{pas du json"), Reglages::default());
        assert_eq!(Reglages::lire("[]"), Reglages::default());
        let abime = Reglages::lire(
            r#"{"misesAJour":false,"statistiques":true,"installation":"=HYPERLINK(1)","installationSignalee":true}"#,
        );
        assert!(!abime.mises_a_jour);
        assert_eq!(abime.statistiques, Some(true));
        assert_eq!(
            abime.installation, None,
            "numéro modifié à la main : écarté"
        );
        assert!(!abime.installation_signalee);
        // Un nouveau numéro est tiré, et le premier signal sera une « installation ».
        let mut repare = abime.clone();
        repare.regler(None, None).unwrap();
        assert!(numero_valable(repare.installation.as_deref().unwrap()));
        assert_eq!(
            repare.signal("fr", "Windows 11").unwrap().evenement,
            "installation"
        );
    }

    #[test]
    fn tire_un_numero_d_installation_au_hasard() {
        let a = numero_au_hasard().unwrap();
        let b = numero_au_hasard().unwrap();
        assert_ne!(a, b);
        for numero in [&a, &b] {
            assert!(numero_valable(numero), "{numero}");
            assert_eq!(numero.len(), 36);
        }
        assert!(numero_valable("3f2a9c1e-7b4d-4e8f-9a0b-1c2d3e4f5a6b"));
        assert!(!numero_valable("3F2A9C1E-7B4D-4E8F-9A0B-1C2D3E4F5A6B"));
        assert!(!numero_valable("3f2a9c1e-7b4d-1e8f-9a0b-1c2d3e4f5a6b"));
        assert!(!numero_valable("3f2a9c1e7b4d4e8f9a0b1c2d3e4f5a6b"));
        assert!(!numero_valable(""));
    }

    #[test]
    fn premier_signal_puis_ouvertures() {
        let mut reglages = Reglages::default();
        reglages.regler(None, Some(true)).unwrap();
        let premier = reglages.signal("fr-FR", "Windows 11 24H2 (26100)").unwrap();
        assert_eq!(premier.evenement, "installation");
        assert_eq!(premier.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(premier.systeme, "Windows 11 24H2 (26100)");
        assert_eq!(premier.langue, "fr-FR");
        assert_eq!(premier.edition, "libre");
        let json = serde_json::to_value(&premier).unwrap();
        let mut champs: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        champs.sort_unstable();
        assert_eq!(
            champs,
            [
                "edition",
                "evenement",
                "installation",
                "langue",
                "systeme",
                "version"
            ],
            "rien d'autre n'est envoyé"
        );
        reglages.installation_signalee = true;
        assert_eq!(
            reglages.signal("fr-FR", "Windows 11").unwrap().evenement,
            "ouverture"
        );
    }

    #[test]
    fn ne_garde_que_des_valeurs_sans_risque() {
        assert_eq!(langue_valable("fr-FR"), "fr-FR");
        assert_eq!(langue_valable(" en "), "en");
        assert_eq!(langue_valable("zh-Hant-TW"), "zh-Hant-TW");
        assert_eq!(langue_valable("=cmd|' /C calc'!A0"), "inconnue");
        assert_eq!(langue_valable(""), "inconnue");
        assert_eq!(langue_valable("fr-FR-x-a-b-c"), "inconnue");
        assert_eq!(
            nettoyer("Windows 11 24H2 (26100)"),
            "Windows 11 24H2 (26100)"
        );
        assert_eq!(nettoyer("=1+1"), "11");
        assert_eq!(nettoyer("-@x"), "inconnu");
        assert_eq!(nettoyer(&"W".repeat(100)).len(), 60);
    }

    #[test]
    fn decrit_la_version_de_windows() {
        let onze = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\r\n    SystemRoot    REG_SZ    C:\\WINDOWS\r\n    CurrentBuild    REG_SZ    26100\r\n    CurrentBuildNumber    REG_SZ    26100\r\n    DisplayVersion    REG_SZ    24H2\r\n    ProductName    REG_SZ    Windows 10 Home\r\n";
        assert_eq!(decrire_windows(onze), "Windows 11 24H2 (26100)");
        let dix = "    CurrentBuild    REG_SZ    19045\n    DisplayVersion    REG_SZ    22H2\n";
        assert_eq!(decrire_windows(dix), "Windows 10 22H2 (19045)");
        assert_eq!(
            decrire_windows("    CurrentBuild    REG_SZ    17763\n"),
            "Windows 10 (17763)"
        );
        assert_eq!(decrire_windows(""), "Windows");
        if cfg!(windows) {
            assert!(systeme().starts_with("Windows"), "{}", systeme());
        }
    }

    #[test]
    fn n_accepte_qu_une_adresse_chiffree() {
        assert!(adresse_valable(
            "https://script.google.com/macros/s/AKfycb/exec"
        ));
        assert!(!adresse_valable("http://exemple.com/statistiques"));
        assert!(!adresse_valable("ftp://exemple.com"));
        assert!(!adresse_valable(""));
        // Les vérifications et les compilations de GitHub n'ont pas d'adresse : rien n'est envoyé.
        if option_env!("AUTOMAT_PDF_STATISTIQUES").is_none() {
            assert_eq!(adresse(), None);
        }
    }

    #[test]
    fn envoie_le_signal_et_suit_la_redirection() {
        // Comme Google Apps Script : l'envoi est redirigé vers la réponse, lue par un GET.
        let serveur = serveur_local(|methode, chemin| match (methode, chemin) {
            ("POST", "/statistiques") => (302, "Location: /reponse\r\n", String::new()),
            ("GET", "/reponse") => (200, "", r#"{"ok":true}"#.to_string()),
            _ => (404, "", String::new()),
        });
        let mut reglages = Reglages::default();
        reglages.regler(None, Some(true)).unwrap();
        let signal = reglages.signal("fr-FR", "Windows 11 24H2 (26100)").unwrap();
        envoyer(&format!("{}/statistiques", serveur.adresse), &signal).unwrap();
        let requetes = serveur.requetes();
        assert_eq!(requetes.len(), 2, "{requetes:?}");
        let (methode, chemin, corps) = &requetes[0];
        assert_eq!(
            (methode.as_str(), chemin.as_str()),
            ("POST", "/statistiques")
        );
        let recu: serde_json::Value = serde_json::from_str(corps).unwrap();
        assert_eq!(
            recu["installation"],
            reglages.installation.unwrap().as_str()
        );
        assert_eq!(recu["evenement"], "installation");
        assert_eq!(recu["langue"], "fr-FR");
        assert_eq!(requetes[1].0, "GET");
    }

    #[test]
    fn refuse_une_reponse_inattendue() {
        // Service mal déployé : Google renvoie sa page de connexion, avec le statut 200.
        let serveur = serveur_local(|_, _| (200, "", "<html>Connexion</html>".to_string()));
        let mut reglages = Reglages::default();
        reglages.regler(None, Some(true)).unwrap();
        let signal = reglages.signal("fr", "Windows").unwrap();
        let erreur = envoyer(&serveur.adresse, &signal).unwrap_err();
        assert!(
            erreur.message.contains("réponse inattendue"),
            "{}",
            erreur.message
        );
        let introuvable = serveur_local(|_, _| (404, "", String::new()));
        let erreur = envoyer(&introuvable.adresse, &signal).unwrap_err();
        assert!(erreur.message.contains("404"), "{}", erreur.message);
    }
}
