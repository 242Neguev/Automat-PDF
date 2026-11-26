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

/// Un PDF d'une page A4 qui n'a que l'image de `pdf_herite(1, texte)`, comme un scan.
pub fn scan(texte: &str) -> Vec<u8> {
    let source = Arc::new(pdf_herite(1, texte));
    let (largeur, hauteur, rvb) = crate::rendu::pixels_page(source, "source.pdf", 0, 200).unwrap();
    let mut png = Vec::new();
    image::RgbImage::from_raw(largeur, hauteur, rvb)
        .unwrap()
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    crate::images::images_en_pdf(
        &[crate::images::ImageSource {
            nom: "scan.png".into(),
            octets: &png,
            rotation: 0,
            ppp: None,
        }],
        crate::images::OptionsImages {
            format: crate::images::FormatFeuille::A4,
            marge_mm: 0.0,
        },
    )
    .unwrap()
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

// ------------------------------------------------------------------------------- Signatures numériques

/// Écriture DER, pour fabriquer des certificats et des signatures d'essai.
pub mod der {
    pub fn tlv(etiquette: u8, contenu: &[u8]) -> Vec<u8> {
        let mut sortie = vec![etiquette];
        if contenu.len() < 0x80 {
            sortie.push(contenu.len() as u8);
        } else {
            let octets = contenu.len().to_be_bytes();
            let debut = octets.iter().position(|&o| o != 0).unwrap();
            sortie.push(0x80 | (octets.len() - debut) as u8);
            sortie.extend_from_slice(&octets[debut..]);
        }
        sortie.extend_from_slice(contenu);
        sortie
    }

    pub fn sequence(parties: &[&[u8]]) -> Vec<u8> {
        tlv(0x30, &parties.concat())
    }

    /// Un SET OF, ses éléments triés (DER).
    pub fn ensemble(parties: &[&[u8]]) -> Vec<u8> {
        let mut triees = parties.to_vec();
        triees.sort();
        tlv(0x31, &triees.concat())
    }

    /// Un entier positif (ses octets, en grand-boutiste).
    pub fn entier(octets: &[u8]) -> Vec<u8> {
        let mut chiffres: Vec<u8> = octets.iter().copied().skip_while(|&o| o == 0).collect();
        if chiffres.first().is_none_or(|&o| o & 0x80 != 0) {
            chiffres.insert(0, 0);
        }
        tlv(0x02, &chiffres)
    }

    pub fn petit(n: u64) -> Vec<u8> {
        entier(&n.to_be_bytes())
    }

    pub fn oid(texte: &str) -> Vec<u8> {
        let arcs: Vec<u64> = texte.split('.').map(|a| a.parse().unwrap()).collect();
        let mut contenu = Vec::new();
        for (rang, &arc) in std::iter::once(&(arcs[0] * 40 + arcs[1]))
            .chain(&arcs[2..])
            .enumerate()
        {
            let _ = rang;
            let mut septets = vec![(arc & 0x7f) as u8];
            let mut reste = arc >> 7;
            while reste > 0 {
                septets.push((reste & 0x7f) as u8 | 0x80);
                reste >>= 7;
            }
            septets.reverse();
            contenu.extend(septets);
        }
        tlv(0x06, &contenu)
    }

    pub fn octets(contenu: &[u8]) -> Vec<u8> {
        tlv(0x04, contenu)
    }

    pub fn bits(contenu: &[u8]) -> Vec<u8> {
        tlv(0x03, &[&[0u8][..], contenu].concat())
    }

    pub fn utf8(texte: &str) -> Vec<u8> {
        tlv(0x0c, texte.as_bytes())
    }

    pub fn nul() -> Vec<u8> {
        vec![0x05, 0x00]
    }

    /// `[n]`, construit (EXPLICIT, ou IMPLICIT d'une structure).
    pub fn contexte(n: u8, contenu: &[u8]) -> Vec<u8> {
        tlv(0xa0 | n, contenu)
    }

    fn chiffres(secondes: i64) -> String {
        crate::cms::iso(secondes)
            .chars()
            .filter(char::is_ascii_digit)
            .collect()
    }

    pub fn utc(secondes: i64) -> Vec<u8> {
        tlv(0x17, format!("{}Z", &chiffres(secondes)[2..]).as_bytes())
    }

    pub fn generalise(secondes: i64) -> Vec<u8> {
        tlv(0x18, format!("{}Z", chiffres(secondes)).as_bytes())
    }

    /// Un nom X.509 : nom commun, et organisation.
    pub fn nom(cn: &str, organisation: Option<&str>) -> Vec<u8> {
        let mut parties = vec![ensemble(&[&sequence(&[&oid("2.5.4.3"), &utf8(cn)])])];
        if let Some(organisation) = organisation {
            parties.push(ensemble(&[&sequence(&[
                &oid("2.5.4.10"),
                &utf8(organisation),
            ])]));
        }
        let references: Vec<&[u8]> = parties.iter().map(Vec::as_slice).collect();
        sequence(&references)
    }
}

/// Un hasard déterministe (SHA-256 d'une graine et d'un compteur) : des clés d'essai qui ne
/// changent pas d'une fois sur l'autre, sans dépendre du système.
pub struct Hasard {
    graine: Vec<u8>,
    compteur: u64,
}

impl Hasard {
    pub fn new(graine: &str) -> Self {
        Self {
            graine: graine.as_bytes().to_vec(),
            compteur: 0,
        }
    }
}

impl rsa::rand_core::TryRng for Hasard {
    type Error = std::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        let mut octets = [0; 4];
        self.try_fill_bytes(&mut octets)?;
        Ok(u32::from_le_bytes(octets))
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        let mut octets = [0; 8];
        self.try_fill_bytes(&mut octets)?;
        Ok(u64::from_le_bytes(octets))
    }

    fn try_fill_bytes(&mut self, sortie: &mut [u8]) -> Result<(), Self::Error> {
        use sha2::Digest;
        for morceau in sortie.chunks_mut(32) {
            let bloc = sha2::Sha256::new()
                .chain_update(&self.graine)
                .chain_update(self.compteur.to_le_bytes())
                .finalize();
            self.compteur += 1;
            morceau.copy_from_slice(&bloc[..morceau.len()]);
        }
        Ok(())
    }
}

