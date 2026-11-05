//! Opérations sur les PDF : fusionner, séparer, assembler des pages choisies (ordre, rotation). Les
//! pages sont recopiées telles quelles (contenu, polices, images, couleurs CMJN) : rien n'est recalculé.

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet};

use lopdf::{Dictionary, Document, LoadOptions, Object, ObjectId, dictionary};

use crate::erreur::Erreur;

/// Attributs qu'une page peut hériter de ses parents dans l'arbre des pages.
const HERITES: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// Charge un PDF ; un PDF chiffré sans mot de passe d'ouverture (simples restrictions) est
/// déchiffré, un PDF qui demande un mot de passe est refusé avec un message clair.
pub(crate) fn charger(octets: &[u8], nom: &str) -> Result<Document, Erreur> {
    let mut document = Document::load_mem_with_options(octets, LoadOptions::with_password("")).map_err(|erreur| match erreur {
        lopdf::Error::InvalidPassword => Erreur::simple(format!(
            "« {nom} » est protégé par un mot de passe : retirez-le d'abord avec l'outil « Mot de passe »."
        )),
        autre => Erreur::simple(format!("« {nom} » n'a pas pu être lu : {autre}")),
    })?;
    aplatir_heritage(&mut document);
    Ok(document)
}

/// Recopie dans chaque page les attributs hérités de ses parents : la page peut changer d'arbre.
fn aplatir_heritage(document: &mut Document) {
    for page in document.get_pages().into_values() {
        let mut ajouts = Vec::new();
        let Ok(dictionnaire) = document.get_dictionary(page) else {
            continue;
        };
        for cle in HERITES {
            if dictionnaire.has(cle) {
                continue;
            }
            let mut parent = dictionnaire
                .get(b"Parent")
                .and_then(Object::as_reference)
                .ok();
            while let Some(id) = parent {
                let Ok(noeud) = document.get_dictionary(id) else {
                    break;
                };
                if let Ok(valeur) = noeud.get(cle) {
                    ajouts.push((cle, valeur.clone()));
                    break;
                }
                parent = noeud.get(b"Parent").and_then(Object::as_reference).ok();
            }
        }
        if let Ok(dictionnaire) = document.get_dictionary_mut(page) {
            for (cle, valeur) in ajouts {
                dictionnaire.set(cle, valeur);
            }
        }
    }
}

/// Objet désigné par une référence, ou l'objet lui-même.
fn resoudre<'a>(document: &'a Document, objet: &'a Object) -> Option<&'a Object> {
    match objet {
        Object::Reference(id) => document.get_object(*id).ok(),
        autre => Some(autre),
    }
}

/// Dictionnaire rangé sous `cle`, directement ou par référence.
pub(crate) fn dictionnaire<'a>(
    document: &'a Document,
    parent: &'a Dictionary,
    cle: &[u8],
) -> Option<&'a Dictionary> {
    resoudre(document, parent.get(cle).ok()?)?.as_dict().ok()
}

/// Contenu d'une page : ses flux de contenu décodés, mis bout à bout ; `None` si l'un d'eux ne
/// peut pas être décodé. Un élément qui n'est pas un flux n'affiche rien : il est ignoré.
pub(crate) fn contenu_page(document: &Document, page: &Dictionary) -> Option<Vec<u8>> {
    let elements: Vec<&Object> = match page
        .get(b"Contents")
        .ok()
        .and_then(|contenu| resoudre(document, contenu))
    {
        Some(Object::Array(elements)) => elements.iter().collect(),
        Some(element) => vec![element],
        None => Vec::new(),
    };
    let mut contenu = Vec::new();
    for flux in elements
        .into_iter()
        .filter_map(|element| resoudre(document, element)?.as_stream().ok())
    {
        contenu.extend_from_slice(&flux.decompressed_content().ok()?);
        contenu.push(b'\n');
    }
    Some(contenu)
}

