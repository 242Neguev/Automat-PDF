//! Ce que fait chaque commande, sans dépendre de Tauri : l'application et les tests de bout en
//! bout appellent exactement le même code.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::caviardage::{self, ModeCaviardage, PageACaviarder, Rapport};
use crate::compression::{self, Bilan, Niveau};
use crate::confidentialite::{self, Confidentialite};
use crate::envoi::{BilanEnvoi, Message};
use crate::erreur::Erreur;
use crate::fichiers::{self, FichierAEcrire, Nature};
use crate::images::{self, ImageSource, OptionsImages};
use crate::imposition::{self, Disposition};
use crate::libreoffice::{self, Convertisseurs};
use crate::marques::{self, OptionsMarques, Placement};
use crate::memoire;
use crate::messagerie::{self, Compte};
use crate::office::{self, Conversions};
use crate::outlook::{self, ModeEnvoi};
use crate::ouverture;
use crate::pdf::{self, PageAssemblee};
use crate::rendu::{self, FormatImage, FormatPage};
use crate::securite::{self, OptionsProtection};
use crate::texte;

/// Nom d'un fichier (avec son extension) à partir de son chemin.
pub fn nom_de(chemin: &str) -> String {
    Path::new(chemin)
        .file_name()
        .map(|nom| nom.to_string_lossy().into_owned())
        .unwrap_or_else(|| chemin.to_string())
}

/// Nom d'un fichier sans son extension « .pdf ».
fn sans_extension(nom: &str) -> &str {
    nom.strip_suffix(".pdf")
        .or_else(|| nom.strip_suffix(".PDF"))
        .unwrap_or(nom)
}

/// Contenu brut d'une liste (Excel, CSV, ODS) ; elle est décodée dans l'interface.
pub fn lire_liste(chemin: &str) -> Result<Vec<u8>, Erreur> {
    fichiers::lire(chemin, Nature::Liste)
}

/// Nombre de pages d'un PDF.
pub fn infos_pdf(chemin: &str) -> Result<usize, Erreur> {
    pdf::nombre_de_pages(&fichiers::lire(chemin, Nature::Pdf)?, &nom_de(chemin))
}

/// Fusionne des PDF, dans l'ordre donné, dans le fichier `sortie`.
pub fn fusionner_pdf(chemins: &[String], sortie: &str) -> Result<(), Erreur> {
    let sources = chemins
        .iter()
        .map(|chemin| Ok((nom_de(chemin), fichiers::lire(chemin, Nature::Pdf)?)))
        .collect::<Result<Vec<_>, Erreur>>()?;
    fichiers::enregistrer(sortie, &pdf::fusionner(&sources)?)
}

/// Sépare un PDF en fichiers de `pages_par_fichier` pages, dans un nouveau dossier (chemin renvoyé).
/// Chaque fichier prend le nom donné par `noms`, sinon celui du PDF suivi d'un numéro.
pub fn separer_pdf(
    chemin: &str,
    pages_par_fichier: usize,
    noms: &[String],
    dossier: &str,
    sous_dossier: &str,
) -> Result<String, Erreur> {
    let nom = nom_de(chemin);
    let parties = pdf::separer(
        &fichiers::lire(chemin, Nature::Pdf)?,
        &nom,
        pages_par_fichier,
    )?;
    let base = sans_extension(&nom);
    let chiffres = parties.len().to_string().len().max(3);
    let a_ecrire: Vec<FichierAEcrire> = parties
        .into_iter()
        .enumerate()
        .map(|(i, octets)| {
            let nom = noms
                .get(i)
                .filter(|nom| !nom.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| format!("{base} - {:0chiffres$}", i + 1));
            FichierAEcrire {
                nom: format!("{nom}.pdf"),
                texte: None,
                octets: Some(octets),
            }
        })
        .collect();
    fichiers::ecrire(dossier, sous_dossier, &a_ecrire)
}

/// Une page choisie dans l'organiseur : un PDF, sa page (à partir de 0) et la rotation ajoutée
/// (degrés, sens horaire).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageChoisie {
    pub chemin: String,
    pub page: usize,
    #[serde(default)]
    pub rotation: i64,
}

/// Taille de chaque page d'un PDF, pour placer les vignettes avant leur rendu.
pub fn pages_pdf(chemin: &str) -> Result<Vec<FormatPage>, Erreur> {
    rendu::formats(memoire::pdf(chemin)?, &nom_de(chemin))
}

/// Image JPEG d'une page (vignette ou aperçu), large de `largeur` pixels.
pub fn vignette_pdf(chemin: &str, page: usize, largeur: u32) -> Result<Vec<u8>, Erreur> {
    rendu::image_page(
        memoire::pdf(chemin)?,
        &nom_de(chemin),
        page,
        largeur.clamp(32, 2400),
        82,
    )
}