impl rsa::rand_core::TryCryptoRng for Hasard {}

pub fn sha256(donnees: &[u8]) -> Vec<u8> {
    use sha2::Digest;
    sha2::Sha256::digest(donnees).to_vec()
}

/// Une clé d'essai : RSA (signature PKCS #1 v1.5 ou PSS) ou ECDSA P-256.
pub enum CleEssai {
    Rsa(Arc<rsa::PrivateKey>),
    RsaPss(Arc<rsa::PrivateKey>),
    P256(Box<p256::ecdsa::SigningKey>),
}

impl CleEssai {
    /// Une clé RSA 2048 bits, faite une fois par graine (c'est un peu long).
    pub fn rsa(graine: &str) -> Arc<rsa::PrivateKey> {
        static CLES: Mutex<Vec<(String, Arc<rsa::PrivateKey>)>> = Mutex::new(Vec::new());
        let mut cles = CLES.lock().unwrap();
        if let Some((_, cle)) = cles.iter().find(|(g, _)| g == graine) {
            return cle.clone();
        }
        let cle = Arc::new(rsa::PrivateKey::new(&mut Hasard::new(graine), 2048).unwrap());
        cles.push((graine.into(), cle.clone()));
        cle
    }

    pub fn p256(graine: &str) -> Self {
        Self::P256(Box::new(
            p256::ecdsa::SigningKey::from_slice(&sha256(graine.as_bytes())).unwrap(),
        ))
    }