/// Page visée par un lien interne à destination explicite, s'il y en a une.
fn page_visee(document: &Document, annotation: &Dictionary) -> Option<ObjectId> {
    let destination = match annotation.get(b"Dest") {
        Ok(destination) => destination,
        Err(_) => {
            let action = resoudre(document, annotation.get(b"A").ok()?)?
                .as_dict()
                .ok()?;
            if action.get(b"S").ok()?.as_name().ok()? != b"GoTo" {
                return None;
            }
            action.get(b"D").ok()?
        }
    };
    match resoudre(document, destination)? {
        Object::Array(destination) => destination.first()?.as_reference().ok(),
        _ => None,
    }
}

/// Retire les liens internes vers une page absente du nouveau PDF : sinon, la page visée (et tout
/// son ancien document, par son arbre de pages) resterait cachée dans le fichier.
fn retirer_liens_sortants(document: &mut Document, pages: &[ObjectId]) {
    let gardees: HashSet<ObjectId> = pages.iter().copied().collect();
    for page in pages {
        let Some(Object::Array(annotations)) = document
            .get_dictionary(*page)
            .ok()
            .and_then(|page| page.get(b"Annots").ok())
            .and_then(|annotations| resoudre(document, annotations))
        else {
            continue;
        };
        let restantes: Vec<Object> = annotations
            .iter()
            .filter(|annotation| {
                let dictionnaire = resoudre(document, annotation).and_then(|a| a.as_dict().ok());
                dictionnaire
                    .and_then(|a| page_visee(document, a))
                    .is_none_or(|cible| gardees.contains(&cible))
            })
            .cloned()
            .collect();
        if restantes.len() == annotations.len() {
            continue;
        }
        if let Ok(dictionnaire) = document.get_dictionary_mut(*page) {
            if restantes.is_empty() {
                dictionnaire.remove(b"Annots");
            } else {
                dictionnaire.set("Annots", restantes);
            }
        }
    }
}

/// Nouveau PDF fait des `pages` (dans l'ordre) prises parmi `objets` ; les objets que plus rien
/// n'utilise (anciens arbres de pages, ressources des pages écartées) sont retirés.
pub(crate) fn assembler(
    objets: BTreeMap<ObjectId, Object>,
    pages: &[ObjectId],
    max_id: u32,
) -> Result<Vec<u8>, Erreur> {
    let mut document = Document::with_version("1.7");
    document.objects = objets;
    document.max_id = max_id;
    retirer_liens_sortants(&mut document, pages);
    let arbre = document.new_object_id();
    for page in pages {
        if let Ok(dictionnaire) = document.get_dictionary_mut(*page) {
            dictionnaire.set("Parent", arbre);
        }
    }
    let enfants: Vec<Object> = pages.iter().map(|page| Object::Reference(*page)).collect();
    document.objects.insert(
        arbre,
        Object::Dictionary(
            dictionary! { "Type" => "Pages", "Kids" => enfants, "Count" => pages.len() as i64 },
        ),
    );
    let catalogue = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => arbre });
    document.trailer = Dictionary::new();
    document.trailer.set("Root", catalogue);
    document.prune_objects();
    document.renumber_objects();
    document.compress();
    let mut octets = Vec::new();
    document
        .save_to(&mut octets)
        .map_err(|erreur| Erreur::simple(format!("Le PDF n'a pas pu être écrit : {erreur}")))?;
    Ok(octets)
}

/// Le document, compressé, prêt à être enregistré.
pub(crate) fn ecrire(mut document: Document) -> Result<Vec<u8>, Erreur> {
    document.compress();
    let mut octets = Vec::new();
    document
        .save_to(&mut octets)
        .map_err(|erreur| Erreur::simple(format!("Le PDF n'a pas pu être écrit : {erreur}")))?;
    Ok(octets)
}

/// Nombre de pages d'un PDF.
pub fn nombre_de_pages(octets: &[u8], nom: &str) -> Result<usize, Erreur> {
    Ok(charger(octets, nom)?.get_pages().len())
}

