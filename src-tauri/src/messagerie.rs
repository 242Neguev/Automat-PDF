//! Envoi des fichiers par la messagerie de l'utilisateur (SMTP), sans Outlook : Gmail, Yahoo,
//! messagerie de l'organisation…, sous Windows comme sous macOS. La connexion est chiffrée par le
//! système, et le mot de passe gardé par lui (Gestionnaire d'identification de Windows, trousseau
//! de macOS), jamais dans un fichier.

use std::error::Error as _;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lettre::message::header::{ContentTransferEncoding, ContentType};
use lettre::message::{Attachment, Body, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::Error as ErreurSmtp;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{Address, Message as Courriel, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};

use crate::envoi::{BilanEnvoi, Message, verifier};
use crate::erreur::{Echec, Erreur};

/// Délai laissé au serveur à chaque étape (connexion, réponse).
const DELAI: Duration = Duration::from_secs(30);
/// Pauses avant de renvoyer un message que le serveur demande de réessayer plus tard (limite
/// d'envoi par minute atteinte, serveur occupé).
const REESSAIS: [Duration; 2] = [Duration::from_secs(30), Duration::from_secs(90)];
/// Service sous lequel le mot de passe est rangé dans le coffre du système.
const SERVICE: &str = "Automat-PDF";
/// Nom de l'entrée du mot de passe de la messagerie.
const ENTREE: &str = "messagerie";

/// Chiffrement de la connexion au serveur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Chiffrement {
    /// Connexion ordinaire chiffrée aussitôt (STARTTLS), en général sur le port 587.
    Starttls,
    /// Connexion chiffrée d'emblée (SSL/TLS), en général sur le port 465.
    Ssl,
    /// Aucun : serveur interne qui ne demande pas de mot de passe (port 25).
    Aucun,
}

/// Compte d'envoi : le serveur d'envoi (SMTP) et l'expéditeur.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compte {
    pub serveur: String,
    pub port: u16,
    pub chiffrement: Chiffrement,
    /// Identifiant de connexion, le plus souvent l'adresse ; vide : serveur sans mot de passe.
    pub identifiant: String,
    /// Adresse de l'expéditeur.
    pub adresse: String,
    /// Nom affiché aux destinataires (facultatif).
    #[serde(default)]
    pub nom: String,
}

impl Compte {
    /// Serveur sur cet ordinateur même : rien ne passe par le réseau.
    fn local(&self) -> bool {
        matches!(
            self.serveur.trim(),
            "localhost" | "127.0.0.1" | "::1" | "[::1]"
        )
    }

    fn decrit(&self) -> String {
        format!("« {} » (port {})", self.serveur.trim(), self.port)
    }

    fn expediteur(&self) -> Result<Mailbox, String> {
        let adresse = self.adresse.trim();
        let nom = Some(self.nom.trim().to_string()).filter(|nom| !nom.is_empty());
        adresse
            .parse::<Address>()
            .map(|adresse| Mailbox::new(nom, adresse))
            .map_err(|_| format!("Adresse d'expédition invalide « {adresse} »."))
    }
}

/// Ce qu'une erreur empêche : ce message seulement, ou tout l'envoi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Portee {
    CeMessage,
    ToutLEnvoi,
}