    /// La clé publique (SubjectPublicKeyInfo).
    pub fn spki(&self) -> Vec<u8> {
        use rsa::traits::PublicKeyParts;
        match self {
            Self::Rsa(cle) | Self::RsaPss(cle) => {
                let publique = der::sequence(&[
                    &der::entier(&cle.n().to_be_bytes()),
                    &der::entier(&cle.e().to_be_bytes()),
                ]);
                der::sequence(&[
                    &der::sequence(&[&der::oid("1.2.840.113549.1.1.1"), &der::nul()]),
                    &der::bits(&publique),
                ])
            }
            Self::P256(cle) => der::sequence(&[
                &der::sequence(&[
                    &der::oid("1.2.840.10045.2.1"),
                    &der::oid("1.2.840.10045.3.1.7"),
                ]),
                &der::bits(&cle.verifying_key().to_sec1_bytes()),
            ]),
        }
    }

    /// L'algorithme de signature, avec SHA-256.
    pub fn algorithme(&self) -> Vec<u8> {
        match self {
            Self::Rsa(_) => der::sequence(&[&der::oid("1.2.840.113549.1.1.11"), &der::nul()]),
            Self::RsaPss(_) => {
                let sha256 = der::sequence(&[&der::oid("2.16.840.1.101.3.4.2.1"), &der::nul()]);
                let mgf1 = der::sequence(&[&der::oid("1.2.840.113549.1.1.8"), &sha256]);
                der::sequence(&[
                    &der::oid("1.2.840.113549.1.1.10"),
                    &der::sequence(&[
                        &der::contexte(0, &sha256),
                        &der::contexte(1, &mgf1),
                        &der::contexte(2, &der::petit(32)),
                    ]),
                ])
            }
            Self::P256(_) => der::sequence(&[&der::oid("1.2.840.10045.4.3.2")]),
        }
    }

    /// Signe l'empreinte SHA-256 `hache`.
    pub fn signer(&self, hache: &[u8]) -> Vec<u8> {
        use p256::ecdsa::signature::hazmat::PrehashSigner;
        let mut hasard = Hasard::new("signature");
        match self {
            Self::Rsa(cle) => cle
                .pkcs1v15_sign_prehash_with_rng::<sha2::Sha256, _>(&mut hasard, hache)
                .unwrap(),
            Self::RsaPss(cle) => cle
                .pss_sign_prehash_with_rng::<sha2::Sha256, _>(&mut hasard, hache)
                .unwrap(),
            Self::P256(cle) => {
                let signature: p256::ecdsa::Signature = cle.sign_prehash(hache).unwrap();
                signature.to_der().to_bytes().to_vec()
            }
        }
    }
}

/// Une identité d'essai : sa clé, son nom, son numéro de série et son certificat.
pub struct Identite {
    pub cle: CleEssai,
    pub nom: Vec<u8>,
    pub serie: Vec<u8>,
    pub certificat: Vec<u8>,
}

/// Le 1er janvier 2026 et le 1er janvier 2036, en secondes depuis 1970.
pub const DEBUT_2026: i64 = 1_767_225_600;
pub const DEBUT_2036: i64 = 2_082_758_400;

fn certificat_essai(
    sujet: &[u8],
    cle_sujet: &CleEssai,
    emetteur: &[u8],
    cle_emetteur: &CleEssai,
    serie: &[u8],
    autorite: bool,
) -> Vec<u8> {
    let mut parties = vec![
        der::contexte(0, &der::petit(2)),
        der::entier(serie),
        cle_emetteur.algorithme(),
        emetteur.to_vec(),
        der::sequence(&[&der::utc(DEBUT_2026), &der::utc(DEBUT_2036)]),
        sujet.to_vec(),
        cle_sujet.spki(),
    ];
    if autorite {
        let oui = [0x01, 0x01, 0xff];
        let contraintes = der::sequence(&[
            &der::oid("2.5.29.19"),
            &oui,
            &der::octets(&der::sequence(&[&oui])),
        ]);
        parties.push(der::contexte(3, &der::sequence(&[&contraintes])));
    }
    let references: Vec<&[u8]> = parties.iter().map(Vec::as_slice).collect();
    let tbs = der::sequence(&references);
    let signature = cle_emetteur.signer(&sha256(&tbs));
    der::sequence(&[&tbs, &cle_emetteur.algorithme(), &der::bits(&signature)])
}