/// Enregistre dans `sortie` un PDF fait des pages choisies, dans l'ordre (organiser, extraire,
/// intercaler).
pub fn enregistrer_pages(pages: &[PageChoisie], sortie: &str) -> Result<(), Erreur> {
    let mut chemins: Vec<&str> = Vec::new();
    let assemblees: Vec<PageAssemblee> = pages
        .iter()
        .map(|choix| {
            let source = chemins
                .iter()
                .position(|chemin| *chemin == choix.chemin)
                .unwrap_or_else(|| {
                    chemins.push(&choix.chemin);
                    chemins.len() - 1
                });
            PageAssemblee {
                source,
                page: choix.page,
                rotation: choix.rotation,
            }
        })
        .collect();
    let contenus: Vec<Arc<Vec<u8>>> = chemins
        .iter()
        .map(|chemin| memoire::pdf(chemin))
        .collect::<Result<_, _>>()?;
    let sources: Vec<(String, &[u8])> = chemins
        .iter()
        .zip(&contenus)
        .map(|(chemin, contenu)| (nom_de(chemin), contenu.as_slice()))
        .collect();
    fichiers::enregistrer(sortie, &pdf::assembler_pages(&sources, &assemblees)?)
}

/// Une image choisie : son fichier et la rotation ajoutée (degrés, sens horaire).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageChoisie {
    pub chemin: String,
    #[serde(default)]
    pub rotation: i64,
}

/// Contenu brut d'une image (pour l'afficher telle quelle, transparence comprise).
pub fn lire_image(chemin: &str) -> Result<Vec<u8>, Erreur> {
    fichiers::lire(chemin, Nature::Image)
}

/// Vignette JPEG d'une image (redressée), large de `largeur` pixels au plus.
pub fn vignette_image(chemin: &str, largeur: u32) -> Result<Vec<u8>, Erreur> {
    images::vignette(
        &fichiers::lire(chemin, Nature::Image)?,
        &nom_de(chemin),
        largeur.clamp(32, 1200),
    )
}

/// Enregistre dans `sortie` un PDF fait des images choisies, une par page, dans l'ordre.
pub fn images_en_pdf(
    choisies: &[ImageChoisie],
    options: OptionsImages,
    sortie: &str,
) -> Result<(), Erreur> {
    let contenus: Vec<Vec<u8>> = choisies
        .iter()
        .map(|image| fichiers::lire(&image.chemin, Nature::Image))
        .collect::<Result<_, _>>()?;
    let sources: Vec<ImageSource> = choisies
        .iter()
        .zip(&contenus)
        .map(|(image, octets)| ImageSource {
            nom: nom_de(&image.chemin),
            octets,
            rotation: image.rotation,
        })
        .collect();
    fichiers::enregistrer(sortie, &images::images_en_pdf(&sources, options)?)
}

/// Pages d'un PDF en images, dans un nouveau dossier (chemin renvoyé). `pages` : numéros à partir
/// de 0 ; vide : toutes les pages. Plusieurs pages sont rendues en même temps, et chaque image est
/// écrite dès qu'elle est prête.
pub fn pdf_en_images(
    chemin: &str,
    pages: &[usize],
    format: FormatImage,
    dpi: u32,
    dossier: &str,
) -> Result<String, Erreur> {
    let octets = memoire::pdf(chemin)?;
    let nom = nom_de(chemin);
    let total = rendu::formats(octets.clone(), &nom)?.len();
    let pages: Vec<usize> = if pages.is_empty() {
        (0..total).collect()
    } else {
        pages.to_vec()
    };
    if let Some(page) = pages.iter().find(|&&page| page >= total) {
        return Err(Erreur::simple(format!(
            "La page {} n'existe pas : le PDF en compte {total}.",
            page + 1
        )));
    }
    let base = sans_extension(&nom).to_string();
    let sous_dossier = format!("{base} - images");
    let extension = match format {
        FormatImage::Png => "png",
        FormatImage::Jpeg => "jpg",
    };
    let chiffres = total.to_string().len().max(3);
    let fils = std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .clamp(1, pages.len().max(1));
    let resultats = std::thread::scope(|portee| {
        let travaux: Vec<_> = pages
            .chunks(pages.len().div_ceil(fils).max(1))
            .map(|morceau| {
                let (octets, nom, base, sous_dossier) =
                    (octets.clone(), &nom, &base, &sous_dossier);
                portee.spawn(move || -> Result<Option<String>, Erreur> {
                    let mut cible = None;
                    for &page in morceau {
                        let image = rendu::exporter_page(octets.clone(), nom, page, dpi, format)?;
                        let fichier = FichierAEcrire {
                            nom: format!("{base} - page {:0chiffres$}.{extension}", page + 1),
                            texte: None,
                            octets: Some(image),
                        };
                        cible = Some(fichiers::ecrire(dossier, sous_dossier, &[fichier])?);
                    }
                    Ok(cible)
                })
            })
            .collect();
        travaux
            .into_iter()
            .map(|travail| {
                travail
                    .join()
                    .unwrap_or_else(|_| Err(Erreur::simple("Le rendu des pages a été interrompu.")))
            })
            .collect::<Result<Vec<_>, _>>()
    })?;
    resultats
        .into_iter()
        .flatten()
        .next()
        .ok_or_else(|| Erreur::simple("Aucune page à exporter."))
}