/// Connexion au serveur, avec l'identifiant et le mot de passe s'il en faut un.
fn transport(compte: &Compte, mot_de_passe: Option<&str>) -> Result<SmtpTransport, Erreur> {
    let serveur = compte.serveur.trim();
    if serveur.is_empty() {
        return Err(Erreur::simple(
            "Indiquez le serveur d'envoi (SMTP) de la messagerie.",
        ));
    }
    let mut constructeur = SmtpTransport::builder_dangerous(serveur)
        .port(compte.port)
        .timeout(Some(DELAI));
    if compte.chiffrement != Chiffrement::Aucun {
        let parametres = TlsParameters::new(serveur.to_string()).map_err(|erreur| {
            Erreur::simple(format!(
                "La connexion chiffrée n'a pas pu être préparée : {erreur}"
            ))
        })?;
        constructeur = constructeur.tls(match compte.chiffrement {
            Chiffrement::Ssl => Tls::Wrapper(parametres),
            _ => Tls::Required(parametres),
        });
    }
    let identifiant = compte.identifiant.trim();
    if !identifiant.is_empty() {
        if compte.chiffrement == Chiffrement::Aucun && !compte.local() {
            return Err(Erreur::simple(
                "Sans chiffrement, le mot de passe circulerait en clair : choisissez STARTTLS ou SSL/TLS, ou laissez l'identifiant vide pour un serveur interne qui n'en demande pas.",
            ));
        }
        let mot_de_passe = mot_de_passe
            .filter(|mot_de_passe| !mot_de_passe.is_empty())
            .ok_or_else(|| {
                Erreur::simple(
                    "Le mot de passe de la messagerie n'est pas enregistré : saisissez-le dans les réglages de la messagerie.",
                )
            })?;
        constructeur =
            constructeur.credentials(Credentials::new(identifiant.into(), mot_de_passe.into()));
    }
    Ok(constructeur.build())
}

/// Identifiant unique d'un message, au nom du domaine de l'expéditeur : les filtres anti-spam se
/// méfient de ceux qui portent le nom de l'ordinateur.
fn identifiant_de_message(adresse: &str) -> String {
    static SUITE: AtomicU64 = AtomicU64::new(0);
    let domaine = adresse
        .rsplit_once('@')
        .map_or("automat-pdf.invalid", |(_, domaine)| domaine);
    let mut octets = [0u8; 12];
    let _ = getrandom::fill(&mut octets);
    let aleatoire: String = octets.iter().map(|octet| format!("{octet:02x}")).collect();
    // Même sans hasard, deux messages n'ont jamais le même identifiant.
    let instant = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duree| duree.as_nanos());
    let suite = SUITE.fetch_add(1, Ordering::Relaxed);
    format!("<{aleatoire}.{instant:x}.{suite}@{domaine}>")
}

/// Le courriel d'un message : expéditeur, destinataires, objet, texte et pièces jointes.
fn courriel(compte: &Compte, message: &Message) -> Result<Courriel, String> {
    let mut constructeur = Courriel::builder()
        .from(compte.expediteur()?)
        .subject(message.objet.trim())
        .message_id(Some(identifiant_de_message(compte.adresse.trim())));
    let mut destinataires = 0;
    for destinataire in message
        .a
        .split([';', ','])
        .map(str::trim)
        .filter(|a| !a.is_empty())
    {
        let boite: Mailbox = destinataire
            .parse()
            .map_err(|_| format!("Adresse invalide « {destinataire} »."))?;
        constructeur = constructeur.to(boite);
        destinataires += 1;
    }
    if destinataires == 0 {
        return Err("Aucune adresse de destinataire.".into());
    }
    let mut parties =
        MultiPart::mixed().singlepart(SinglePart::plain(message.texte.replace("\r\n", "\n")));
    for piece in &message.pieces {
        let chemin = Path::new(piece);
        let contenu = std::fs::read(chemin)
            .map_err(|erreur| format!("« {piece} » n'a pas pu être lu : {erreur}"))?;
        let nom = chemin.file_name().map_or_else(
            || "piece-jointe".to_string(),
            |nom| nom.to_string_lossy().into_owned(),
        );
        let genre = if nom.to_lowercase().ends_with(".pdf") {
            "application/pdf"
        } else {
            "application/octet-stream"
        };
        let genre = ContentType::parse(genre).map_err(|erreur| erreur.to_string())?;
        // En base64 quel que soit le contenu : un PDF dont tous les octets sont du texte partirait
        // sinon tel quel, et un serveur qui réécrit les fins de ligne fausserait ses positions.
        let corps = Body::new_with_encoding(contenu, ContentTransferEncoding::Base64)
            .map_err(|_| format!("« {piece} » n'a pas pu être joint au message."))?;
        parties = parties.singlepart(Attachment::new(nom).body(corps, genre));
    }
    constructeur
        .multipart(parties)
        .map_err(|erreur| format!("Le message n'a pas pu être préparé : {erreur}"))
}

