//! Caviardage : des zones de pages passent au noir, et ce qui se trouve dessous est vraiment
//! supprimé du fichier — caractères du texte (y compris le texte invisible d'une page numérisée),
//! dessins entièrement couverts, pixels des images, annotations. Une page qui contient sous une
//! zone ce qui ne se nettoie pas ainsi (formulaire PDF, image dans un format particulier) est
//! remplacée par son image, zones noircies. Le document est reconstruit avec ses seules pages :
//! ni signets, ni métadonnées, ni champs de formulaire ne gardent l'information.

use std::collections::HashMap;
use std::sync::Arc;

use image::ExtendedColorType;
use image::codecs::jpeg::JpegEncoder;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, StringFormat, dictionary};
use serde::{Deserialize, Serialize};

use crate::erreur::Erreur;
use crate::geometrie::{Cadre, Matrice, boite, resoudre};
use crate::pdf::{assembler, charger, contenu_page, dictionnaire};
use crate::pixels::Pixels;
use crate::police;
use crate::rendu;

/// Zone à noircir, en fractions de la page affichée, depuis son coin en haut à gauche.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Zone {
    pub x: f64,
    pub y: f64,
    pub largeur: f64,
    pub hauteur: f64,
}

/// Les zones d'une page (à partir de 0).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageACaviarder {
    pub page: usize,
    pub zones: Vec<Zone>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModeCaviardage {
    /// Seul ce qui est sous les zones disparaît ; le reste de la page est gardé tel quel.
    Texte,
    /// Chaque page caviardée devient une image : le plus sûr.
    Image,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rapport {
    /// Pages (à partir de 1) remplacées par leur image.
    pub pages_en_image: Vec<usize>,
}

/// Rectangle dans le repère de la page : x0, y0, x1, y1.
type Rect = [f64; 4];

/// Résolution des pages remplacées par leur image.
const RESOLUTION: u32 = 200;
/// Entrées d'une page qui peuvent garder une trace de son contenu.
const TRACES: [&[u8]; 3] = [b"Thumb", b"PieceInfo", b"Metadata"];

fn intersecte(a: &Rect, b: &Rect) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

fn contient(grand: &Rect, petit: &Rect) -> bool {
    grand[0] <= petit[0] && grand[1] <= petit[1] && petit[2] <= grand[2] && petit[3] <= grand[3]
}

/// Zones dans le repère de la page (elles restent des rectangles : les rotations vont par quarts de tour).
fn zones_page(cadre: &Cadre, zones: &[Zone]) -> Vec<Rect> {
    let (largeur, hauteur) = cadre.taille_vue();
    let depuis_vue = cadre.depuis_vue();
    zones
        .iter()
        .map(|zone| {
            let (x0, x1) = (zone.x * largeur, (zone.x + zone.largeur) * largeur);
            let (y0, y1) = (
                (1.0 - zone.y - zone.hauteur) * hauteur,
                (1.0 - zone.y) * hauteur,
            );
            boite(depuis_vue, &[(x0, y0), (x1, y0), (x0, y1), (x1, y1)])
        })
        .collect()
}

fn nombre(document: &Document, objet: &Object) -> Option<f64> {
    resoudre(document, objet).as_float().ok().map(f64::from)
}

/// Ce qu'il faut d'une police pour situer ses caractères : octets par code, largeurs, hauteur.
struct Police {
    octets_par_code: usize,
    /// Largeurs en millièmes de la taille.
    largeurs: HashMap<u32, f64>,
    defaut: Option<f64>,
    /// Police standard sans largeurs déclarées (Helvetica, Courier…).
    standard: Option<String>,
    /// Facteur des largeurs (polices Type 3 : matrice de la police).
    facteur: f64,
    montee: f64,
    descente: f64,
}

impl Police {
    fn largeur(&self, code: u32) -> Option<f64> {
        self.largeurs
            .get(&code)
            .copied()
            .or_else(|| {
                let nom = self.standard.as_deref()?;
                police::largeur_standard(nom, u8::try_from(code).ok()?)
            })
            .or(self.defaut)
            .map(|largeur| largeur * self.facteur)
    }

    fn de(document: &Document, police: &Dictionary) -> Police {
        let nom = |cle: &[u8]| {
            police
                .get(cle)
                .ok()
                .and_then(|objet| resoudre(document, objet).as_name().ok())
                .map(|nom| String::from_utf8_lossy(nom).into_owned())
        };
        let hauteurs = |descripteur: Option<&Dictionary>| {
            let valeur = |cle: &[u8]| {
                descripteur
                    .and_then(|d| d.get(cle).ok())
                    .and_then(|v| nombre(document, v))
            };
            (
                valeur(b"Ascent").filter(|v| *v > 0.0).unwrap_or(800.0),
                valeur(b"Descent").filter(|v| *v < 0.0).unwrap_or(-200.0),
            )
        };
        if nom(b"Subtype").as_deref() == Some("Type0") {
            // Police composée : codes de deux octets, largeurs dans la police descendante (/W).
            let descendante = police
                .get(b"DescendantFonts")
                .ok()
                .and_then(|d| resoudre(document, d).as_array().ok())
                .and_then(|d| d.first())
                .and_then(|d| resoudre(document, d).as_dict().ok());
            let mut largeurs = HashMap::new();
            if let Some(w) = descendante
                .and_then(|d| d.get(b"W").ok())
                .and_then(|w| resoudre(document, w).as_array().ok())
            {
                let mut i = 0;
                while let Some(premier) = w.get(i).and_then(|v| nombre(document, v)) {
                    match w.get(i + 1).map(|v| resoudre(document, v)) {
                        Some(Object::Array(liste)) => {
                            for (rang, largeur) in liste.iter().enumerate() {
                                if let Some(largeur) = nombre(document, largeur) {
                                    largeurs.insert(premier as u32 + rang as u32, largeur);
                                }
                            }
                            i += 2;
                        }
                        Some(dernier) => {
                            let (Some(dernier), Some(largeur)) = (
                                nombre(document, dernier),
                                w.get(i + 2).and_then(|v| nombre(document, v)),
                            ) else {
                                break;
                            };
                            for code in
                                premier as u32..=(dernier as u32).min(premier as u32 + 65_535)
                            {
                                largeurs.insert(code, largeur);
                            }
                            i += 3;
                        }
                        None => break,
                    }
                }
            }
            let defaut = descendante
                .and_then(|d| d.get(b"DW").ok())
                .and_then(|v| nombre(document, v))
                .unwrap_or(1000.0);
            let (montee, descente) =
                hauteurs(descendante.and_then(|d| dictionnaire(document, d, b"FontDescriptor")));
            return Police {
                octets_par_code: 2,
                largeurs,
                defaut: Some(defaut),
                standard: None,
                facteur: 1.0,
                montee,
                descente,
            };
        }
        let premier = police
            .get(b"FirstChar")
            .ok()
            .and_then(|v| nombre(document, v))
            .unwrap_or(0.0) as u32;
        let mut largeurs = HashMap::new();
        if let Some(liste) = police
            .get(b"Widths")
            .ok()
            .and_then(|w| resoudre(document, w).as_array().ok())
        {
            for (rang, largeur) in liste.iter().enumerate() {
                if let Some(largeur) = nombre(document, largeur) {
                    largeurs.insert(premier + rang as u32, largeur);
                }
            }
        }
        let descripteur = dictionnaire(document, police, b"FontDescriptor");
        let manquante = descripteur
            .and_then(|d| d.get(b"MissingWidth").ok())
            .and_then(|v| nombre(document, v));
        // Type 3 : largeurs exprimées dans le repère de la police (matrice souvent 0,001).
        let facteur = if nom(b"Subtype").as_deref() == Some("Type3") {
            police
                .get(b"FontMatrix")
                .ok()
                .and_then(|m| resoudre(document, m).as_array().ok())
                .and_then(|m| m.first())
                .and_then(|m| nombre(document, m))
                .map_or(1.0, |m| m * 1000.0)
        } else {
            1.0
        };
        let (montee, descente) = hauteurs(descripteur);
        Police {
            octets_par_code: 1,
            standard: if largeurs.is_empty() {
                nom(b"BaseFont")
            } else {
                None
            },
            largeurs,
            defaut: manquante,
            facteur,
            montee,
            descente,
        }
    }
}

/// État graphique et de texte, sauvegardé par « q » et restauré par « Q ».
#[derive(Clone)]
struct Etat {
    ctm: Matrice,
    police: Option<Vec<u8>>,
    taille: f64,
    espacement: f64,
    espacement_mots: f64,
    echelle: f64,
    interligne: f64,
    elevation: f64,
}

impl Default for Etat {
    fn default() -> Self {
        Etat {
            ctm: Matrice::IDENTITE,
            police: None,
            taille: 0.0,
            espacement: 0.0,
            espacement_mots: 0.0,
            echelle: 1.0,
            interligne: 0.0,
            elevation: 0.0,
        }
    }
}

/// Ce que devient un texte affiché (Tj, TJ…).
enum Affichage {
    Inchange,
    /// Morceaux d'un TJ : les caractères couverts sont remplacés par un décalage de même largeur.
    Remplace(Vec<Object>),
    /// Largeur d'un caractère inconnue : on ne saurait garder la place du texte restant.
    Impossible,
}

/// Parcourt un texte affiché, fait avancer la matrice de texte comme le lecteur, et retire les
/// caractères dont la boîte touche une zone.
fn afficher(
    elements: &[Object],
    police: &Police,
    etat: &Etat,
    tm: &mut Matrice,
    zones: &[Rect],
) -> Affichage {
    let (taille, echelle) = (etat.taille, etat.echelle);
    let mut nouveaux = Vec::new();
    let mut morceau: Vec<u8> = Vec::new();
    let mut decalage = 0.0;
    let mut retire = false;
    let ranger = |nouveaux: &mut Vec<Object>, morceau: &mut Vec<u8>| {
        if !morceau.is_empty() {
            nouveaux.push(Object::String(
                std::mem::take(morceau),
                StringFormat::Hexadecimal,
            ));
        }
    };
    for element in elements {
        match element {
            Object::String(octets, _) => {
                for octets_code in octets.chunks(police.octets_par_code) {
                    let code = octets_code
                        .iter()
                        .fold(0u32, |code, &octet| (code << 8) | u32::from(octet));
                    let largeur = police.largeur(code);
                    let espace = if police.octets_par_code == 1 && code == 32 {
                        etat.espacement_mots
                    } else {
                        0.0
                    };
                    let largeur_glyphe = largeur.unwrap_or(500.0) / 1000.0 * taille * echelle;
                    let avance =
                        (largeur.unwrap_or(500.0) / 1000.0 * taille + etat.espacement + espace)
                            * echelle;
                    let (bas, haut) = (
                        etat.elevation + police.descente / 1000.0 * taille,
                        etat.elevation + police.montee / 1000.0 * taille,
                    );
                    let cadre = boite(
                        tm.puis(etat.ctm),
                        &[
                            (0.0, bas),
                            (largeur_glyphe, bas),
                            (0.0, haut),
                            (largeur_glyphe, haut),
                        ],
                    );
                    if zones.iter().any(|zone| intersecte(zone, &cadre)) {
                        if largeur.is_none() || taille * echelle == 0.0 {
                            return Affichage::Impossible;
                        }
                        retire = true;
                        ranger(&mut nouveaux, &mut morceau);
                        decalage -= avance * 1000.0 / (taille * echelle);
                    } else {
                        if decalage != 0.0 {
                            ranger(&mut nouveaux, &mut morceau);
                            nouveaux.push(Object::Real(decalage as f32));
                            decalage = 0.0;
                        }
                        morceau.extend_from_slice(octets_code);
                    }
                    *tm = Matrice::translation(avance, 0.0).puis(*tm);
                }
            }
            Object::Integer(_) | Object::Real(_) => {
                let n = f64::from(element.as_float().unwrap_or(0.0));
                *tm = Matrice::translation(-n / 1000.0 * taille * echelle, 0.0).puis(*tm);
                ranger(&mut nouveaux, &mut morceau);
                decalage += n;
            }
            _ => {}
        }
    }
    if !retire {
        return Affichage::Inchange;
    }
    ranger(&mut nouveaux, &mut morceau);
    if decalage != 0.0 {
        nouveaux.push(Object::Real(decalage as f32));
    }
    Affichage::Remplace(nouveaux)
}

/// Nettoie une page : retire ce qui est sous les zones, puis les noircit. `None` si c'est
/// impossible (la page n'a alors pas été modifiée).
fn nettoyer_page(document: &mut Document, page: ObjectId, zones: &[Rect]) -> Option<()> {
    let page_dict = document.get_dictionary(page).ok()?.clone();
    let mut ressources = dictionnaire(document, &page_dict, b"Resources")
        .cloned()
        .unwrap_or_default();
    let polices_page = dictionnaire(document, &ressources, b"Font")
        .cloned()
        .unwrap_or_default();
    let mut objets = dictionnaire(document, &ressources, b"XObject")
        .cloned()
        .unwrap_or_default();
    let operations = Content::decode(&contenu_page(document, &page_dict)?)
        .ok()?
        .operations;
    let mut polices: HashMap<Vec<u8>, Police> = HashMap::new();
    let mut pile: Vec<Etat> = Vec::new();
    let mut etat = Etat::default();
    let (mut tm, mut tlm) = (Matrice::IDENTITE, Matrice::IDENTITE);
    let mut chemin: Vec<(f64, f64)> = Vec::new();
    let mut images: HashMap<Vec<u8>, Pixels> = HashMap::new();
    let mut sortie = Vec::with_capacity(operations.len() + 2);
    sortie.push(Operation::new("q", Vec::new()));
    for operation in operations {
        let o = &operation.operands;
        let n = |i: usize| {
            o.get(i)
                .and_then(|v| v.as_float().ok())
                .map_or(0.0, f64::from)
        };
        let matrice = || Matrice([n(0), n(1), n(2), n(3), n(4), n(5)]);
        match operation.operator.as_str() {
            "q" => pile.push(etat.clone()),
            "Q" => {
                // Un « Q » de trop est ignoré, comme le font les lecteurs.
                if let Some(precedent) = pile.pop() {
                    etat = precedent;
                }
            }
            "cm" => etat.ctm = matrice().puis(etat.ctm),
            "BT" => (tm, tlm) = (Matrice::IDENTITE, Matrice::IDENTITE),
            "Tf" => {
                etat.police = o.first().and_then(|p| p.as_name().ok()).map(<[u8]>::to_vec);
                etat.taille = n(1);
            }
            "Tc" => etat.espacement = n(0),
            "Tw" => etat.espacement_mots = n(0),
            "Tz" => etat.echelle = n(0) / 100.0,
            "TL" => etat.interligne = n(0),
            "Ts" => etat.elevation = n(0),
            "Td" | "TD" => {
                if operation.operator == "TD" {
                    etat.interligne = -n(1);
                }
                tlm = Matrice::translation(n(0), n(1)).puis(tlm);
                tm = tlm;
            }
            "Tm" => (tm, tlm) = (matrice(), matrice()),
            "T*" => {
                tlm = Matrice::translation(0.0, -etat.interligne).puis(tlm);
                tm = tlm;
            }
            "Tj" | "TJ" | "'" | "\"" => {
                let mut prefixe = Vec::new();
                if operation.operator == "\"" {
                    etat.espacement_mots = n(0);
                    etat.espacement = n(1);
                    prefixe.push(Operation::new("Tw", vec![o.first()?.clone()]));
                    prefixe.push(Operation::new("Tc", vec![o.get(1)?.clone()]));
                }
                if operation.operator == "'" || operation.operator == "\"" {
                    tlm = Matrice::translation(0.0, -etat.interligne).puis(tlm);
                    tm = tlm;
                    prefixe.push(Operation::new("T*", Vec::new()));
                }
                let elements: Vec<Object> = if operation.operator == "TJ" {
                    o.first()?.as_array().ok()?.clone()
                } else {
                    vec![o.last()?.clone()]
                };
                let nom = etat.police.clone()?;
                if !polices.contains_key(&nom) {
                    let dict = polices_page
                        .get(&nom)
                        .ok()
                        .and_then(|p| resoudre(document, p).as_dict().ok())?;
                    polices.insert(nom.clone(), Police::de(document, dict));
                }
                match afficher(&elements, &polices[&nom], &etat, &mut tm, zones) {
                    Affichage::Inchange => sortie.push(operation),
                    Affichage::Remplace(nouveaux) => {
                        sortie.extend(prefixe);
                        sortie.push(Operation::new("TJ", vec![Object::Array(nouveaux)]));
                    }
                    Affichage::Impossible => return None,
                }
                continue;
            }
            "Do" => {
                let nom = o.first().and_then(|n| n.as_name().ok()).map(<[u8]>::to_vec);
                if let Some(nom) = nom
                    && let Ok(reference) = objets.get(&nom)
                    && let Ok(flux) = resoudre(document, reference).as_stream()
                {
                    match flux.dict.get(b"Subtype").and_then(Object::as_name).ok() {
                        Some(b"Image") => {
                            let cadre =
                                boite(etat.ctm, &[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]);
                            if zones.iter().any(|zone| intersecte(zone, &cadre)) {
                                if !images.contains_key(&nom) {
                                    images.insert(nom.clone(), Pixels::de(document, flux)?);
                                }
                                images.get_mut(&nom)?.noircir(etat.ctm, zones);
                            }
                        }
                        Some(b"Form") => {
                            let bbox = flux
                                .dict
                                .get(b"BBox")
                                .ok()
                                .and_then(|b| crate::geometrie::rectangle(document, b))?;
                            let matrice_forme = flux
                                .dict
                                .get(b"Matrix")
                                .and_then(Object::as_array)
                                .ok()
                                .and_then(|m| Matrice::de_nombres(m))
                                .unwrap_or(Matrice::IDENTITE);
                            let cadre = boite(
                                matrice_forme.puis(etat.ctm),
                                &[
                                    (bbox[0], bbox[1]),
                                    (bbox[2], bbox[1]),
                                    (bbox[0], bbox[3]),
                                    (bbox[2], bbox[3]),
                                ],
                            );
                            // Le contenu d'un formulaire n'est pas analysé : la page passera en image.
                            if zones.iter().any(|zone| intersecte(zone, &cadre)) {
                                return None;
                            }
                        }
                        _ => {}
                    }
                }
            }
            "BI" => {
                let cadre = boite(etat.ctm, &[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]);
                if zones.iter().any(|zone| intersecte(zone, &cadre)) {
                    return None;
                }
            }
            "m" | "l" => chemin.push(etat.ctm.point(n(0), n(1))),
            "c" => chemin.extend([
                etat.ctm.point(n(0), n(1)),
                etat.ctm.point(n(2), n(3)),
                etat.ctm.point(n(4), n(5)),
            ]),
            "v" | "y" => chemin.extend([etat.ctm.point(n(0), n(1)), etat.ctm.point(n(2), n(3))]),
            "re" => {
                let (x, y, l, h) = (n(0), n(1), n(2), n(3));
                chemin.extend(
                    [(x, y), (x + l, y), (x, y + h), (x + l, y + h)]
                        .map(|(px, py)| etat.ctm.point(px, py)),
                );
            }
            "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" => {
                let etendue = boite(Matrice::IDENTITE, &chemin);
                let couvert =
                    !chemin.is_empty() && zones.iter().any(|zone| contient(zone, &etendue));
                chemin.clear();
                if couvert {
                    sortie.push(Operation::new("n", Vec::new()));
                    continue;
                }
            }
            "n" => chemin.clear(),
            // Les propriétés du contenu balisé peuvent contenir le texte (/ActualText, /Alt) : retirées.
            "BDC" => {
                sortie.push(Operation::new(
                    "BMC",
                    o.first().cloned().into_iter().collect(),
                ));
                continue;
            }
            "DP" => {
                sortie.push(Operation::new(
                    "MP",
                    o.first().cloned().into_iter().collect(),
                ));
                continue;
            }
            _ => {}
        }
        sortie.push(operation);
    }
    // Contenu d'origine fermé (ses « q » en trop ne déplacent pas les zones noires), puis les zones.
    sortie.push(Operation::new("Q", Vec::new()));
    let mut contenu = Content { operations: sortie }.encode().ok()?;
    for zone in zones {
        contenu.extend_from_slice(
            format!(
                "\nq 0 g {:.3} {:.3} {:.3} {:.3} re f Q",
                zone[0],
                zone[1],
                zone[2] - zone[0],
                zone[3] - zone[1]
            )
            .as_bytes(),
        );
    }
    // Tout a pu être analysé : la page est modifiée.
    for (nom, pixels) in images {
        objets.set(nom, document.add_object(pixels.en_flux()));
    }
    if !objets.is_empty() {
        ressources.set("XObject", objets);
    }
    let annotations: Vec<Object> = page_dict
        .get(b"Annots")
        .ok()
        .and_then(|a| resoudre(document, a).as_array().ok())
        .map(|liste| {
            liste
                .iter()
                .filter(|annotation| {
                    let cadre = resoudre(document, annotation)
                        .as_dict()
                        .ok()
                        .and_then(|a| a.get(b"Rect").ok())
                        .and_then(|r| crate::geometrie::rectangle(document, r));
                    cadre.is_none_or(|cadre| !zones.iter().any(|zone| intersecte(zone, &cadre)))
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let flux = document.add_object(Stream::new(Dictionary::new(), contenu));
    let page = document.get_dictionary_mut(page).ok()?;
    page.set("Contents", flux);
    page.set("Resources", ressources);
    if annotations.is_empty() {
        page.remove(b"Annots");
    } else {
        page.set("Annots", annotations);
    }
    for trace in TRACES {
        page.remove(trace);
    }
    Some(())
}

/// Remplace une page par son image, zones noircies.
fn page_en_image(
    document: &mut Document,
    page: ObjectId,
    (largeur, hauteur, mut pixels): (u32, u32, Vec<u8>),
    zones: &[Zone],
) -> Result<(), Erreur> {
    let (l, h) = (largeur as usize, hauteur as usize);
    for zone in zones {
        let colonnes = ((zone.x * l as f64).floor().max(0.0) as usize)
            ..(((zone.x + zone.largeur) * l as f64).ceil() as usize).min(l);
        let lignes = ((zone.y * h as f64).floor().max(0.0) as usize)
            ..(((zone.y + zone.hauteur) * h as f64).ceil() as usize).min(h);
        if colonnes.start >= colonnes.end {
            continue;
        }
        for ligne in lignes {
            pixels[(ligne * l + colonnes.start) * 3..(ligne * l + colonnes.end) * 3].fill(0);
        }
    }
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 90)
        .encode(&pixels, largeur, hauteur, ExtendedColorType::Rgb8)
        .map_err(|erreur| {
            Erreur::simple(format!(
                "L'image de la page n'a pas pu être créée : {erreur}"
            ))
        })?;
    let image = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image",
            "Width" => i64::from(largeur), "Height" => i64::from(hauteur),
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8, "Filter" => "DCTDecode",
        },
        jpeg,
    ));
    let page_dict = document
        .get_dictionary(page)
        .map_err(|_| Erreur::simple("Une page n'a pas pu être lue."))?
        .clone();
    let cadre = Cadre::de_page(document, &page_dict);
    let (vue_l, vue_h) = cadre.taille_vue();
    let contenu = format!(
        "q {} {} /AutomatPage Do Q",
        cadre.depuis_vue().operateur(),
        Matrice::echelle(vue_l, vue_h).operateur()
    );
    let flux = document.add_object(Stream::new(Dictionary::new(), contenu.into_bytes()));
    let page = document
        .get_dictionary_mut(page)
        .map_err(|_| Erreur::simple("Une page n'a pas pu être modifiée."))?;
    page.set("Contents", flux);
    page.set(
        "Resources",
        dictionary! { "XObject" => dictionary! { "AutomatPage" => image } },
    );
    page.remove(b"Annots");
    for trace in TRACES {
        page.remove(trace);
    }
    Ok(())
}

/// Le PDF caviardé, et les pages qu'il a fallu remplacer par leur image.
pub fn caviarder(
    octets: Arc<Vec<u8>>,
    nom: &str,
    pages: &[PageACaviarder],
    mode: ModeCaviardage,
) -> Result<(Vec<u8>, Rapport), Erreur> {
    let mut document = charger(&octets, nom)?;
    let toutes: Vec<ObjectId> = document.get_pages().into_values().collect();
    let mut rapport = Rapport::default();
    for choix in pages.iter().filter(|choix| !choix.zones.is_empty()) {
        let &page = toutes
            .get(choix.page)
            .ok_or_else(|| Erreur::simple(format!("La page {} n'existe pas.", choix.page + 1)))?;
        let page_dict = document.get_dictionary(page).map_err(|_| {
            Erreur::simple(format!("La page {} n'a pas pu être lue.", choix.page + 1))
        })?;
        let zones = zones_page(&Cadre::de_page(&document, page_dict), &choix.zones);
        let nettoyee =
            mode == ModeCaviardage::Texte && nettoyer_page(&mut document, page, &zones).is_some();
        if !nettoyee {
            let rendu = rendu::pixels_page(octets.clone(), nom, choix.page, RESOLUTION)?;
            page_en_image(&mut document, page, rendu, &choix.zones)?;
            rapport.pages_en_image.push(choix.page + 1);
        }
    }
    // Le document est reconstruit avec ses seules pages : catalogue et informations neufs.
    let max_id = document.max_id;
    Ok((assembler(document.objects, &toutes, max_id)?, rapport))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::pdf_herite;

    /// Zone donnée dans le repère d'une page A4 non tournée (points), en fractions de la page.
    fn zone_points(x0: f64, y0: f64, x1: f64, y1: f64) -> Zone {
        Zone {
            x: x0 / 595.0,
            y: (842.0 - y1) / 842.0,
            largeur: (x1 - x0) / 595.0,
            hauteur: (y1 - y0) / 842.0,
        }
    }

    /// Octets de toutes les chaînes du fichier : textes des contenus (pages et formulaires) et
    /// chaînes des dictionnaires (annotations, informations…).
    fn chaines(pdf: &[u8]) -> Vec<u8> {
        fn depuis(objet: &Object, sortie: &mut Vec<u8>) {
            match objet {
                Object::String(octets, _) => sortie.extend_from_slice(octets),
                Object::Array(liste) => liste.iter().for_each(|o| depuis(o, sortie)),
                Object::Dictionary(dict) => dict.iter().for_each(|(_, o)| depuis(o, sortie)),
                Object::Stream(flux) => {
                    flux.dict.iter().for_each(|(_, o)| depuis(o, sortie));
                    if let Ok(contenu) = Content::decode(
                        &flux
                            .decompressed_content()
                            .unwrap_or_else(|_| flux.content.clone()),
                    ) {
                        for operation in contenu.operations {
                            operation.operands.iter().for_each(|o| depuis(o, sortie));
                        }
                    }
                }
                _ => {}
            }
        }
        let document = Document::load_mem(pdf).unwrap();
        let mut sortie = Vec::new();
        document
            .objects
            .values()
            .for_each(|objet| depuis(objet, &mut sortie));
        document
            .trailer
            .iter()
            .for_each(|(_, objet)| depuis(objet, &mut sortie));
        sortie
    }

    fn contient_octets(meule: &[u8], aiguille: &[u8]) -> bool {
        meule
            .windows(aiguille.len())
            .any(|fenetre| fenetre == aiguille)
    }

    fn texte_page(pdf: &[u8]) -> String {
        let document = Document::load_mem(pdf).unwrap();
        document.extract_text(&[1]).unwrap_or_default()
    }

    /// Document d'une page, modifié par `modifier`.
    fn document_modifie(texte: &str, modifier: impl FnOnce(&mut Document, ObjectId)) -> Vec<u8> {
        // Chargé comme par l'application : les ressources héritées sont recopiées dans la page.
        let mut document = charger(&pdf_herite(1, texte), "essai.pdf").unwrap();
        let page = document.get_pages()[&1];
        modifier(&mut document, page);
        let mut octets = Vec::new();
        document.save_to(&mut octets).unwrap();
        octets
    }

    /// Ajoute un objet graphique (image, formulaire) aux ressources de la page.
    fn ajouter_objet(document: &mut Document, page: ObjectId, nom: &str, objet: ObjectId) {
        let dict = document.get_dictionary_mut(page).unwrap();
        let mut ressources = dict.get(b"Resources").unwrap().as_dict().unwrap().clone();
        ressources.set("XObject", dictionary! { nom => objet });
        dict.set("Resources", ressources);
    }

    fn ajouter_contenu(document: &mut Document, page: ObjectId, contenu: &str) {
        let ajout =
            document.add_object(Stream::new(Dictionary::new(), contenu.as_bytes().to_vec()));
        let dict = document.get_dictionary_mut(page).unwrap();
        let actuel = dict.get(b"Contents").unwrap().clone();
        dict.set("Contents", vec![actuel, Object::Reference(ajout)]);
    }

    fn caviarder_page(pdf: Vec<u8>, zones: Vec<Zone>, mode: ModeCaviardage) -> (Vec<u8>, Rapport) {
        caviarder(
            Arc::new(pdf),
            "essai.pdf",
            &[PageACaviarder { page: 0, zones }],
            mode,
        )
        .unwrap()
    }

    #[test]
    fn retire_le_texte_sous_la_zone_et_garde_le_reste_a_sa_place() {
        // « Secret 1 » en Helvetica 24 à (72, 720) : « Secret » occupe x 72 à 141.
        let origine = pdf_herite(1, "Secret");
        let (pdf, rapport) = caviarder_page(
            origine.clone(),
            vec![zone_points(70.0, 710.0, 145.0, 745.0)],
            ModeCaviardage::Texte,
        );
        assert!(rapport.pages_en_image.is_empty());
        let toutes = chaines(&pdf);
        assert!(!contient_octets(&toutes, b"Secret") && !contient_octets(&toutes, b"ecret"));
        assert!(texte_page(&pdf).contains('1'), "{}", texte_page(&pdf));
        // Le « 1 » qui reste n'a pas bougé ; la zone est noire.
        let rendu = |octets: Vec<u8>| rendu::pixels_page(Arc::new(octets), "e.pdf", 0, 72).unwrap();
        let ((l, _, avant), (_, _, apres)) = (rendu(origine), rendu(pdf));
        let somme = |pixels: &[u8], colonnes: std::ops::Range<usize>| -> u64 {
            (97..132)
                .flat_map(|y| colonnes.clone().map(move |x| (y * l as usize + x) * 3))
                .map(|i| u64::from(pixels[i]))
                .sum()
        };
        assert_eq!(somme(&avant, 148..164), somme(&apres, 148..164));
        assert!(somme(&apres, 75..140) < 100);
    }

    #[test]
    fn noircit_les_pixels_d_une_image_sous_la_zone() {
        let pdf = document_modifie("T", |document, page| {
            let image = document.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Image", "Width" => 10, "Height" => 10,
                    "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
                },
                vec![255; 300],
            ));
            ajouter_objet(document, page, "Im1", image);
            ajouter_contenu(document, page, "q 200 0 0 200 100 100 cm /Im1 Do Q");
        });
        // Moitié gauche de l'image (x 100 à 200).
        let (pdf, rapport) = caviarder_page(
            pdf,
            vec![zone_points(100.0, 100.0, 200.0, 300.0)],
            ModeCaviardage::Texte,
        );
        assert!(rapport.pages_en_image.is_empty());
        let document = Document::load_mem(&pdf).unwrap();
        let images: Vec<Vec<u8>> = document
            .objects
            .values()
            .filter_map(|o| o.as_stream().ok())
            .filter(|f| f.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image"))
            .map(|f| {
                f.decompressed_content()
                    .unwrap_or_else(|_| f.content.clone())
            })
            .collect();
        // Une seule image : la copie noircie ; l'originale, toute blanche, n'est plus dans le fichier.
        assert_eq!(images.len(), 1);
        let pixel = |x: usize, y: usize| images[0][(y * 10 + x) * 3];
        assert_eq!((pixel(2, 5), pixel(8, 5)), (0, 255));
    }

    #[test]
    fn remplace_par_son_image_une_page_dont_un_formulaire_est_sous_une_zone() {
        let pdf = document_modifie("Visible", |document, page| {
            let police = document.add_object(
                dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" },
            );
            let forme = document.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Form",
                    "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
                    "Resources" => dictionary! { "Font" => dictionary! { "F1" => police } },
                },
                b"BT /F1 12 Tf 10 50 Td (Formulaire) Tj ET".to_vec(),
            ));
            ajouter_objet(document, page, "Fm1", forme);
            ajouter_contenu(document, page, "q 1 0 0 1 300 300 cm /Fm1 Do Q");
        });
        let (pdf, rapport) = caviarder_page(
            pdf,
            vec![zone_points(300.0, 300.0, 400.0, 400.0)],
            ModeCaviardage::Texte,
        );
        assert_eq!(rapport.pages_en_image, [1]);
        let toutes = chaines(&pdf);
        assert!(!contient_octets(&toutes, b"Formulaire") && !contient_octets(&toutes, b"Visible"));
    }

    #[test]
    fn mode_image_remplace_toujours_la_page() {
        let (pdf, rapport) = caviarder_page(
            pdf_herite(1, "Secret"),
            vec![zone_points(70.0, 710.0, 145.0, 745.0)],
            ModeCaviardage::Image,
        );
        assert_eq!(rapport.pages_en_image, [1]);
        assert!(!contient_octets(&chaines(&pdf), b"Secret"));
        assert_eq!(Document::load_mem(&pdf).unwrap().get_pages().len(), 1);
    }

    #[test]
    fn caviarde_au_bon_endroit_sur_une_page_tournee() {
        let pdf = document_modifie("Secret", |document, page| {
            document.get_dictionary_mut(page).unwrap().set("Rotate", 90);
        });
        // La zone est donnée sur la page affichée : on la calcule depuis le repère de la page.
        let cadre = Cadre {
            boite: [0.0, 0.0, 595.0, 842.0],
            rotation: 90,
        };
        let r = boite(
            cadre.vers_vue(),
            &[(70.0, 710.0), (145.0, 710.0), (70.0, 745.0), (145.0, 745.0)],
        );
        let (l, h) = cadre.taille_vue();
        let zone = Zone {
            x: r[0] / l,
            y: (h - r[3]) / h,
            largeur: (r[2] - r[0]) / l,
            hauteur: (r[3] - r[1]) / h,
        };
        let (pdf, rapport) = caviarder_page(pdf, vec![zone], ModeCaviardage::Texte);
        assert!(rapport.pages_en_image.is_empty());
        assert!(!contient_octets(&chaines(&pdf), b"Secret"));
        assert!(texte_page(&pdf).contains('1'));
    }

    #[test]
    fn retire_les_annotations_touchees_et_les_informations_du_document() {
        let pdf = document_modifie("Secret", |document, page| {
            let note = document.add_object(dictionary! {
                "Type" => "Annot", "Subtype" => "Text",
                "Rect" => vec![80.into(), 715.into(), 100.into(), 735.into()],
                "Contents" => Object::string_literal("Secret de la note"),
            });
            document
                .get_dictionary_mut(page)
                .unwrap()
                .set("Annots", vec![Object::Reference(note)]);
            let informations = document
                .add_object(dictionary! { "Title" => Object::string_literal("Dossier Secret") });
            document.trailer.set("Info", informations);
        });
        let (pdf, _) = caviarder_page(
            pdf,
            vec![zone_points(70.0, 710.0, 145.0, 745.0)],
            ModeCaviardage::Texte,
        );
        assert!(!contient_octets(&chaines(&pdf), b"Secret"));
        assert!(!Document::load_mem(&pdf).unwrap().trailer.has(b"Info"));
    }
}