/// Enregistre dans `sortie` le PDF mis en page : 2 ou 4 pages par feuille, livret ou format A4.
pub fn imposer_pdf(chemin: &str, disposition: Disposition, sortie: &str) -> Result<(), Erreur> {
    let octets = memoire::pdf(chemin)?;
    fichiers::enregistrer(
        sortie,
        &imposition::imposer(&octets, &nom_de(chemin), disposition)?,
    )
}

/// Enregistre dans `sortie` le PDF avec numéros de page et tampon sur les pages choisies.
pub fn marquer_pdf(chemin: &str, options: &OptionsMarques, sortie: &str) -> Result<(), Erreur> {
    let octets = memoire::pdf(chemin)?;
    let mut document = pdf::charger(&octets, &nom_de(chemin))?;
    marques::marquer(&mut document, options)?;
    fichiers::enregistrer(sortie, &pdf::ecrire(document)?)
}

/// Aperçu (JPEG) d'une page avec son numéro et son tampon, tels qu'ils seront dans le document.
pub fn apercu_marques(
    chemin: &str,
    page: usize,
    options: &OptionsMarques,
    largeur: u32,
) -> Result<Vec<u8>, Erreur> {
    let octets = memoire::pdf(chemin)?;
    let nom = nom_de(chemin);
    let total = rendu::formats(octets.clone(), &nom)?.len();
    let numero = marques::numeros(options, total)
        .into_iter()
        .find(|(choisie, _)| *choisie == page)
        .map(|(_, numero)| numero);
    let seule = pdf::assembler_pages(
        &[(nom.clone(), octets.as_slice())],
        &[PageAssemblee {
            source: 0,
            page,
            rotation: 0,
        }],
    )?;
    let mut document = pdf::charger(&seule, &nom)?;
    if let (Some(numero), Some(&id)) = (numero, document.get_pages().values().next()) {
        marques::marquer_page(&mut document, id, options, numero)?;
    }
    rendu::image_page(
        Arc::new(pdf::ecrire(document)?),
        &nom,
        0,
        largeur.clamp(64, 1600),
        85,
    )
}

/// Enregistre dans `sortie` le PDF avec une image (signature, cachet) posée sur les pages choisies.
pub fn apposer_image(
    chemin: &str,
    image: &str,
    placement: Placement,
    pages: &[usize],
    sortie: &str,
) -> Result<(), Erreur> {
    let octets = memoire::pdf(chemin)?;
    let mut document = pdf::charger(&octets, &nom_de(chemin))?;
    let image = images::image_decodee(&fichiers::lire(image, Nature::Image)?, &nom_de(image))?;
    marques::apposer(&mut document, image, placement, pages)?;
    fichiers::enregistrer(sortie, &pdf::ecrire(document)?)
}

/// Enregistre dans `sortie` le PDF caviardé ; renvoie les pages remplacées par leur image.
pub fn caviarder_pdf(
    chemin: &str,
    pages: &[PageACaviarder],
    mode: ModeCaviardage,
    sortie: &str,
) -> Result<Rapport, Erreur> {
    let (pdf, rapport) =
        caviardage::caviarder(memoire::pdf(chemin)?, &nom_de(chemin), pages, mode)?;
    fichiers::enregistrer(sortie, &pdf)?;
    Ok(rapport)
}

/// Taille d'un fichier, en octets.
pub fn taille_fichier(chemin: &str) -> Result<u64, Erreur> {
    std::fs::metadata(chemin)
        .map(|infos| infos.len())
        .map_err(|erreur| Erreur::simple(format!("Impossible de lire « {chemin} » : {erreur}")))
}

/// Enregistre dans `sortie` le PDF protégé par mot de passe (AES-256).
pub fn proteger_pdf(chemin: &str, options: &OptionsProtection, sortie: &str) -> Result<(), Erreur> {
    let document = pdf::charger(&memoire::pdf(chemin)?, &nom_de(chemin))?;
    fichiers::enregistrer(sortie, &securite::proteger(document, options)?)
}

/// Enregistre dans `sortie` le PDF sans son mot de passe.
pub fn deverrouiller_pdf(chemin: &str, mot_de_passe: &str, sortie: &str) -> Result<(), Erreur> {
    let octets = fichiers::lire(chemin, Nature::Pdf)?;
    fichiers::enregistrer(
        sortie,
        &securite::deverrouiller(&octets, &nom_de(chemin), mot_de_passe)?,
    )
}

/// Enregistre dans `sortie` le PDF allégé ; renvoie ce que la compression a fait.
pub fn compresser_pdf(chemin: &str, niveau: Niveau, sortie: &str) -> Result<Bilan, Erreur> {
    let (octets, bilan) = compression::compresser(&memoire::pdf(chemin)?, &nom_de(chemin), niveau)?;
    fichiers::enregistrer(sortie, &octets)?;
    Ok(bilan)
}