/// Réponse d'une identification refusée (identifiant, mot de passe, mot de passe d'application
/// exigé, identification interdite).
fn identification_refusee(erreur: &ErreurSmtp) -> bool {
    erreur
        .status()
        .is_some_and(|code| matches!(code.to_string().as_str(), "530" | "534" | "535" | "538"))
}

fn portee(erreur: &ErreurSmtp) -> Portee {
    // Une réponse du serveur (destinataire refusé, message trop lourd…) ne touche que ce message ;
    // le reste (connexion perdue, chiffrement, identification) arrête tout.
    if erreur.status().is_some() && !identification_refusee(erreur) {
        Portee::CeMessage
    } else {
        Portee::ToutLEnvoi
    }
}

/// Connexion établie puis perdue (coupée par le serveur, réponse inachevée) : le serveur a été
/// joint, ce n'est pas lui qui est introuvable.
fn connexion_coupee(erreur: &ErreurSmtp) -> bool {
    if erreur.is_response() {
        return true;
    }
    let mut cause = erreur.source();
    while let Some(erreur) = cause {
        if let Some(systeme) = erreur.downcast_ref::<std::io::Error>() {
            return matches!(
                systeme.kind(),
                ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
                    | ErrorKind::BrokenPipe
                    | ErrorKind::UnexpectedEof
            );
        }
        cause = erreur.source();
    }
    false
}

/// Texte de la réponse du serveur, tel quel (souvent en anglais), ou de l'erreur du système.
fn reponse(erreur: &ErreurSmtp) -> String {
    erreur
        .source()
        .map_or_else(|| erreur.to_string(), |source| source.to_string())
}

/// Explication en clair d'une erreur d'envoi.
fn explication(erreur: &ErreurSmtp, compte: &Compte) -> String {
    if identification_refusee(erreur) {
        return format!(
            "Le serveur refuse l'identifiant ou le mot de passe ({}). Gmail, Yahoo et iCloud demandent un « mot de passe d'application » ; avec Microsoft 365, l'administrateur doit autoriser l'envoi SMTP authentifié.",
            reponse(erreur)
        );
    }
    if erreur.status().is_some() {
        return if erreur.is_transient() {
            format!(
                "Le serveur demande de réessayer plus tard : {}",
                reponse(erreur)
            )
        } else {
            format!("Refusé par le serveur : {}", reponse(erreur))
        };
    }
    if erreur.is_tls() {
        return format!(
            "La connexion chiffrée avec {} a échoué : vérifiez le chiffrement (STARTTLS pour le port 587, SSL/TLS pour le port 465). {}",
            compte.decrit(),
            reponse(erreur)
        );
    }
    if erreur.is_timeout() {
        return format!(
            "Le serveur {} ne répond pas : vérifiez le port et le chiffrement.",
            compte.decrit()
        );
    }
    if connexion_coupee(erreur) {
        return format!(
            "La connexion avec le serveur {} a été coupée en cours d'envoi ({}) : réessayez ; si cela se reproduit, vérifiez le port et le chiffrement.",
            compte.decrit(),
            reponse(erreur)
        );
    }
    if erreur.is_client() {
        return format!(
            "Le serveur {} n'accepte pas cette identification ({}) : vérifiez le chiffrement, ou laissez l'identifiant vide pour un serveur qui ne demande pas de mot de passe.",
            compte.decrit(),
            reponse(erreur)
        );
    }
    format!(
        "Impossible de joindre le serveur {} : vérifiez son nom, le port et la connexion à internet ({}).",
        compte.decrit(),
        reponse(erreur)
    )
}

/// Envoie un courriel ; s'il est refusé pour un temps (limite d'envoi, serveur occupé), il est
/// renvoyé après chaque pause de `reessais`.
fn envoyer_un(
    transport: &SmtpTransport,
    courriel: &Courriel,
    reessais: &[Duration],
) -> Result<(), ErreurSmtp> {
    let mut pauses = reessais.iter();
    loop {
        match transport.send(courriel) {
            Ok(_) => return Ok(()),
            Err(erreur) if erreur.is_transient() => match pauses.next() {
                Some(pause) => thread::sleep(*pause),
                None => return Err(erreur),
            },
            Err(erreur) => return Err(erreur),
        }
    }
}