/// Objets de plusieurs PDF réunis, avec les pages de chacun ; les numéros d'objets ne se
/// chevauchent pas d'un fichier à l'autre.
struct Sources {
    objets: BTreeMap<ObjectId, Object>,
    pages: Vec<Vec<ObjectId>>,
    max_id: u32,
}

fn charger_tous<T: AsRef<[u8]>>(sources: &[(String, T)]) -> Result<Sources, Erreur> {
    let mut reunies = Sources {
        objets: BTreeMap::new(),
        pages: Vec::new(),
        max_id: 1,
    };
    for (nom, octets) in sources {
        let mut document = charger(octets.as_ref(), nom)?;
        document.renumber_objects_with(reunies.max_id);
        reunies.max_id = document.max_id + 1;
        reunies
            .pages
            .push(document.get_pages().into_values().collect());
        reunies.objets.extend(document.objects);
    }
    Ok(reunies)
}

/// Un seul PDF fait de tous les PDF donnés (nom, contenu), dans l'ordre.
pub fn fusionner<T: AsRef<[u8]>>(sources: &[(String, T)]) -> Result<Vec<u8>, Erreur> {
    if sources.is_empty() {
        return Err(Erreur::simple("Aucun PDF à fusionner."));
    }
    let reunies = charger_tous(sources)?;
    let pages: Vec<ObjectId> = reunies.pages.concat();
    assembler(reunies.objets, &pages, reunies.max_id)
}

/// Une page du PDF à produire : la page `page` (à partir de 0) de la source `source`, tournée de
/// `rotation` degrés de plus (sens horaire, multiple de 90).
#[derive(Debug, Clone, Copy)]
pub struct PageAssemblee {
    pub source: usize,
    pub page: usize,
    pub rotation: i64,
}

/// Ajoute `degres` à la rotation d'une page.
fn pivoter(objets: &mut BTreeMap<ObjectId, Object>, page: ObjectId, degres: i64) {
    if degres.rem_euclid(360) == 0 {
        return;
    }
    let actuelle = match objets.get(&page).and_then(|page| page.as_dict().ok()) {
        Some(page) => match page.get(b"Rotate") {
            Ok(Object::Reference(id)) => objets.get(id).and_then(|r| r.as_i64().ok()),
            Ok(rotation) => rotation.as_i64().ok(),
            Err(_) => None,
        },
        None => None,
    };
    if let Some(Object::Dictionary(page)) = objets.get_mut(&page) {
        page.set("Rotate", (actuelle.unwrap_or(0) + degres).rem_euclid(360));
    }
}

/// Un PDF fait des pages choisies parmi les sources (nom, contenu), dans l'ordre donné ; une même
/// page peut revenir plusieurs fois.
pub fn assembler_pages<T: AsRef<[u8]>>(
    sources: &[(String, T)],
    pages: &[PageAssemblee],
) -> Result<Vec<u8>, Erreur> {
    if pages.is_empty() {
        return Err(Erreur::simple("Aucune page à enregistrer."));
    }
    let Sources {
        mut objets,
        pages: pages_des_sources,
        mut max_id,
    } = charger_tous(sources)?;
    // Dictionnaire de chaque page avant toute rotation : une page reprise une seconde fois en est
    // une copie (contenu partagé), qui ne doit pas hériter de la rotation de la première.
    let mut originaux: HashMap<ObjectId, Object> = HashMap::new();
    let mut choisies = Vec::with_capacity(pages.len());
    for choix in pages {
        let page = pages_des_sources
            .get(choix.source)
            .and_then(|pages| pages.get(choix.page))
            .copied()
            .ok_or_else(|| Erreur::simple(format!("La page {} n'existe pas.", choix.page + 1)))?;
        let page = match originaux.entry(page) {
            Entry::Vacant(place) => {
                place.insert(objets[&page].clone());
                page
            }
            Entry::Occupied(original) => {
                let id = (max_id, 0);
                max_id += 1;
                objets.insert(id, original.get().clone());
                id
            }
        };
        pivoter(&mut objets, page, choix.rotation);
        choisies.push(page);
    }
    assembler(objets, &choisies, max_id)
}