/// Texte de chaque page, pour y repérer à qui elle appartient.
pub fn textes_pdf(chemin: &str) -> Result<Vec<String>, Erreur> {
    let nom = nom_de(chemin);
    let octets = memoire::pdf(chemin)?;
    // Lu d'abord par lopdf : message clair pour un PDF protégé par mot de passe ou abîmé.
    pdf::nombre_de_pages(&octets, &nom)?;
    texte::textes(octets, &nom)
}

/// Le fichier d'une personne : ses pages (comptées à partir de 0), et son mot de passe s'il faut
/// le protéger.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FichierPersonne {
    pub nom: String,
    pub pages: Vec<usize>,
    #[serde(default)]
    pub mot_de_passe: Option<String>,
}

/// Fichiers créés : le dossier, et le chemin de chaque fichier dans l'ordre demandé.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Separation {
    pub dossier: String,
    pub fichiers: Vec<String>,
}

/// Un PDF par personne, dans le dossier `sous_dossier` de `dossier` ; protégé par son mot de
/// passe s'il en a un (on peut l'imprimer, pas en copier le texte ni le modifier).
pub fn separer_par_personne(
    chemin: &str,
    personnes: &[FichierPersonne],
    dossier: &str,
    sous_dossier: &str,
) -> Result<Separation, Erreur> {
    let groupes: Vec<Vec<usize>> = personnes.iter().map(|p| p.pages.clone()).collect();
    let parties = pdf::extraire(&memoire::pdf(chemin)?, &nom_de(chemin), &groupes)?;
    let a_ecrire = personnes
        .iter()
        .zip(parties)
        .map(|(personne, octets)| {
            let octets = match personne.mot_de_passe.as_deref().filter(|m| !m.is_empty()) {
                Some(mot_de_passe) => {
                    let options = OptionsProtection {
                        mot_de_passe: mot_de_passe.to_string(),
                        proprietaire: String::new(),
                        impression: true,
                        copie: false,
                        modification: false,
                    };
                    securite::proteger(pdf::charger(&octets, &personne.nom)?, &options)?
                }
                None => octets,
            };
            Ok(FichierAEcrire {
                nom: format!("{}.pdf", personne.nom),
                texte: None,
                octets: Some(octets),
            })
        })
        .collect::<Result<Vec<_>, Erreur>>()?;
    let dossier = fichiers::ecrire(dossier, sous_dossier, &a_ecrire)?;
    let fichiers = a_ecrire
        .iter()
        .map(|fichier| fichiers::chemin_dans(&dossier, &fichier.nom).unwrap_or_default())
        .collect();
    Ok(Separation { dossier, fichiers })
}

/// Prépare dans Outlook (Brouillons) ou envoie les messages ; `progression` reçoit l'avancement.
pub fn envoyer_par_outlook(
    messages: &[Message],
    mode: ModeEnvoi,
    progression: impl Fn(usize, usize),
) -> Result<BilanEnvoi, Erreur> {
    outlook::envoyer(messages, mode, progression)
}

/// Envoie les messages par la messagerie de l'utilisateur (SMTP), sans Outlook ; `progression`
/// reçoit l'avancement.
pub fn envoyer_par_messagerie(
    compte: &Compte,
    messages: &[Message],
    progression: impl Fn(usize, usize),
) -> Result<BilanEnvoi, Erreur> {
    messagerie::envoyer(compte, messages, progression)
}

/// Vérifie la connexion à la messagerie et l'identification, sans rien envoyer.
pub fn tester_messagerie(compte: &Compte, mot_de_passe: Option<&str>) -> Result<(), Erreur> {
    messagerie::tester(compte, mot_de_passe)
}

/// Un mot de passe de messagerie est-il gardé par le système ?
pub fn mot_de_passe_messagerie_enregistre() -> Result<bool, Erreur> {
    messagerie::mot_de_passe_enregistre()
}

/// Garde le mot de passe de la messagerie dans le coffre du système ; vide : l'oublie.
pub fn enregistrer_mot_de_passe_messagerie(mot_de_passe: &str) -> Result<(), Erreur> {
    messagerie::enregistrer_mot_de_passe(mot_de_passe)
}

/// Ce qui permet de convertir les documents Office sur ce PC : Microsoft Office, LibreOffice, et
/// si LibreOffice peut être installé d'ici.
pub fn convertisseurs_office() -> Convertisseurs {
    libreoffice::convertisseurs()
}

/// Installe LibreOffice par son installateur officiel (winget, Windows).
pub fn installer_libreoffice() -> Result<(), Erreur> {
    libreoffice::installer().map(|_| ())
}

/// Ouvre la page de téléchargement officielle de LibreOffice.
pub fn ouvrir_page_libreoffice() -> Result<(), Erreur> {
    libreoffice::ouvrir_page()
}

/// Ouvre un fichier créé par l'application (PDF, document Word, image) dans l'application
/// habituelle, ou un dossier.
pub fn ouvrir_fichier(chemin: &str) -> Result<(), Erreur> {
    ouverture::ouvrir(chemin)
}