fn envoyer_avec(
    compte: &Compte,
    mot_de_passe: Option<&str>,
    messages: &[Message],
    reessais: &[Duration],
    progression: impl Fn(usize, usize),
) -> Result<BilanEnvoi, Erreur> {
    verifier(messages)?;
    compte.expediteur().map_err(Erreur::simple)?;
    let transport = transport(compte, mot_de_passe)?;
    let total = messages.len();
    let mut bilan = BilanEnvoi::default();
    for (rang, message) in messages.iter().enumerate() {
        let envoi = courriel(compte, message)
            .map_err(|raison| (Portee::CeMessage, raison))
            .and_then(|courriel| {
                envoyer_un(&transport, &courriel, reessais)
                    .map_err(|erreur| (portee(&erreur), explication(&erreur, compte)))
            });
        match envoi {
            Ok(()) => bilan.reussis += 1,
            Err((Portee::CeMessage, raison)) => bilan.echecs.push(Echec {
                rang,
                message: raison,
            }),
            Err((Portee::ToutLEnvoi, raison)) => {
                // Rien n'est parti : l'erreur suffit. Sinon, ce qui reste est signalé non envoyé.
                if bilan.reussis == 0 {
                    return Err(Erreur::simple(raison));
                }
                bilan.echecs.extend((rang..total).map(|rang| Echec {
                    rang,
                    message: format!("Non envoyé : {raison}"),
                }));
                progression(total, total);
                return Ok(bilan);
            }
        }
        progression(rang + 1, total);
    }
    Ok(bilan)
}

/// Envoie les messages par la messagerie, avec le mot de passe gardé par le système ;
/// `progression` reçoit le nombre de messages traités et le total.
pub fn envoyer(
    compte: &Compte,
    messages: &[Message],
    progression: impl Fn(usize, usize),
) -> Result<BilanEnvoi, Erreur> {
    let mot_de_passe = if compte.identifiant.trim().is_empty() {
        None
    } else {
        lire_coffre(ENTREE)?
    };
    envoyer_avec(
        compte,
        mot_de_passe.as_deref(),
        messages,
        &REESSAIS,
        progression,
    )
}

/// Vérifie la connexion au serveur et l'identification, sans rien envoyer ; `mot_de_passe` :
/// celui qui vient d'être saisi, sinon celui gardé par le système.
pub fn tester(compte: &Compte, mot_de_passe: Option<&str>) -> Result<(), Erreur> {
    compte.expediteur().map_err(Erreur::simple)?;
    let garde = match mot_de_passe {
        None if !compte.identifiant.trim().is_empty() => lire_coffre(ENTREE)?,
        _ => None,
    };
    let transport = transport(compte, mot_de_passe.or(garde.as_deref()))?;
    match transport.test_connection() {
        Ok(true) => Ok(()),
        Ok(false) => Err(Erreur::simple(format!(
            "Le serveur {} ne répond pas comme prévu.",
            compte.decrit()
        ))),
        Err(erreur) => Err(Erreur::simple(explication(&erreur, compte))),
    }
}

fn coffre(nom: &str) -> Result<keyring::Entry, Erreur> {
    keyring::Entry::new(SERVICE, nom).map_err(|erreur| {
        Erreur::simple(format!(
            "Le coffre des mots de passe du système est inaccessible : {erreur}"
        ))
    })
}