impl Identite {
    /// Une autorité de certification d'essai (son certificat fait par elle-même).
    pub fn autorite(cle: CleEssai, nom: &str) -> Self {
        let nom = der::nom(nom, Some("Automat-PDF (essais)"));
        let serie = vec![1];
        let certificat = certificat_essai(&nom, &cle, &nom, &cle, &serie, true);
        Self {
            cle,
            nom,
            serie,
            certificat,
        }
    }

    /// Une identité dont le certificat est émis par `autorite`.
    pub fn emise_par(
        autorite: &Identite,
        cle: CleEssai,
        nom: &str,
        organisation: Option<&str>,
        serie: u64,
    ) -> Self {
        let nom = der::nom(nom, organisation);
        let serie = serie.to_be_bytes().to_vec();
        let certificat = certificat_essai(&nom, &cle, &autorite.nom, &autorite.cle, &serie, false);
        Self {
            cle,
            nom,
            serie,
            certificat,
        }
    }
}

const DATA: &str = "1.2.840.113549.1.7.1";
const TST_INFO: &str = "1.2.840.113549.1.9.16.1.4";

/// Une signature CMS de `donnees` (détachée), ou de `encapsule` (un jeton d'horodatage), par
/// `signataire`, les certificats `autres` joints ; `date` : attribut signingTime ; `horodatage` :
/// un jeton de la signature par cette autorité, à cette date.
pub fn cms_essai(
    signataire: &Identite,
    autres: &[&[u8]],
    donnees: &[&[u8]],
    encapsule: Option<&[u8]>,
    date: Option<i64>,
    horodatage: Option<(&Identite, i64)>,
) -> Vec<u8> {
    let type_contenu = if encapsule.is_some() { TST_INFO } else { DATA };
    let empreinte = match encapsule {
        Some(contenu) => sha256(contenu),
        None => sha256(&donnees.concat()),
    };
    let mut attributs = vec![
        der::sequence(&[
            &der::oid("1.2.840.113549.1.9.3"),
            &der::ensemble(&[&der::oid(type_contenu)]),
        ]),
        der::sequence(&[
            &der::oid("1.2.840.113549.1.9.4"),
            &der::ensemble(&[&der::octets(&empreinte)]),
        ]),
    ];
    if let Some(date) = date {
        attributs.push(der::sequence(&[
            &der::oid("1.2.840.113549.1.9.5"),
            &der::ensemble(&[&der::utc(date)]),
        ]));
    }
    let references: Vec<&[u8]> = attributs.iter().map(Vec::as_slice).collect();
    let ensemble = der::ensemble(&references);
    let signature = signataire.cle.signer(&sha256(&ensemble));
    let implicites = [&[0xa0][..], &ensemble[1..]].concat();
    let sha256_alg = der::sequence(&[&der::oid("2.16.840.1.101.3.4.2.1"), &der::nul()]);
    let mut info = vec![
        der::petit(1),
        der::sequence(&[
            &signataire_emetteur(signataire),
            &der::entier(&signataire.serie),
        ]),
        sha256_alg.clone(),
        implicites,
        signataire.cle.algorithme(),
        der::octets(&signature),
    ];
    if let Some((autorite, date)) = horodatage {
        let jeton = jeton_horodatage(autorite, &[&signature], date);
        info.push(der::tlv(
            0xa1,
            &der::sequence(&[
                &der::oid("1.2.840.113549.1.9.16.2.14"),
                &der::ensemble(&[&jeton]),
            ]),
        ));
    }
    let references: Vec<&[u8]> = info.iter().map(Vec::as_slice).collect();
    let info = der::sequence(&references);
    let mut certificats = signataire.certificat.clone();
    for autre in autres {
        certificats.extend_from_slice(autre);
    }
    let encapsule = match encapsule {
        Some(contenu) => der::sequence(&[
            &der::oid(type_contenu),
            &der::contexte(0, &der::octets(contenu)),
        ]),
        None => der::sequence(&[&der::oid(type_contenu)]),
    };
    let donnees_signees = der::sequence(&[
        &der::petit(1),
        &der::ensemble(&[&sha256_alg]),
        &encapsule,
        &der::contexte(0, &certificats),
        &der::ensemble(&[&info]),
    ]);
    der::sequence(&[
        &der::oid("1.2.840.113549.1.7.2"),
        &der::contexte(0, &donnees_signees),
    ])
}

