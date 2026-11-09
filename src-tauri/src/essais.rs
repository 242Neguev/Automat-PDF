//! PDF construits pour les tests : pages A4 dont la taille et les ressources sont héritées de
//! l'arbre des pages, chacune avec une ligne de texte « <texte> <numéro> » en Helvetica.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use lopdf::{Document, Object, ObjectId, dictionary};

fn construire(n: usize, texte: &str, lien_vers_la_derniere: bool) -> Vec<u8> {
    let mut document = Document::with_version("1.5");
    let arbre = document.new_object_id();
    let police = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut pages: Vec<ObjectId> = Vec::new();
    for page in 0..n {
        let contenu = format!("BT /F1 24 Tf 72 720 Td ({texte} {}) Tj ET", page + 1);
        let flux = document.add_object(lopdf::Stream::new(dictionary! {}, contenu.into_bytes()));
        pages.push(
            document.add_object(
                dictionary! { "Type" => "Page", "Parent" => arbre, "Contents" => flux },
            ),
        );
    }
    if lien_vers_la_derniere && n > 1 {
        // Lien de la première page vers la dernière, comme une table des matières.
        let destination = vec![
            Object::Reference(pages[n - 1]),
            "XYZ".into(),
            0.into(),
            842.into(),
            0.into(),
        ];
        let lien = document.add_object(dictionary! {
            "Type" => "Annot", "Subtype" => "Link",
            "Rect" => vec![72.into(), 700.into(), 300.into(), 740.into()],
            "Dest" => destination,
        });
        if let Ok(premiere) = document.get_dictionary_mut(pages[0]) {
            premiere.set("Annots", vec![Object::Reference(lien)]);
        }
    }
    let enfants: Vec<Object> = pages.into_iter().map(Object::Reference).collect();
    document.objects.insert(
        arbre,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => enfants, "Count" => n as i64,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => police } },
        }),
    );
    let catalogue = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => arbre });
    document.trailer.set("Root", catalogue);
    let mut octets = Vec::new();
    document.save_to(&mut octets).unwrap();
    octets
}

/// PDF de `n` pages.
pub fn pdf_herite(n: usize, texte: &str) -> Vec<u8> {
    construire(n, texte, false)
}

/// PDF de `n` pages dont la première a un lien interne vers la dernière.
pub fn pdf_avec_lien(n: usize, texte: &str) -> Vec<u8> {
    construire(n, texte, true)
}

/// Écrit un fichier de test dans un dossier temporaire propre au test ; renvoie son chemin.
pub fn fichier_temporaire(dossier: &str, nom: &str, octets: &[u8]) -> String {
    let dossier = std::env::temp_dir().join("automat-pdf-tests").join(dossier);
    std::fs::create_dir_all(&dossier).unwrap();
    let chemin = dossier.join(nom);
    std::fs::write(&chemin, octets).unwrap();
    chemin.to_string_lossy().into_owned()
}

/// Requête reçue par le serveur local : méthode, chemin, corps.
pub type RequeteRecue = (String, String, String);

/// Petit serveur HTTP sur ce PC, pour essayer un envoi sans internet.
pub struct ServeurLocal {
    /// « http://127.0.0.1:<port> »
    pub adresse: String,
    recues: Arc<Mutex<Vec<RequeteRecue>>>,
}

impl ServeurLocal {
    pub fn requetes(&self) -> Vec<RequeteRecue> {
        self.recues.lock().unwrap().clone()
    }
}

/// Démarre un serveur local : chaque requête reçoit la réponse donnée par `repondre(méthode,
/// chemin)` : statut, en-têtes en plus (lignes terminées par « \r\n ») et corps.
pub fn serveur_local(
    repondre: impl Fn(&str, &str) -> (u16, &'static str, String) + Send + 'static,
) -> ServeurLocal {
    let ecoute = TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = format!("http://{}", ecoute.local_addr().unwrap());
    let recues = Arc::new(Mutex::new(Vec::new()));
    let journal = Arc::clone(&recues);
    std::thread::spawn(move || {
        for connexion in ecoute.incoming() {
            let Ok(mut connexion) = connexion else {
                continue;
            };
            let Some((methode, chemin, corps)) = lire_requete(&connexion) else {
                continue;
            };
            let (statut, entetes, reponse) = repondre(&methode, &chemin);
            journal.lock().unwrap().push((methode, chemin, corps));
            let _ = write!(
                connexion,
                "HTTP/1.1 {statut} Reponse\r\nContent-Length: {}\r\nConnection: close\r\n{entetes}\r\n{reponse}",
                reponse.len()
            );
        }
    });
    ServeurLocal { adresse, recues }
}

fn lire_requete(connexion: &TcpStream) -> Option<RequeteRecue> {
    let mut lecteur = BufReader::new(connexion);
    let mut premiere = String::new();
    lecteur.read_line(&mut premiere).ok()?;
    let mut morceaux = premiere.split_whitespace();
    let methode = morceaux.next()?.to_string();
    let chemin = morceaux.next()?.to_string();
    let mut longueur = 0;
    loop {
        let mut ligne = String::new();
        lecteur.read_line(&mut ligne).ok()?;
        let ligne = ligne.trim_end();
        if ligne.is_empty() {
            break;
        }
        if let Some((nom, valeur)) = ligne.split_once(':')
            && nom.eq_ignore_ascii_case("content-length")
        {
            longueur = valeur.trim().parse().ok()?;
        }
    }
    let mut corps = vec![0; longueur];
    lecteur.read_exact(&mut corps).ok()?;
    Some((
        methode,
        chemin,
        String::from_utf8_lossy(&corps).into_owned(),
    ))
}
