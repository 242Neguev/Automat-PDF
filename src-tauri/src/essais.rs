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

/// Formulaire d'essai « Demande de congés » d'une page A4, comme en produisent les logiciels de
/// bureau : un groupe « Salarie » (nom, prénom, matricule en cinq cases), une liste déroulante
/// (service), trois boutons radio (type de congé), des dates, un nombre aligné à droite, un motif
/// de plusieurs lignes, une case à cocher (état « Oui »), un visa en lecture seule déjà rempli, et
/// un champ de signature. Les cases et les boutons radio ont leurs apparences ; celles des champs
/// de texte sont dessinées par `formulaires::completer_apparences`.
pub fn pdf_formulaire() -> Vec<u8> {
    use crate::police::chaine;
    use lopdf::{Stream, StringFormat};

    let mut document = Document::with_version("1.7");
    let arbre = document.new_object_id();
    let page = document.new_object_id();
    let texte = |t: &str| Object::String(t.as_bytes().to_vec(), StringFormat::Literal);
    let rect = |x0: i64, y0: i64, x1: i64, y1: i64| -> Object {
        vec![x0.into(), y0.into(), x1.into(), y1.into()].into()
    };
    let gris =
        || -> Object { vec![Object::Real(0.55), Object::Real(0.55), Object::Real(0.55)].into() };
    let mut annotations: Vec<Object> = Vec::new();
    let mut champs: Vec<Object> = Vec::new();

    // Une apparence de case (signe ZapfDingbats), et la case vide.
    // Les clés de `plus` remplacent celles de `base` (lopdf `extend` les réunirait en tableau).
    let fusion = |mut base: lopdf::Dictionary, plus: &lopdf::Dictionary| {
        for (cle, valeur) in plus.iter() {
            base.set(cle.clone(), valeur.clone());
        }
        base
    };
    let etat = |document: &mut Document, signe: &str, cote: f64| -> (ObjectId, ObjectId) {
        let flux = |contenu: String| {
            Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Form",
                    "BBox" => vec![0.into(), 0.into(), Object::Real(cote as f32), Object::Real(cote as f32)],
                    "Resources" => dictionary! { "Font" => dictionary! {
                        "ZaDb" => dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "ZapfDingbats" },
                    } },
                },
                contenu.into_bytes(),
            )
        };
        let bord = format!(
            "0.55 G 1 w 0.5 0.5 {:.1} {:.1} re S\n",
            cote - 1.0,
            cote - 1.0
        );
        let taille = cote * 0.8;
        let coche = format!(
            "q {bord}BT /ZaDb {taille:.1} Tf 0 g {:.2} {:.2} Td ({signe}) Tj ET Q",
            (cote - taille * 0.8) / 2.0,
            (cote - taille * 0.7) / 2.0
        );
        (
            document.add_object(flux(coche)),
            document.add_object(flux(format!("q {bord}Q"))),
        )
    };
    let widget = |extra: lopdf::Dictionary, rectangle: Object| -> lopdf::Dictionary {
        let base = dictionary! {
            "Type" => "Annot", "Subtype" => "Widget", "Rect" => rectangle, "P" => page, "F" => 4,
            "MK" => dictionary! { "BC" => gris() },
            "BS" => dictionary! { "W" => 1, "S" => "S" },
        };
        fusion(base, &extra)
    };

    // Le groupe « Salarie » : nom, prénom, matricule (champs de texte, sorte héritée).
    let salarie = document.new_object_id();
    let mut enfants = Vec::new();
    for (nom, rectangle, plus) in [
        ("Nom", rect(150, 692, 330, 712), dictionary! {}),
        (
            "Prenom",
            rect(400, 692, 540, 712),
            dictionary! { "TU" => crate::annotations::texte_pdf("Prénom") },
        ),
        (
            "Matricule",
            rect(150, 652, 250, 672),
            dictionary! { "Ff" => 1 << 24, "MaxLen" => 5 },
        ),
    ] {
        let champ = fusion(
            widget(
                dictionary! { "T" => texte(nom), "Parent" => salarie },
                rectangle,
            ),
            &plus,
        );
        let id = document.add_object(champ);
        enfants.push(Object::Reference(id));
        annotations.push(Object::Reference(id));
    }
    document.objects.insert(
        salarie,
        Object::Dictionary(dictionary! {
            "T" => texte("Salarie"), "FT" => "Tx", "DA" => texte("/Helv 0 Tf 0 g"), "Kids" => enfants,
        }),
    );
    champs.push(Object::Reference(salarie));

    // Le service : une liste déroulante.
    let service = document.add_object(widget(
        dictionary! {
            "T" => texte("Service"), "FT" => "Ch", "Ff" => 1 << 17, "DA" => texte("/Helv 0 Tf 0 g"),
            "Opt" => vec![
                Object::String(vec![b'C', b'o', b'm', b'p', b't', b'a', b'b', b'i', b'l', b'i', b't', 0xE9], StringFormat::Literal),
                texte("Direction"), texte("Informatique"), texte("Ressources humaines"),
            ],
        },
        rect(345, 652, 540, 672),
    ));
    champs.push(Object::Reference(service));
    annotations.push(Object::Reference(service));

    // Le type de congé : trois boutons radio (un coché ne se décoche pas d'un clic).
    let type_conge = document.new_object_id();
    let mut radios = Vec::new();
    for (etat_actif, x) in [("Conges", 150), ("RTT", 290), ("SansSolde", 400)] {
        let (coche, vide) = etat(&mut document, "l", 12.0);
        let mut normale = lopdf::Dictionary::new();
        normale.set(etat_actif, Object::Reference(coche));
        normale.set("Off", Object::Reference(vide));
        let id = document.add_object(widget(
            dictionary! {
                "Parent" => type_conge, "AS" => "Off",
                "AP" => dictionary! { "N" => normale },
                "MK" => dictionary! { "BC" => gris(), "CA" => texte("l") },
            },
            rect(x, 610, x + 12, 622),
        ));
        radios.push(Object::Reference(id));
        annotations.push(Object::Reference(id));
    }
    document.objects.insert(
        type_conge,
        Object::Dictionary(dictionary! {
            "T" => texte("Type"), "FT" => "Btn", "Ff" => (1 << 15) | (1 << 14), "Kids" => radios,
        }),
    );
    champs.push(Object::Reference(type_conge));

    // Dates, nombre de jours (aligné à droite), motif sur plusieurs lignes.
    for (nom, rectangle, plus) in [
        ("Du", rect(150, 567, 260, 587), dictionary! {}),
        ("Au", rect(310, 567, 420, 587), dictionary! {}),
        (
            "Jours",
            rect(480, 567, 540, 587),
            dictionary! { "Q" => 2, "DA" => texte("/Helv 11 Tf 0 g") },
        ),
        (
            "Motif",
            rect(150, 470, 540, 545),
            dictionary! { "Ff" => 1 << 12, "DA" => texte("/Helv 10 Tf 0 g") },
        ),
    ] {
        let champ = fusion(
            widget(
                dictionary! { "T" => texte(nom), "FT" => "Tx", "DA" => texte("/Helv 0 Tf 0 g") },
                rectangle,
            ),
            &plus,
        );
        let id = document.add_object(champ);
        champs.push(Object::Reference(id));
        annotations.push(Object::Reference(id));
    }

    // La case « Je certifie… » : son état coché s'appelle « Oui ».
    let (coche, vide) = etat(&mut document, "4", 12.0);
    let certifie = document.add_object(widget(
        dictionary! {
            "T" => texte("Certifie"), "FT" => "Btn", "V" => "Off", "AS" => "Off",
            "AP" => dictionary! { "N" => dictionary! { "Oui" => Object::Reference(coche), "Off" => Object::Reference(vide) } },
            "MK" => dictionary! { "BC" => gris(), "CA" => texte("4") },
        },
        rect(56, 432, 68, 444),
    ));
    champs.push(Object::Reference(certifie));
    annotations.push(Object::Reference(certifie));

    // Le visa du responsable, en lecture seule ; la signature du salarié.
    let visa = document.add_object(widget(
        dictionary! {
            "T" => texte("Visa"), "FT" => "Tx", "Ff" => 1, "V" => texte("En attente"),
            "DA" => texte("/Helv 10 Tf 0 g"),
        },
        rect(200, 375, 400, 400),
    ));
    let signature = document.add_object(widget(
        dictionary! { "T" => texte("Signature"), "FT" => "Sig" },
        rect(56, 290, 260, 340),
    ));
    for id in [visa, signature] {
        champs.push(Object::Reference(id));
        annotations.push(Object::Reference(id));
    }

    // La page : titre, libellés, et les champs.
    let mut contenu = String::from("BT /F2 20 Tf 56 780 Td ");
    contenu.push_str(&format!("{} Tj ET\n", chaine("Demande de congés")));
    contenu.push_str(&format!(
        "BT /F1 11 Tf 56 758 Td {} Tj ET\n",
        chaine("Eau sèche SARL — Service du personnel")
    ));
    for (x, y, libelle) in [
        (56, 698, "Nom"),
        (345, 698, "Prénom"),
        (56, 658, "Matricule"),
        (280, 658, "Service"),
        (56, 613, "Type de congé :"),
        (168, 613, "Congés payés"),
        (308, 613, "RTT"),
        (418, 613, "Sans solde"),
        (56, 573, "Du"),
        (280, 573, "Au"),
        (440, 573, "Jours"),
        (56, 531, "Motif"),
        (
            76,
            434,
            "Je certifie l'exactitude des informations ci-dessus.",
        ),
        (56, 383, "Visa du responsable"),
        (56, 348, "Signature du salarié"),
    ] {
        contenu.push_str(&format!(
            "BT /F1 11 Tf {x} {y} Td {} Tj ET\n",
            chaine(libelle)
        ));
    }
    let flux = document.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
    let helvetica = |nom: &str| {
        dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => nom.to_string(), "Encoding" => "WinAnsiEncoding" }
    };
    document.objects.insert(
        page,
        Object::Dictionary(dictionary! {
            "Type" => "Page", "Parent" => arbre, "Contents" => flux, "Annots" => annotations,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! { "Font" => dictionary! {
                "F1" => helvetica("Helvetica"), "F2" => helvetica("Helvetica-Bold"),
            } },
        }),
    );
    document.objects.insert(
        arbre,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1 }),
    );
    let formulaire = document.add_object(dictionary! {
        "Fields" => champs,
        "DA" => texte("/Helv 0 Tf 0 g"),
        "DR" => dictionary! { "Font" => dictionary! {
            "Helv" => helvetica("Helvetica"),
            "ZaDb" => dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "ZapfDingbats" },
        } },
    });
    let catalogue = document.add_object(
        dictionary! { "Type" => "Catalog", "Pages" => arbre, "AcroForm" => formulaire },
    );
    document.trailer.set("Root", catalogue);
    crate::formulaires::completer_apparences(&mut document);
    let mut octets = Vec::new();
    document.save_to(&mut octets).unwrap();
    octets
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