/// Un PDF par groupe de `pages_par_fichier` pages (le dernier peut être plus court).
pub fn separer(octets: &[u8], nom: &str, pages_par_fichier: usize) -> Result<Vec<Vec<u8>>, Erreur> {
    let document = charger(octets, nom)?;
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    pages
        .chunks(pages_par_fichier.max(1))
        .map(|groupe| assembler(objets_des_pages(&document, groupe), groupe, document.max_id))
        .collect()
}

/// Un PDF par groupe de pages (numérotées à partir de 0, dans l'ordre voulu) : le fichier n'est
/// lu qu'une fois, même pour des centaines de bulletins.
pub fn extraire(octets: &[u8], nom: &str, groupes: &[Vec<usize>]) -> Result<Vec<Vec<u8>>, Erreur> {
    let document = charger(octets, nom)?;
    let pages: Vec<ObjectId> = document.get_pages().into_values().collect();
    groupes
        .iter()
        .map(|groupe| {
            let choisies = groupe
                .iter()
                .map(|&i| {
                    pages.get(i).copied().ok_or_else(|| {
                        Erreur::simple(format!("La page {} n'existe pas dans « {nom} ».", i + 1))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            assembler(
                objets_des_pages(&document, &choisies),
                &choisies,
                document.max_id,
            )
        })
        .collect()
}

/// Objets dont dépendent ces pages (contenus, ressources, annotations…), sans remonter à l'arbre
/// des pages : un petit PDF de quelques pages d'un grand ne recopie pas tout le fichier.
fn objets_des_pages(document: &Document, pages: &[ObjectId]) -> BTreeMap<ObjectId, Object> {
    let mut objets = BTreeMap::new();
    let mut a_voir: Vec<ObjectId> = pages.to_vec();
    while let Some(id) = a_voir.pop() {
        if objets.contains_key(&id) {
            continue;
        }
        if let Ok(objet) = document.get_object(id) {
            references(objet, &mut a_voir);
            objets.insert(id, objet.clone());
        }
    }
    objets
}

/// Références contenues dans un objet ; le parent d'une page (l'arbre des pages) est laissé de côté.
fn references(objet: &Object, a_voir: &mut Vec<ObjectId>) {
    match objet {
        Object::Reference(id) => a_voir.push(*id),
        Object::Array(liste) => liste.iter().for_each(|o| references(o, a_voir)),
        Object::Dictionary(dict) => references_du_dictionnaire(dict, a_voir),
        Object::Stream(flux) => references_du_dictionnaire(&flux.dict, a_voir),
        _ => {}
    }
}

fn references_du_dictionnaire(dict: &Dictionary, a_voir: &mut Vec<ObjectId>) {
    let page = matches!(
        dict.get(b"Type").and_then(Object::as_name),
        Ok(b"Page" | b"Pages")
    );
    for (cle, valeur) in dict.iter() {
        if !(page && cle.as_slice() == b"Parent") {
            references(valeur, a_voir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::essais::{pdf_avec_lien, pdf_herite};

    fn pages_et_textes(octets: &[u8]) -> (usize, Vec<String>) {
        let document = Document::load_mem(octets).unwrap();
        let pages = document.get_pages();
        let textes = pages
            .keys()
            .map(|numero| {
                document
                    .extract_text(&[*numero])
                    .unwrap()
                    .trim()
                    .to_string()
            })
            .collect();
        for page in pages.values() {
            // Taille et ressources recopiées dans chaque page.
            let dictionnaire = document.get_dictionary(*page).unwrap();
            assert!(dictionnaire.has(b"MediaBox") && dictionnaire.has(b"Resources"));
        }
        (pages.len(), textes)
    }

    #[test]
    fn fusionne_dans_l_ordre() {
        let sources = [
            ("a.pdf".to_string(), pdf_herite(2, "A")),
            ("b.pdf".to_string(), pdf_herite(3, "B")),
        ];
        let fusion = fusionner(&sources).unwrap();
        assert_eq!(
            pages_et_textes(&fusion),
            (
                5,
                vec![
                    "A 1".into(),
                    "A 2".into(),
                    "B 1".into(),
                    "B 2".into(),
                    "B 3".into()
                ]
            )
        );
        assert_eq!(nombre_de_pages(&fusion, "fusion.pdf").unwrap(), 5);
    }

    #[test]
    fn separe_par_groupes_de_pages() {
        let parties = separer(&pdf_herite(5, "P"), "p.pdf", 2).unwrap();
        let resultats: Vec<_> = parties
            .iter()
            .map(|partie| pages_et_textes(partie))
            .collect();
        assert_eq!(resultats[0], (2, vec!["P 1".into(), "P 2".into()]));
        assert_eq!(resultats[2], (1, vec!["P 5".into()]));
        assert_eq!(resultats.len(), 3);
        // Une partie ne garde pas les pages des autres.
        assert!(parties[2].len() < parties[0].len());
    }

    /// Rotation enregistrée de chaque page, dans l'ordre.
    fn rotations(octets: &[u8]) -> Vec<i64> {
        let document = Document::load_mem(octets).unwrap();
        document
            .get_pages()
            .values()
            .map(|page| {
                let page = document.get_dictionary(*page).unwrap();
                page.get(b"Rotate").and_then(Object::as_i64).unwrap_or(0)
            })
            .collect()
    }

    #[test]
    fn assemble_des_pages_choisies_tournees_et_repetees() {
        let sources = [
            ("a.pdf".to_string(), pdf_herite(2, "A")),
            ("b.pdf".to_string(), pdf_herite(2, "B")),
        ];
        let choix = |source, page, rotation| PageAssemblee {
            source,
            page,
            rotation,
        };
        let pdf = assembler_pages(
            &sources,
            &[
                choix(1, 1, 90),
                choix(0, 0, 0),
                choix(1, 1, -90),
                choix(0, 1, 540),
            ],
        )
        .unwrap();
        assert_eq!(
            pages_et_textes(&pdf),
            (
                4,
                vec!["B 2".into(), "A 1".into(), "B 2".into(), "A 2".into()]
            )
        );
        // La page reprise deux fois est une copie : chacune garde sa propre rotation.
        assert_eq!(rotations(&pdf), [90, 0, 270, 180]);
        // Une page qui n'existe pas est signalée.
        let erreur = assembler_pages(&sources, &[choix(0, 5, 0)]).unwrap_err();
        assert!(erreur.message.contains("page 6"));
    }

    #[test]
    fn retire_les_liens_vers_les_pages_ecartees() {
        let source = [("liens.pdf".to_string(), pdf_avec_lien(3, "L"))];
        let premiere = PageAssemblee {
            source: 0,
            page: 0,
            rotation: 0,
        };
        let pdf = assembler_pages(&source, &[premiere]).unwrap();
        let document = Document::load_mem(&pdf).unwrap();
        // Le lien menait à la page 3, absente : il disparaît, et la page 3 n'est pas gardée en cachette.
        let page = document.get_pages()[&1];
        assert!(!document.get_dictionary(page).unwrap().has(b"Annots"));
        let pages_cachees = document
            .objects
            .values()
            .filter(|objet| {
                objet
                    .as_dict()
                    .is_ok_and(|d| d.get(b"Type").and_then(Object::as_name).ok() == Some(b"Page"))
            })
            .count();
        assert_eq!(pages_cachees, 1);
        // Le lien reste quand sa page cible fait partie du nouveau PDF.
        let derniere = PageAssemblee {
            page: 2,
            ..premiere
        };
        let pdf = assembler_pages(&source, &[premiere, derniere]).unwrap();
        let document = Document::load_mem(&pdf).unwrap();
        let page = document.get_pages()[&1];
        assert!(document.get_dictionary(page).unwrap().has(b"Annots"));
    }

    #[test]
    fn refuse_un_fichier_qui_n_est_pas_un_pdf() {
        let erreur = nombre_de_pages(b"pas un PDF", "faux.pdf").unwrap_err();
        assert!(erreur.message.contains("faux.pdf"));
    }
}