/// Affiche un fichier dans son dossier, sélectionné ; un dossier est simplement ouvert.
pub fn montrer_dans_dossier(chemin: &str) -> Result<(), Erreur> {
    ouverture::montrer(chemin)
}

/// Page de la dernière version publiée : la mise à jour s'y télécharge à la main quand
/// l'application n'a pas pu l'installer elle-même.
const PAGE_DES_VERSIONS: &str = "https://github.com/242Neguev/Automat-PDF/releases/latest";

/// Ouvre la page de la dernière version publiée dans le navigateur.
pub fn ouvrir_page_des_versions() -> Result<(), Erreur> {
    ouverture::ouvrir_page(PAGE_DES_VERSIONS)
}

/// Fichier des réglages de confidentialité : recherche des mises à jour, statistiques anonymes.
const CONFIDENTIALITE: &str = "confidentialite.json";

fn reglages_de_confidentialite(dossier: &Path) -> Result<confidentialite::Reglages, Erreur> {
    lire_reglages(dossier, CONFIDENTIALITE).map(|contenu| confidentialite::Reglages::lire(&contenu))
}

/// Réglages de confidentialité, tels que l'interface les présente.
pub fn confidentialite(dossier: &Path) -> Result<Confidentialite, Erreur> {
    Ok(reglages_de_confidentialite(dossier)?.etat(confidentialite::adresse().is_some()))
}

/// Relit les réglages de confidentialité, les modifie et les enregistre s'ils ont changé, un
/// appel à la fois : un signal qui aboutit pendant qu'on change un réglage ne l'efface pas.
fn modifier_confidentialite(
    dossier: &Path,
    modifier: impl FnOnce(&mut confidentialite::Reglages) -> Result<(), Erreur>,
) -> Result<confidentialite::Reglages, Erreur> {
    static ECRITURE: Mutex<()> = Mutex::new(());
    let _une_a_la_fois = ECRITURE.lock().unwrap_or_else(PoisonError::into_inner);
    let lus = reglages_de_confidentialite(dossier)?;
    let mut reglages = lus.clone();
    modifier(&mut reglages)?;
    if reglages != lus {
        enregistrer_reglages(dossier, CONFIDENTIALITE, &reglages.texte())?;
    }
    Ok(reglages)
}

/// Change les réglages de confidentialité donnés ; les autres restent tels quels.
pub fn regler_confidentialite(
    dossier: &Path,
    mises_a_jour: Option<bool>,
    statistiques: Option<bool>,
) -> Result<Confidentialite, Erreur> {
    let reglages = modifier_confidentialite(dossier, |reglages| {
        reglages.regler(mises_a_jour, statistiques)
    })?;
    Ok(reglages.etat(confidentialite::adresse().is_some()))
}

/// Envoie le signal d'ouverture si l'utilisateur a accepté les statistiques anonymes, une seule
/// fois par lancement, même si l'interface se recharge ; `true` si un signal est parti.
pub fn signaler_ouverture(dossier: &Path, langue: &str) -> Result<bool, Erreur> {
    static ENVOYE: AtomicBool = AtomicBool::new(false);
    let Some(adresse) = confidentialite::adresse() else {
        return Ok(false);
    };
    if reglages_de_confidentialite(dossier)?.statistiques != Some(true)
        || ENVOYE.swap(true, Ordering::SeqCst)
    {
        return Ok(false);
    }
    signaler(adresse, dossier, langue, &confidentialite::systeme())
}

/// Envoie le signal à `adresse` : le premier reçu annonce l'installation, les suivants des
/// ouvertures ; s'il n'arrive pas, l'installation sera annoncée au prochain lancement.
fn signaler(adresse: &str, dossier: &Path, langue: &str, systeme: &str) -> Result<bool, Erreur> {
    // Numéro écarté (fichier modifié à la main) : un nouveau est tiré.
    let reglages = modifier_confidentialite(dossier, |reglages| reglages.regler(None, None))?;
    let Some(signal) = reglages.signal(langue, systeme) else {
        return Ok(false);
    };
    confidentialite::envoyer(adresse, &signal)?;
    if signal.evenement == "installation" {
        // Relu après l'envoi : un réglage changé entre-temps est gardé.
        modifier_confidentialite(dossier, |reglages| {
            if reglages.installation.as_deref() == Some(signal.installation.as_str()) {
                reglages.installation_signalee = true;
            }
            Ok(())
        })?;
    }
    Ok(true)
}

/// Documents Word, classeurs Excel et présentations PowerPoint en PDF, dans `dossier`.
pub fn office_en_pdf(
    fichiers: &[String],
    dossier: &str,
    progression: impl Fn(usize, usize),
) -> Result<Conversions, Erreur> {
    office::en_pdf(fichiers, dossier, progression)
}

/// PDF en documents Word modifiables, dans `dossier`.
pub fn pdf_en_word(
    fichiers: &[String],
    dossier: &str,
    progression: impl Fn(usize, usize),
) -> Result<Conversions, Erreur> {
    office::pdf_en_word(fichiers, dossier, progression)
}