/// L'émetteur du certificat de `identite` (le nom de son autorité), lu dans son certificat.
fn signataire_emetteur(identite: &Identite) -> Vec<u8> {
    let certificat = crate::cms::premier(&identite.certificat).unwrap();
    crate::cms::certificat(certificat)
        .unwrap()
        .emetteur_der
        .to_vec()
}

/// Un jeton d'horodatage (RFC 3161) de `donnees`, par `autorite`, à la date `date`.
pub fn jeton_horodatage(autorite: &Identite, donnees: &[&[u8]], date: i64) -> Vec<u8> {
    let sha256_alg = der::sequence(&[&der::oid("2.16.840.1.101.3.4.2.1"), &der::nul()]);
    let info = der::sequence(&[
        &der::petit(1),
        &der::oid("1.3.6.1.4.1.99999.1"),
        &der::sequence(&[&sha256_alg, &der::octets(&sha256(&donnees.concat()))]),
        &der::petit(7),
        &der::generalise(date),
    ]);
    cms_essai(autorite, &[], &[], Some(&info), None, None)
}

/// Ce qu'il faut pour un PDF signé d'essai.
pub struct PdfSigne<'a> {
    pub signataire: &'a Identite,
    /// Certificats joints en plus de celui du signataire (son autorité).
    pub autres: Vec<&'a [u8]>,
    pub sous_filtre: &'a str,
    /// Date de signature (secondes depuis 1970).
    pub date: i64,
    /// Horodatage de la signature : autorité et date.
    pub horodatage: Option<(&'a Identite, i64)>,
    /// Signature de certification : modifications permises (1, 2, 3).
    pub certification: Option<i64>,
    /// Horodatage du document (pas de signature : un jeton d'horodatage de ses octets).
    pub horodatage_du_document: bool,
}

/// Place réservée à la signature, en octets.
const PLACE_SIGNATURE: usize = 12_000;
/// Nombre écrit dans /ByteRange avant de connaître les vraies plages (dix chiffres).
const RESERVE: i64 = 9_999_999_999;