fn lire_coffre(nom: &str) -> Result<Option<String>, Erreur> {
    match coffre(nom)?.get_password() {
        Ok(mot_de_passe) => Ok(Some(mot_de_passe)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(erreur) => Err(Erreur::simple(format!(
            "Le mot de passe de la messagerie n'a pas pu être lu : {erreur}"
        ))),
    }
}

fn ecrire_coffre(nom: &str, mot_de_passe: &str) -> Result<(), Erreur> {
    let entree = coffre(nom)?;
    let resultat = if mot_de_passe.is_empty() {
        match entree.delete_credential() {
            Err(keyring::Error::NoEntry) => Ok(()),
            autre => autre,
        }
    } else {
        entree.set_password(mot_de_passe)
    };
    resultat.map_err(|erreur| {
        Erreur::simple(format!(
            "Le mot de passe de la messagerie n'a pas pu être enregistré : {erreur}"
        ))
    })
}

/// Un mot de passe de messagerie est-il gardé par le système ?
pub fn mot_de_passe_enregistre() -> Result<bool, Erreur> {
    Ok(lire_coffre(ENTREE)?.is_some())
}

/// Garde le mot de passe de la messagerie dans le coffre du système ; vide : l'oublie.
pub fn enregistrer_mot_de_passe(mot_de_passe: &str) -> Result<(), Erreur> {
    ecrire_coffre(ENTREE, mot_de_passe)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::essais::fichier_temporaire;

    /// Un message reçu par le serveur d'essai.
    #[derive(Debug, Default, Clone)]
    struct Recu {
        de: String,
        a: Vec<String>,
        donnees: String,
    }

    /// Comportement du serveur d'essai.
    #[derive(Default)]
    struct Scenario {
        /// Réponses opposées, dans l'ordre, aux premiers destinataires (les suivants sont acceptés).
        refus: Vec<&'static str>,
        /// Réponse à l'identification ; acceptée sinon.
        identification: Option<&'static str>,
        /// Après ce nombre de messages, le serveur s'arrête : connexion coupée, plus de réponse.
        arret_apres: Option<usize>,
    }

    struct Etat {
        scenario: Scenario,
        refus: Mutex<VecDeque<&'static str>>,
        recus: Mutex<Vec<Recu>>,
        arrete: AtomicBool,
    }

    /// Serveur SMTP d'essai, sur cet ordinateur et sans chiffrement.
    struct ServeurEssai {
        port: u16,
        etat: Arc<Etat>,
    }

    impl ServeurEssai {
        fn demarrer(scenario: Scenario) -> Self {
            let ecoute = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = ecoute.local_addr().unwrap().port();
            let etat = Arc::new(Etat {
                refus: Mutex::new(scenario.refus.iter().copied().collect()),
                scenario,
                recus: Mutex::new(Vec::new()),
                arrete: AtomicBool::new(false),
            });
            let partage = etat.clone();
            thread::spawn(move || {
                for flux in ecoute.incoming() {
                    // Arrêté : la connexion et l'écoute sont fermées, plus personne ne répond.
                    let Ok(flux) = flux else { return };
                    if partage.arrete.load(Ordering::SeqCst) {
                        return;
                    }
                    let etat = partage.clone();
                    thread::spawn(move || dialoguer(flux, &etat));
                }
            });
            Self { port, etat }
        }

        fn recus(&self) -> Vec<Recu> {
            self.etat.recus.lock().unwrap().clone()
        }
    }

    fn dialoguer(flux: TcpStream, etat: &Etat) {
        let Ok(lecture) = flux.try_clone() else {
            return;
        };
        let mut lecteur = BufReader::new(lecture);
        let mut ecriture = flux;
        let mut repondre = |texte: &str| {
            ecriture
                .write_all(format!("{texte}\r\n").as_bytes())
                .is_ok()
        };
        repondre("220 essai ESMTP");
        let mut courant = Recu::default();
        let mut ligne = String::new();
        loop {
            ligne.clear();
            if lecteur.read_line(&mut ligne).unwrap_or(0) == 0 {
                return;
            }
            let commande = ligne.trim_end().to_string();
            let majuscules = commande.to_ascii_uppercase();
            let reponse = if majuscules.starts_with("EHLO") {
                "250-essai\r\n250-AUTH PLAIN LOGIN\r\n250 8BITMIME"
            } else if majuscules.starts_with("AUTH") {
                etat.scenario
                    .identification
                    .unwrap_or("235 2.7.0 Bienvenue")
            } else if majuscules.starts_with("MAIL FROM:") {
                courant = Recu {
                    de: commande[10..].trim().into(),
                    ..Recu::default()
                };
                "250 OK"
            } else if majuscules.starts_with("RCPT TO:") {
                match etat.refus.lock().unwrap().pop_front() {
                    Some(refus) => refus,
                    None => {
                        courant.a.push(commande[8..].trim().into());
                        "250 OK"
                    }
                }
            } else if majuscules == "DATA" {
                repondre("354 Terminez par un point seul");
                let mut donnees = String::new();
                loop {
                    ligne.clear();
                    if lecteur.read_line(&mut ligne).unwrap_or(0) == 0 {
                        return;
                    }
                    if ligne == ".\r\n" {
                        break;
                    }
                    donnees.push_str(&ligne);
                }
                courant.donnees = donnees;
                let mut recus = etat.recus.lock().unwrap();
                recus.push(std::mem::take(&mut courant));
                if etat.scenario.arret_apres == Some(recus.len()) {
                    etat.arrete.store(true, Ordering::SeqCst);
                    repondre("250 OK");
                    return;
                }
                "250 OK"
            } else if majuscules == "QUIT" {
                repondre("221 Au revoir");
                return;
            } else {
                // RSET, NOOP
                "250 OK"
            };
            if !repondre(reponse) {
                return;
            }
        }
    }

    fn compte_essai(port: u16) -> Compte {
        Compte {
            serveur: "127.0.0.1".into(),
            port,
            chiffrement: Chiffrement::Aucun,
            identifiant: String::new(),
            adresse: "rh@exemple.com".into(),
            nom: "Service RH".into(),
        }
    }

    fn message_essai() -> Message {
        Message {
            a: "a.diallo@exemple.com".into(),
            objet: "Votre bulletin de paie".into(),
            texte: "Bonjour,\nci-joint votre bulletin.".into(),
            pieces: vec![fichier_temporaire(
                "messagerie",
                "Bulletin.pdf",
                b"%PDF-1.7 essai",
            )],
        }
    }

    #[test]
    fn envoie_chaque_message_avec_ses_destinataires_et_sa_piece_jointe() {
        let serveur = ServeurEssai::demarrer(Scenario::default());
        let bulletin = fichier_temporaire(
            "messagerie",
            "Bulletin DIALLO Aïssatou.pdf",
            b"%PDF-1.7 essai",
        );
        // Un PDF dont tous les octets sont du texte, avec des fins de ligne d'Unix.
        let texte_seul = fichier_temporaire(
            "messagerie",
            "Attestation.pdf",
            b"%PDF-1.7\nligne 1\nligne 2\n",
        );
        let messages = [
            Message {
                a: "a.diallo@exemple.com".into(),
                objet: "Votre bulletin de paie d'octobre".into(),
                texte: "Bonjour Aïssatou,\nci-joint votre bulletin.".into(),
                pieces: vec![bulletin.clone()],
            },
            Message {
                a: "b.okemba@exemple.com ; compta@exemple.com".into(),
                objet: "Bulletin".into(),
                texte: "Bonjour".into(),
                pieces: vec![bulletin, texte_seul],
            },
        ];
        let etapes = RefCell::new(Vec::new());
        let bilan = envoyer_avec(
            &compte_essai(serveur.port),
            None,
            &messages,
            &[],
            |fait, total| etapes.borrow_mut().push((fait, total)),
        )
        .unwrap();
        assert_eq!(bilan.reussis, 2);
        assert!(bilan.echecs.is_empty(), "{:?}", bilan.echecs);
        assert_eq!(*etapes.borrow(), [(1, 2), (2, 2)]);
        let recus = serveur.recus();
        assert_eq!(recus.len(), 2);
        assert!(
            recus[0].de.starts_with("<rh@exemple.com>"),
            "{}",
            recus[0].de
        );
        assert_eq!(recus[0].a, ["<a.diallo@exemple.com>"]);
        assert_eq!(
            recus[1].a,
            ["<b.okemba@exemple.com>", "<compta@exemple.com>"]
        );
        let donnees = &recus[0].donnees;
        let en_tete = |nom: &str| {
            donnees
                .lines()
                .find(|ligne| ligne.starts_with(nom))
                .unwrap_or_default()
                .to_string()
        };
        assert_eq!(en_tete("From:"), r#"From: "Service RH" <rh@exemple.com>"#);
        assert_eq!(
            en_tete("Subject:"),
            "Subject: Votre bulletin de paie d'octobre"
        );
        assert!(
            en_tete("Message-ID:").ends_with("@exemple.com>"),
            "{donnees}"
        );
        assert!(donnees.contains("Content-Type: text/plain; charset=utf-8"));
        assert!(
            donnees.contains("Content-Type: application/pdf"),
            "{donnees}"
        );
        // Le texte, accents compris, et le nom accentué de la pièce jointe sont codés, pas abîmés.
        assert!(donnees.contains("Bonjour A=C3=AFssatou,"), "{donnees}");
        assert!(
            donnees.contains("filename*0*=utf-8''Bulletin%20DIALLO%20A%C3%AFssatou.pdf"),
            "{donnees}"
        );
        // « %PDF-1.7 essai », en base64 : la pièce jointe arrive intacte.
        assert!(
            donnees.contains("Content-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjcgZXNzYWk=\r\n"),
            "{donnees}"
        );
        // Le PDF tout en texte aussi, octet pour octet : ses fins de ligne n'ont pas été réécrites.
        assert!(
            recus[1]
                .donnees
                .contains("JVBERi0xLjcKbGlnbmUgMQpsaWduZSAyCg=="),
            "{}",
            recus[1].donnees
        );
        assert_ne!(
            en_tete("Message-ID:"),
            recus[1]
                .donnees
                .lines()
                .find(|ligne| ligne.starts_with("Message-ID:"))
                .unwrap()
        );
    }

    #[test]
    fn renvoie_un_message_que_le_serveur_demande_de_reessayer() {
        let serveur = ServeurEssai::demarrer(Scenario {
            refus: vec!["451 4.7.1 Trop de messages, reessayez plus tard"],
            ..Scenario::default()
        });
        let bilan = envoyer_avec(
            &compte_essai(serveur.port),
            None,
            &[message_essai()],
            &[Duration::from_millis(10)],
            |_, _| {},
        )
        .unwrap();
        assert_eq!(bilan.reussis, 1, "{:?}", bilan.echecs);
        assert_eq!(serveur.recus().len(), 1);
    }

    #[test]
    fn continue_apres_un_destinataire_refuse() {
        let serveur = ServeurEssai::demarrer(Scenario {
            refus: vec!["550 5.1.1 Destinataire inconnu"],
            ..Scenario::default()
        });
        let invalide = Message {
            a: "jean dupont@exemple.com".into(),
            ..message_essai()
        };
        let bilan = envoyer_avec(
            &compte_essai(serveur.port),
            None,
            &[message_essai(), invalide, message_essai()],
            &[],
            |_, _| {},
        )
        .unwrap();
        assert_eq!(bilan.reussis, 1);
        assert_eq!(
            bilan.echecs,
            [
                Echec {
                    rang: 0,
                    message: "Refusé par le serveur : 5.1.1 Destinataire inconnu".into()
                },
                Echec {
                    rang: 1,
                    message: "Adresse invalide « jean dupont@exemple.com ».".into()
                }
            ]
        );
        assert_eq!(serveur.recus().len(), 1);
    }

    #[test]
    fn s_arrete_si_le_serveur_refuse_l_identification() {
        let serveur = ServeurEssai::demarrer(Scenario {
            identification: Some("535 5.7.8 Identifiants refuses"),
            ..Scenario::default()
        });
        // Sans chiffrement, un mot de passe n'est admis que vers cet ordinateur même.
        let compte = Compte {
            identifiant: "rh@exemple.com".into(),
            ..compte_essai(serveur.port)
        };
        for erreur in [
            envoyer_avec(
                &compte,
                Some("faux"),
                &[message_essai(), message_essai()],
                &[],
                |_, _| {},
            )
            .unwrap_err(),
            tester(&compte, Some("faux")).unwrap_err(),
        ] {
            assert!(
                erreur
                    .message
                    .starts_with("Le serveur refuse l'identifiant ou le mot de passe (5.7.8"),
                "{}",
                erreur.message
            );
        }
        assert!(serveur.recus().is_empty());
        assert!(
            tester(
                &Compte {
                    identifiant: String::new(),
                    ..compte
                },
                None
            )
            .is_ok()
        );
    }

    #[test]
    fn signale_ce_qui_n_est_pas_parti_quand_le_serveur_s_arrete() {
        let serveur = ServeurEssai::demarrer(Scenario {
            arret_apres: Some(1),
            ..Scenario::default()
        });
        let bilan = envoyer_avec(
            &compte_essai(serveur.port),
            None,
            &[message_essai(), message_essai(), message_essai()],
            &[],
            |_, _| {},
        )
        .unwrap();
        assert_eq!(bilan.reussis, 1);
        assert_eq!(
            bilan
                .echecs
                .iter()
                .map(|echec| echec.rang)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        // Le serveur avait répondu : la connexion a été coupée, il n'est pas introuvable.
        assert!(
            bilan.echecs[0].message.starts_with(&format!(
                "Non envoyé : La connexion avec le serveur « 127.0.0.1 » (port {}) a été coupée",
                serveur.port
            )),
            "{}",
            bilan.echecs[0].message
        );
    }

    #[test]
    fn explique_un_serveur_injoignable() {
        // Un port libéré aussitôt réservé : personne n'y répond.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let erreur = envoyer_avec(
            &compte_essai(port),
            None,
            &[message_essai()],
            &[],
            |_, _| {},
        )
        .unwrap_err();
        assert!(
            erreur.message.starts_with(&format!(
                "Impossible de joindre le serveur « 127.0.0.1 » (port {port})"
            )),
            "{}",
            erreur.message
        );
    }

    #[test]
    fn ne_laisse_pas_passer_un_mot_de_passe_en_clair_ni_un_compte_incomplet() {
        let distant = Compte {
            serveur: "smtp.exemple.com".into(),
            identifiant: "rh@exemple.com".into(),
            ..compte_essai(25)
        };
        assert!(
            tester(&distant, Some("secret"))
                .unwrap_err()
                .message
                .contains("en clair")
        );
        let sans_mot_de_passe = Compte {
            chiffrement: Chiffrement::Starttls,
            port: 587,
            ..distant.clone()
        };
        assert!(
            envoyer_avec(&sans_mot_de_passe, None, &[message_essai()], &[], |_, _| {})
                .unwrap_err()
                .message
                .contains("n'est pas enregistré")
        );
        let sans_serveur = Compte {
            serveur: " ".into(),
            ..sans_mot_de_passe.clone()
        };
        assert!(
            tester(&sans_serveur, Some("secret"))
                .unwrap_err()
                .message
                .starts_with("Indiquez le serveur")
        );
        let mauvaise_adresse = Compte {
            adresse: "rh chez exemple".into(),
            ..sans_mot_de_passe
        };
        assert_eq!(
            tester(&mauvaise_adresse, Some("secret"))
                .unwrap_err()
                .message,
            "Adresse d'expédition invalide « rh chez exemple »."
        );
    }

    #[test]
    fn lit_les_reglages_de_l_interface() {
        let compte: Compte = serde_json::from_str(
            r#"{"serveur":"smtp.gmail.com","port":587,"chiffrement":"starttls","identifiant":"rh@gmail.com","adresse":"rh@gmail.com","nom":"Service RH"}"#,
        )
        .unwrap();
        assert_eq!(compte.chiffrement, Chiffrement::Starttls);
        let ssl: Chiffrement = serde_json::from_str(r#""ssl""#).unwrap();
        assert_eq!(ssl, Chiffrement::Ssl);
        assert_eq!(
            serde_json::to_string(&Chiffrement::Aucun).unwrap(),
            r#""aucun""#
        );
    }

    /// Le coffre de Windows ne demande rien ; celui de macOS demanderait une autorisation.
    #[cfg(windows)]
    #[test]
    fn garde_le_mot_de_passe_dans_le_coffre_du_systeme() {
        let nom = format!("essai-{}", std::process::id());
        assert_eq!(lire_coffre(&nom).unwrap(), None);
        ecrire_coffre(&nom, "secret été 2026").unwrap();
        assert_eq!(
            lire_coffre(&nom).unwrap().as_deref(),
            Some("secret été 2026")
        );
        ecrire_coffre(&nom, "").unwrap();
        assert_eq!(lire_coffre(&nom).unwrap(), None);
        // L'oublier une seconde fois n'est pas une erreur.
        ecrire_coffre(&nom, "").unwrap();
    }
}