/// Fichiers confiés à l'application par l'Explorateur (clic droit) : l'outil demandé et les
/// fichiers. Ligne de commande : `Automat-PDF.exe --outil compresser "C:\…\paie.pdf"`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recus {
    pub outil: Option<String>,
    pub fichiers: Vec<String>,
}

/// Ce que demande une ligne de commande (nom du programme compris) ; `None` s'il n'y a rien à ouvrir.
pub fn recus_de(arguments: impl IntoIterator<Item = String>) -> Option<Recus> {
    let mut outil = None;
    let mut fichiers = Vec::new();
    let mut arguments = arguments.into_iter().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--outil" {
            outil = arguments.next();
        } else if !argument.starts_with('-') {
            fichiers.push(argument);
        }
    }
    (!fichiers.is_empty()).then_some(Recus { outil, fichiers })
}

/// Fichiers confiés par l'Explorateur, gardés tant que l'interface n'est pas prête à les recevoir :
/// plusieurs fichiers choisis lancent chacun l'application, et les lancements suivants arrivent
/// souvent avant que la fenêtre ait fini de se charger.
#[derive(Debug, Default)]
pub struct Reception {
    prete: bool,
    en_attente: Vec<Recus>,
}

impl Reception {
    pub fn new(au_lancement: Option<Recus>) -> Self {
        Reception {
            prete: false,
            en_attente: au_lancement.into_iter().collect(),
        }
    }

    /// Fichiers d'un nouveau lancement : rendus s'il faut les remettre aussitôt à l'interface,
    /// gardés sinon.
    pub fn recevoir(&mut self, recus: Recus) -> Option<Recus> {
        if self.prete {
            return Some(recus);
        }
        self.en_attente.push(recus);
        None
    }

    /// L'interface est prête : elle prend les fichiers gardés, et recevra les suivants aussitôt.
    pub fn remettre(&mut self) -> Vec<Recus> {
        self.prete = true;
        std::mem::take(&mut self.en_attente)
    }
}

/// Contenu d'un fichier de réglages de l'application (vide s'il n'existe pas encore).
pub fn lire_reglages(dossier: &Path, nom: &str) -> Result<String, Erreur> {
    match std::fs::read_to_string(dossier.join(nom)) {
        Ok(contenu) => Ok(contenu),
        Err(erreur) if erreur.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(erreur) => Err(Erreur::simple(format!(
            "Les réglages n'ont pas pu être lus : {erreur}"
        ))),
    }
}