/// Un contrat d'une page, signé numériquement comme le ferait un logiciel de signature : un champ
/// de signature visible, sa signature (/ByteRange, /Contents) calculée sur le fichier écrit.
pub fn pdf_signe(options: &PdfSigne) -> Vec<u8> {
    use lopdf::{Stream, StringFormat};

    let texte = crate::annotations::texte_pdf;
    let mut document = Document::with_version("1.7");
    let arbre = document.new_object_id();
    let page = document.new_object_id();
    let police = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let contenu = document.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 22 Tf 72 760 Td (Contrat de prestation) Tj ET\nBT /F1 11 Tf 72 728 Td (Entre la societe Exemple et son client, pour l'annee 2027.) Tj ET\nBT /F1 11 Tf 72 150 Td (Signature du prestataire :) Tj ET".to_vec(),
    ));
    let signataire =
        crate::cms::certificat(crate::cms::premier(&options.signataire.certificat).unwrap())
            .unwrap()
            .sujet
            .lisible()
            .unwrap();
    let apparence = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 220.into(), 60.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => police } },
        },
        format!("0.2 0.3 0.6 rg BT /F1 10 Tf 8 36 Td (Signe numeriquement par {signataire}) Tj ET")
            .into_bytes(),
    ));
    let (type_, filtre) = if options.horodatage_du_document {
        ("DocTimeStamp", "ETSI.RFC3161")
    } else {
        ("Sig", options.sous_filtre)
    };
    let mut signature = dictionary! {
        "Type" => type_,
        "Filter" => "Adobe.PPKLite",
        "SubFilter" => Object::Name(filtre.as_bytes().to_vec()),
        "ByteRange" => vec![0.into(), RESERVE.into(), RESERVE.into(), RESERVE.into()],
        "Contents" => Object::String(vec![0; PLACE_SIGNATURE], StringFormat::Hexadecimal),
    };
    if !options.horodatage_du_document {
        signature.set("M", texte("D:20261126091200+01'00'"));
        signature.set("Name", texte("Jeanne Martin"));
        signature.set("Reason", texte("J'approuve ce contrat"));
        signature.set("Location", texte("Lyon"));
    }
    if let Some(permissions) = options.certification {
        signature.set(
            "Reference",
            vec![
                dictionary! {
                    "Type" => "SigRef", "TransformMethod" => "DocMDP",
                    "TransformParams" => dictionary! { "Type" => "TransformParams", "P" => permissions, "V" => "1.2" },
                }
                .into(),
            ],
        );
    }
    let id_signature = document.add_object(signature);
    let champ = document.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Widget", "FT" => "Sig", "T" => texte("Signature1"),
        "Rect" => vec![300.into(), 120.into(), 520.into(), 180.into()], "F" => 4, "P" => page,
        "AP" => dictionary! { "N" => apparence }, "V" => id_signature,
    });
    document.objects.insert(
        page,
        dictionary! {
            "Type" => "Page", "Parent" => arbre,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Contents" => contenu,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => police } },
            "Annots" => vec![champ.into()],
        }
        .into(),
    );
    document.objects.insert(
        arbre,
        dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }.into(),
    );
    let mut catalogue = dictionary! {
        "Type" => "Catalog", "Pages" => arbre,
        "AcroForm" => dictionary! { "Fields" => vec![champ.into()], "SigFlags" => 3 },
    };
    if options.certification.is_some() {
        catalogue.set("Perms", dictionary! { "DocMDP" => id_signature });
    }
    let racine = document.add_object(catalogue);
    document.trailer.set("Root", racine);
    let mut octets = Vec::new();
    document.save_to(&mut octets).unwrap();

    // Les plages signées : tout, sauf la place de la signature (« <000…> »).
    let place = [&b"<"[..], &vec![b'0'; 2 * PLACE_SIGNATURE], &b">"[..]].concat();
    let debut = octets
        .windows(place.len())
        .position(|f| f == place.as_slice())
        .unwrap();
    let fin = debut + place.len();
    let reserve = format!("{RESERVE} {RESERVE} {RESERVE}");
    let position = octets
        .windows(reserve.len())
        .position(|f| f == reserve.as_bytes())
        .unwrap();
    let plages = format!("{debut:<10} {fin:<10} {:<10}", octets.len() - fin);
    octets[position..position + reserve.len()].copy_from_slice(plages.as_bytes());
    let morceaux = [&octets[..debut], &octets[fin..]];
    let signature = if options.horodatage_du_document {
        jeton_horodatage(options.signataire, &morceaux, options.date)
    } else {
        cms_essai(
            options.signataire,
            &options.autres,
            &morceaux,
            None,
            Some(options.date),
            options.horodatage,
        )
    };
    let hexa: String = signature.iter().map(|o| format!("{o:02X}")).collect();
    assert!(hexa.len() <= 2 * PLACE_SIGNATURE, "signature trop grande");
    octets[debut + 1..debut + 1 + hexa.len()].copy_from_slice(hexa.as_bytes());
    octets
}