/// Enregistre un fichier de réglages : écrit à côté puis mis en place d'un coup, il n'est jamais
/// laissé à moitié écrit.
pub fn enregistrer_reglages(dossier: &Path, nom: &str, contenu: &str) -> Result<(), Erreur> {
    let echec = |erreur: std::io::Error| {
        Erreur::simple(format!(
            "Les réglages n'ont pas pu être enregistrés : {erreur}"
        ))
    };
    std::fs::create_dir_all(dossier).map_err(echec)?;
    let provisoire = dossier.join(format!("{nom}.tmp"));
    std::fs::write(&provisoire, contenu).map_err(echec)?;
    std::fs::rename(&provisoire, dossier.join(nom)).map_err(echec)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::essais::{fichier_temporaire, pdf_herite, serveur_local};

    #[test]
    fn enregistre_les_pages_choisies_de_plusieurs_pdf() {
        let a = fichier_temporaire("organiser", "a.pdf", &pdf_herite(2, "A"));
        let b = fichier_temporaire("organiser", "b.pdf", &pdf_herite(3, "B"));
        let sortie = fichier_temporaire("organiser", "resultat.pdf", b"");
        let choix = |chemin: &str, page| PageChoisie {
            chemin: chemin.to_string(),
            page,
            rotation: 0,
        };
        enregistrer_pages(&[choix(&b, 2), choix(&a, 0), choix(&b, 0)], &sortie).unwrap();
        assert_eq!(infos_pdf(&sortie).unwrap(), 3);
        assert_eq!(pages_pdf(&sortie).unwrap().len(), 3);
        let vignette = vignette_pdf(&sortie, 0, 120).unwrap();
        assert_eq!(&vignette[..2], [0xFF, 0xD8]);
        // Un fichier modifié est relu, pas pris dans la mémoire.
        enregistrer_pages(&[choix(&a, 1)], &sortie).unwrap();
        assert_eq!(pages_pdf(&sortie).unwrap().len(), 1);
    }

    #[test]
    fn exporte_les_pages_choisies_en_images() {
        let pdf = fichier_temporaire("images", "Rapport.pdf", &pdf_herite(3, "R"));
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("images");
        let _ = std::fs::remove_dir_all(dossier.join("Rapport - images"));
        let cible = pdf_en_images(
            &pdf,
            &[0, 2],
            FormatImage::Png,
            72,
            dossier.to_str().unwrap(),
        )
        .unwrap();
        let mut noms: Vec<String> = std::fs::read_dir(&cible)
            .unwrap()
            .map(|f| f.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        noms.sort();
        assert_eq!(noms, ["Rapport - page 001.png", "Rapport - page 003.png"]);
        let erreur = pdf_en_images(&pdf, &[5], FormatImage::Jpeg, 72, dossier.to_str().unwrap())
            .unwrap_err();
        assert!(erreur.message.contains("page 6"));
    }

    #[test]
    fn cree_un_pdf_par_personne_protege_par_son_mot_de_passe() {
        let paie = fichier_temporaire("personnes", "paie.pdf", &pdf_herite(5, "Bulletin"));
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("personnes");
        let personne = |nom: &str, pages: Vec<usize>, mot_de_passe: Option<&str>| FichierPersonne {
            nom: nom.to_string(),
            pages,
            mot_de_passe: mot_de_passe.map(str::to_string),
        };
        let personnes = [
            personne("DUPONT Jean", vec![0, 1], None),
            personne("MBARGA Awa", vec![4, 2], Some("15/03/1985")),
        ];
        let separation =
            separer_par_personne(&paie, &personnes, dossier.to_str().unwrap(), "Bulletins")
                .unwrap();
        assert!(separation.fichiers[0].ends_with("DUPONT Jean.pdf"));
        let lire = |chemin: &str| std::fs::read(chemin).unwrap();
        assert_eq!(
            texte::textes(Arc::new(lire(&separation.fichiers[0])), "a.pdf").unwrap(),
            ["Bulletin 1\n", "Bulletin 2\n"]
        );
        // Le second ne s'ouvre qu'avec son mot de passe ; ses pages sont dans l'ordre demandé.
        let protege = lire(&separation.fichiers[1]);
        assert!(pdf::charger(&protege, "b.pdf").is_err());
        let libre = securite::deverrouiller(&protege, "b.pdf", "15/03/1985").unwrap();
        assert_eq!(
            texte::textes(Arc::new(libre), "b.pdf").unwrap(),
            ["Bulletin 5\n", "Bulletin 3\n"]
        );
        // Une page qui n'existe pas est signalée.
        let erreur = separer_par_personne(
            &paie,
            &[personne("X", vec![7], None)],
            dossier.to_str().unwrap(),
            "Bulletins",
        )
        .unwrap_err();
        assert!(erreur.message.contains("page 8"), "{}", erreur.message);
    }

    #[test]
    fn garde_les_reglages_d_une_fois_sur_l_autre() {
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("reglages");
        let _ = std::fs::remove_dir_all(&dossier);
        assert_eq!(lire_reglages(&dossier, "enchainements.json").unwrap(), "");
        enregistrer_reglages(&dossier, "enchainements.json", r#"[{"nom":"Paie"}]"#).unwrap();
        enregistrer_reglages(
            &dossier,
            "enchainements.json",
            r#"[{"nom":"Paie mensuelle"}]"#,
        )
        .unwrap();
        assert_eq!(
            lire_reglages(&dossier, "enchainements.json").unwrap(),
            r#"[{"nom":"Paie mensuelle"}]"#
        );
        assert!(!dossier.join("enchainements.json.tmp").exists());
    }

    /// Événement de chaque signal reçu par le serveur local.
    fn evenements(requetes: &[crate::essais::RequeteRecue]) -> Vec<String> {
        requetes
            .iter()
            .map(|(_, _, corps)| {
                let signal: serde_json::Value = serde_json::from_str(corps).unwrap();
                signal["evenement"].as_str().unwrap().to_string()
            })
            .collect()
    }

    #[test]
    fn n_envoie_les_statistiques_qu_apres_un_oui() {
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("confidentialite");
        let _ = std::fs::remove_dir_all(&dossier);
        let serveur = serveur_local(|_, _| (200, "", r#"{"ok":true}"#.to_string()));
        let signaler_ici = || signaler(&serveur.adresse, &dossier, "fr-FR", "Windows 11").unwrap();

        // Premier lancement : la question n'a pas encore reçu de réponse, rien ne part.
        let neuf = confidentialite(&dossier).unwrap();
        assert_eq!(neuf.statistiques, None);
        assert!(neuf.mises_a_jour);
        assert!(!signaler_ici());
        // « Non merci » : rien ne part, et aucun numéro n'est tiré.
        regler_confidentialite(&dossier, Some(true), Some(false)).unwrap();
        assert!(!signaler_ici());
        assert!(serveur.requetes().is_empty());
        let fichier = lire_reglages(&dossier, CONFIDENTIALITE).unwrap();
        assert!(fichier.contains(r#""installation": null"#), "{fichier}");

        // « Oui » : le premier signal annonce l'installation, les suivants des ouvertures.
        let accepte = regler_confidentialite(&dossier, None, Some(true)).unwrap();
        assert_eq!(accepte.statistiques, Some(true));
        assert!(accepte.mises_a_jour, "réglage des mises à jour gardé");
        assert!(signaler_ici());
        assert!(signaler_ici());
        let requetes = serveur.requetes();
        assert_eq!(evenements(&requetes), ["installation", "ouverture"]);
        let numero = |rang: usize| {
            let signal: serde_json::Value = serde_json::from_str(&requetes[rang].2).unwrap();
            signal["installation"].as_str().unwrap().to_string()
        };
        assert_eq!(numero(0), numero(1), "même installation");

        // Les mises à jour se coupent sans toucher aux statistiques.
        let coupees = regler_confidentialite(&dossier, Some(false), None).unwrap();
        assert!(!coupees.mises_a_jour);
        assert_eq!(coupees.statistiques, Some(true));
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn garde_un_reglage_change_pendant_l_envoi() {
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("confidentialite-pendant");
        let _ = std::fs::remove_dir_all(&dossier);
        regler_confidentialite(&dossier, None, Some(true)).unwrap();
        // Le service répond lentement ; pendant ce temps, la recherche des mises à jour est coupée.
        let lent = serveur_local(|_, _| {
            std::thread::sleep(std::time::Duration::from_millis(600));
            (200, "", r#"{"ok":true}"#.to_string())
        });
        std::thread::scope(|fils| {
            let envoi = fils.spawn(|| signaler(&lent.adresse, &dossier, "fr", "Windows 11"));
            std::thread::sleep(std::time::Duration::from_millis(200));
            regler_confidentialite(&dossier, Some(false), None).unwrap();
            assert!(envoi.join().unwrap().unwrap());
        });
        let apres = reglages_de_confidentialite(&dossier).unwrap();
        assert!(
            !apres.mises_a_jour,
            "le réglage changé pendant l'envoi est gardé"
        );
        assert!(apres.installation_signalee);
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn annonce_l_installation_tant_qu_elle_n_est_pas_arrivee() {
        let dossier = std::env::temp_dir()
            .join("automat-pdf-tests")
            .join("confidentialite-panne");
        let _ = std::fs::remove_dir_all(&dossier);
        regler_confidentialite(&dossier, None, Some(true)).unwrap();
        let en_panne = serveur_local(|_, _| (500, "", String::new()));
        let erreur = signaler(&en_panne.adresse, &dossier, "fr", "Windows 10").unwrap_err();
        assert!(erreur.message.contains("500"), "{}", erreur.message);
        let repare = serveur_local(|_, _| (200, "", r#"{"ok":true}"#.to_string()));
        assert!(signaler(&repare.adresse, &dossier, "fr", "Windows 10").unwrap());
        assert!(signaler(&repare.adresse, &dossier, "fr", "Windows 10").unwrap());
        assert_eq!(
            evenements(&repare.requetes()),
            ["installation", "ouverture"]
        );
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    #[test]
    fn lit_les_fichiers_confies_par_l_explorateur() {
        let ligne = |arguments: &[&str]| recus_de(arguments.iter().map(|a| a.to_string()));
        assert_eq!(
            ligne(&[
                "Automat-PDF.exe",
                "--outil",
                "compresser",
                r"C:\Paie\octobre.pdf"
            ]),
            Some(Recus {
                outil: Some("compresser".into()),
                fichiers: vec![r"C:\Paie\octobre.pdf".into()]
            })
        );
        assert_eq!(
            ligne(&["Automat-PDF.exe", "a.pdf", "b.pdf"]).map(|r| (r.outil, r.fichiers.len())),
            Some((None, 2))
        );
        // Lancement ordinaire, ou option inconnue sans fichier : rien à ouvrir.
        assert_eq!(ligne(&["Automat-PDF.exe"]), None);
        assert_eq!(ligne(&["Automat-PDF.exe", "--outil", "fusionner"]), None);
    }

    #[test]
    fn garde_les_fichiers_arrives_avant_que_l_interface_soit_prete() {
        let recus = |fichier: &str| Recus {
            outil: Some("fusionner".into()),
            fichiers: vec![fichier.into()],
        };
        // Trois PDF choisis, l'application fermée : le premier lancement ouvre la fenêtre, les deux
        // autres lui confient leur fichier pendant qu'elle se charge.
        let mut reception = Reception::new(Some(recus("a.pdf")));
        assert_eq!(reception.recevoir(recus("b.pdf")), None);
        assert_eq!(reception.recevoir(recus("c.pdf")), None);
        assert_eq!(
            reception.remettre(),
            [recus("a.pdf"), recus("b.pdf"), recus("c.pdf")]
        );
        // L'interface écoute désormais : les suivants lui vont directement, rien n'est gardé.
        assert_eq!(reception.recevoir(recus("d.pdf")), Some(recus("d.pdf")));
        assert!(reception.remettre().is_empty());
        // Lancement ordinaire : rien à remettre.
        assert!(Reception::new(None).remettre().is_empty());
    }

    #[test]
    fn nomme_d_apres_le_chemin() {
        assert_eq!(nom_de("/tmp/Paie mars.pdf"), "Paie mars.pdf");
        assert_eq!(sans_extension("Paie mars.PDF"), "Paie mars");
        assert_eq!(sans_extension("notes.txt"), "notes.txt");
    }
}
