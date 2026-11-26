//! Commandes appelées par l'interface (`invoke`) : elles délèguent à `operations`, hors du fil
//! principal pour ne jamais figer la fenêtre.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::annotations::Annotation;
use crate::caviardage::{ModeCaviardage, PageACaviarder, Rapport};
use crate::commentaires::{Commentaire, Retouches};
use crate::compression::{Bilan, Niveau};
use crate::confidentialite::Confidentialite;
use crate::envoi::{BilanEnvoi, Message};
use crate::erreur::Erreur;
use crate::formulaires::Formulaire;
use crate::images::OptionsImages;
use crate::imposition::Disposition;
use crate::impression::{Feuille, Imprimante, OptionsImpression, Reglages};
use crate::lecteur_defaut::{Demande, EtatLecteur};
use crate::libreoffice::Convertisseurs;
use crate::marques::{OptionsMarques, Placement};
use crate::messagerie::Compte;
use crate::navigation::Navigation;
use crate::office::Conversions;
use crate::operations::{
    self, FichierPersonne, ImageChoisie, PageChoisie, Reception, Recus, Separation,
};
use crate::outlook::ModeEnvoi;
use crate::rendu::{FormatImage, FormatPage};
use crate::securite::OptionsProtection;
use crate::signatures::SignaturePdf;

async fn en_arriere_plan<T: Send + 'static>(
    travail: impl FnOnce() -> Result<T, Erreur> + Send + 'static,
) -> Result<T, Erreur> {
    tauri::async_runtime::spawn_blocking(travail)
        .await
        .map_err(|erreur| Erreur::simple(format!("Le traitement a été interrompu : {erreur}")))?
}

/// Contenu brut d'une liste choisie par l'utilisateur ; elle est décodée dans l'interface.
#[tauri::command]
pub async fn lire_liste(chemin: String) -> Result<Response, Erreur> {
    en_arriere_plan(move || operations::lire_liste(&chemin).map(Response::new)).await
}

/// Nombre de pages d'un PDF choisi par l'utilisateur.
#[tauri::command]
pub async fn infos_pdf(chemin: String) -> Result<usize, Erreur> {
    en_arriere_plan(move || operations::infos_pdf(&chemin)).await
}

/// Fusionne des PDF, dans l'ordre donné, dans le fichier `sortie`.
#[tauri::command]
pub async fn fusionner_pdf(chemins: Vec<String>, sortie: String) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::fusionner_pdf(&chemins, &sortie)).await
}

/// Sépare un PDF en fichiers de `pages_par_fichier` pages, dans un nouveau dossier (chemin renvoyé).
#[tauri::command]
pub async fn separer_pdf(
    chemin: String,
    pages_par_fichier: usize,
    noms: Vec<String>,
    dossier: String,
    sous_dossier: String,
) -> Result<String, Erreur> {
    en_arriere_plan(move || {
        operations::separer_pdf(&chemin, pages_par_fichier, &noms, &dossier, &sous_dossier)
    })
    .await
}

/// Taille de chaque page d'un PDF, pour placer les vignettes avant leur rendu.
#[tauri::command]
pub async fn pages_pdf(chemin: String) -> Result<Vec<FormatPage>, Erreur> {
    en_arriere_plan(move || operations::pages_pdf(&chemin)).await
}

/// Image JPEG d'une page (vignette ou aperçu).
#[tauri::command]
pub async fn vignette_pdf(chemin: String, page: usize, largeur: u32) -> Result<Response, Erreur> {
    en_arriere_plan(move || operations::vignette_pdf(&chemin, page, largeur).map(Response::new))
        .await
}

/// Enregistre un PDF fait des pages choisies, dans l'ordre.
#[tauri::command]
pub async fn enregistrer_pages(pages: Vec<PageChoisie>, sortie: String) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::enregistrer_pages(&pages, &sortie)).await
}

/// Vignette JPEG d'une image (redressée).
#[tauri::command]
pub async fn vignette_image(chemin: String, largeur: u32) -> Result<Response, Erreur> {
    en_arriere_plan(move || operations::vignette_image(&chemin, largeur).map(Response::new)).await
}

/// Enregistre un PDF fait des images choisies, une par page.
#[tauri::command]
pub async fn images_en_pdf(
    images: Vec<ImageChoisie>,
    options: OptionsImages,
    sortie: String,
) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::images_en_pdf(&images, options, &sortie)).await
}

/// Pages d'un PDF en images, dans un nouveau dossier (chemin renvoyé).
#[tauri::command]
pub async fn pdf_en_images(
    chemin: String,
    pages: Vec<usize>,
    format: FormatImage,
    dpi: u32,
    dossier: String,
) -> Result<String, Erreur> {
    en_arriere_plan(move || operations::pdf_en_images(&chemin, &pages, format, dpi, &dossier)).await
}

/// Enregistre le PDF mis en page (2 ou 4 par feuille, à la suite ou répétées, livret, format A4).
#[tauri::command]
pub async fn imposer_pdf(
    chemin: String,
    disposition: Disposition,
    repeter: bool,
    sortie: String,
) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::imposer_pdf(&chemin, disposition, repeter, &sortie)).await
}

/// Enregistre le PDF avec numéros de page et tampon.
#[tauri::command]
pub async fn marquer_pdf(
    chemin: String,
    options: OptionsMarques,
    sortie: String,
) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::marquer_pdf(&chemin, &options, &sortie)).await
}

/// Aperçu (JPEG) d'une page avec son numéro et son tampon.
#[tauri::command]
pub async fn apercu_marques(
    chemin: String,
    page: usize,
    options: OptionsMarques,
    largeur: u32,
) -> Result<Response, Erreur> {
    en_arriere_plan(move || {
        operations::apercu_marques(&chemin, page, &options, largeur).map(Response::new)
    })
    .await
}

/// Enregistre le PDF avec une image (signature, cachet) posée sur les pages choisies.
#[tauri::command]
pub async fn apposer_image(
    chemin: String,
    image: String,
    placement: Placement,
    pages: Vec<usize>,
    sortie: String,
) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::apposer_image(&chemin, &image, placement, &pages, &sortie))
        .await
}

/// Enregistre le PDF avec les annotations faites dans le lecteur.
#[tauri::command]
pub async fn annoter_pdf(
    chemin: String,
    annotations: Vec<Annotation>,
    sortie: String,
    aplatir: Option<bool>,
    retouches: Option<Retouches>,
) -> Result<(), Erreur> {
    en_arriere_plan(move || {
        operations::annoter_pdf(
            &chemin,
            &annotations,
            &sortie,
            aplatir.unwrap_or(false),
            retouches.as_ref(),
        )
    })
    .await
}

/// Contenu brut d'une image, pour l'afficher telle quelle.
#[tauri::command]
pub async fn lire_image(chemin: String) -> Result<Response, Erreur> {
    en_arriere_plan(move || operations::lire_image(&chemin).map(Response::new)).await
}

/// Enregistre le PDF caviardé ; renvoie les pages remplacées par leur image.
#[tauri::command]
pub async fn caviarder_pdf(
    chemin: String,
    pages: Vec<PageACaviarder>,
    mode: ModeCaviardage,
    sortie: String,
) -> Result<Rapport, Erreur> {
    en_arriere_plan(move || operations::caviarder_pdf(&chemin, &pages, mode, &sortie)).await
}

/// Taille d'un fichier, en octets.
#[tauri::command]
pub async fn taille_fichier(chemin: String) -> Result<u64, Erreur> {
    en_arriere_plan(move || operations::taille_fichier(&chemin)).await
}

/// Enregistre le PDF protégé par mot de passe.
#[tauri::command]
pub async fn proteger_pdf(
    chemin: String,
    options: OptionsProtection,
    sortie: String,
) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::proteger_pdf(&chemin, &options, &sortie)).await
}

/// Enregistre le PDF sans son mot de passe.
#[tauri::command]
pub async fn deverrouiller_pdf(
    chemin: String,
    mot_de_passe: String,
    sortie: String,
) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::deverrouiller_pdf(&chemin, &mot_de_passe, &sortie)).await
}

/// Imprime le PDF du lecteur (avec ses annotations en cours) : sous Windows avec les choix de la
/// fenêtre d'impression d'Automat-PDF, sur Mac par celle de macOS ; `false` : l'utilisateur a annulé.
#[tauri::command]
pub async fn imprimer_pdf(
    app: AppHandle,
    fenetre: tauri::WebviewWindow,
    chemin: String,
    annotations: Vec<Annotation>,
    options: Option<OptionsImpression>,
) -> Result<bool, Erreur> {
    en_arriere_plan(move || {
        operations::imprimer_pdf(&app, &fenetre, &chemin, &annotations, options.as_ref())
    })
    .await
}

/// Les imprimantes de l'ordinateur, celle par défaut en tête (Windows).
#[tauri::command]
pub async fn imprimantes() -> Result<Vec<Imprimante>, Erreur> {
    en_arriere_plan(operations::imprimantes).await
}

/// La feuille telle que l'imprimante la prendra, pour l'aperçu (Windows).
#[tauri::command]
pub async fn feuille_imprimante(options: OptionsImpression) -> Result<Feuille, Erreur> {
    en_arriere_plan(move || operations::feuille_imprimante(&options)).await
}

/// Les réglages du pilote de l'imprimante, dans sa propre fenêtre ; `None` : annulés (Windows).
#[tauri::command]
pub async fn proprietes_imprimante(
    app: AppHandle,
    fenetre: tauri::WebviewWindow,
    imprimante: String,
    pilote: Vec<u8>,
) -> Result<Option<Reglages>, Erreur> {
    en_arriere_plan(move || operations::proprietes_imprimante(&app, &fenetre, &imprimante, &pilote))
        .await
}

/// Automat-PDF ouvre-t-il les PDF du système ? Et peut-il le proposer ?
#[tauri::command]
pub async fn lecteur_par_defaut(app: AppHandle) -> EtatLecteur {
    en_arriere_plan(move || Ok::<_, Erreur>(operations::lecteur_par_defaut(&app)))
        .await
        .unwrap_or(EtatLecteur {
            possible: false,
            par_defaut: false,
        })
}

/// Propose de faire d'Automat-PDF le lecteur des PDF, par la fenêtre du système.
#[tauri::command]
pub async fn devenir_lecteur_par_defaut(
    app: AppHandle,
    fenetre: tauri::WebviewWindow,
) -> Result<Demande, Erreur> {
    en_arriere_plan(move || operations::devenir_lecteur_par_defaut(&app, &fenetre)).await
}

/// Les commentaires déjà présents dans un PDF reçu, pour la liste du lecteur.
#[tauri::command]
pub async fn commentaires_pdf(chemin: String) -> Result<Vec<Commentaire>, Erreur> {
    en_arriere_plan(move || operations::commentaires_pdf(&chemin)).await
}

/// Les champs du formulaire d'un PDF, pour le lecteur.
#[tauri::command]
pub async fn formulaire_pdf(chemin: String) -> Result<Formulaire, Erreur> {
    en_arriere_plan(move || operations::formulaire_pdf(&chemin)).await
}

/// Les signatures numériques d'un PDF, vérifiées.
#[tauri::command]
pub async fn signatures_pdf(chemin: String) -> Result<Vec<SignaturePdf>, Erreur> {
    en_arriere_plan(move || operations::signatures_pdf(&chemin)).await
}

/// La version d'un PDF qu'a signée une signature, écrite dans le dossier temporaire pour l'ouvrir.
#[tauri::command]
pub async fn version_signee(chemin: String, fin: usize) -> Result<String, Erreur> {
    let dossier = std::env::temp_dir()
        .join("Automat-PDF")
        .join("Versions signées");
    en_arriere_plan(move || operations::version_signee(&chemin, fin, &dossier)).await
}

/// Retouches du lecteur, en mémoire seulement : commentaires du PDF supprimés ou changés, champs
/// du formulaire remplis.
#[tauri::command]
pub async fn retoucher_lecture(chemin: String, retouches: Retouches) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::retoucher_lecture(&chemin, &retouches)).await
}

/// Signets et liens d'un PDF, pour le lecteur.
#[tauri::command]
pub async fn navigation_pdf(chemin: String) -> Result<Navigation, Erreur> {
    en_arriere_plan(move || operations::navigation_pdf(&chemin)).await
}

/// Ouvre le lien d'un PDF (page web, e-mail), après l'accord de l'utilisateur dans le lecteur.
#[tauri::command]
pub async fn ouvrir_lien(adresse: String) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::ouvrir_lien(&adresse)).await
}

/// Oublie le contenu déchiffré d'un PDF : son onglet s'est fermé dans le lecteur.
#[tauri::command]
pub fn reverrouiller_lecture(chemin: String) {
    operations::reverrouiller_lecture(&chemin);
}

/// Ouvre dans le lecteur un PDF protégé par un mot de passe : déchiffré en mémoire seulement.
#[tauri::command]
pub async fn deverrouiller_lecture(chemin: String, mot_de_passe: String) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::deverrouiller_lecture(&chemin, &mot_de_passe)).await
}

/// Enregistre le PDF allégé ; renvoie ce que la compression a fait.
#[tauri::command]
pub async fn compresser_pdf(
    chemin: String,
    niveau: Niveau,
    sortie: String,
) -> Result<Bilan, Erreur> {
    en_arriere_plan(move || operations::compresser_pdf(&chemin, niveau, &sortie)).await
}

/// Texte de chaque page, pour y repérer à qui elle appartient.
#[tauri::command]
pub async fn textes_pdf(chemin: String) -> Result<Vec<String>, Erreur> {
    en_arriere_plan(move || operations::textes_pdf(&chemin)).await
}

/// Les mots d'une page et leur place, pour le lecteur.
#[tauri::command]
pub async fn mots_page(chemin: String, page: usize) -> Result<Vec<crate::texte::Mot>, Erreur> {
    en_arriere_plan(move || operations::mots_page(&chemin, page)).await
}

/// Un PDF par personne (protégé par son mot de passe s'il en a un) ; renvoie les fichiers créés.
#[tauri::command]
pub async fn separer_par_personne(
    chemin: String,
    personnes: Vec<FichierPersonne>,
    dossier: String,
    sous_dossier: String,
) -> Result<Separation, Erreur> {
    en_arriere_plan(move || {
        operations::separer_par_personne(&chemin, &personnes, &dossier, &sous_dossier)
    })
    .await
}

/// Prépare dans Outlook (Brouillons) ou envoie les messages ; l'avancement est annoncé par
/// l'événement `progression-envoi` (messages traités, total).
#[tauri::command]
pub async fn envoyer_par_outlook(
    app: AppHandle,
    messages: Vec<Message>,
    mode: ModeEnvoi,
) -> Result<BilanEnvoi, Erreur> {
    en_arriere_plan(move || {
        operations::envoyer_par_outlook(&messages, mode, |fait, total| {
            let _ = app.emit("progression-envoi", (fait, total));
        })
    })
    .await
}

/// Envoie les messages par la messagerie de l'utilisateur (SMTP), avec le mot de passe gardé par
/// le système ; l'avancement est annoncé par l'événement `progression-envoi`.
#[tauri::command]
pub async fn envoyer_par_messagerie(
    app: AppHandle,
    compte: Compte,
    messages: Vec<Message>,
) -> Result<BilanEnvoi, Erreur> {
    en_arriere_plan(move || {
        operations::envoyer_par_messagerie(&compte, &messages, |fait, total| {
            let _ = app.emit("progression-envoi", (fait, total));
        })
    })
    .await
}

/// Vérifie la connexion à la messagerie et l'identification, sans rien envoyer ;
/// `mot_de_passe` : celui qui vient d'être saisi, sinon celui gardé par le système.
#[tauri::command]
pub async fn tester_messagerie(compte: Compte, mot_de_passe: Option<String>) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::tester_messagerie(&compte, mot_de_passe.as_deref())).await
}

/// Un mot de passe de messagerie est-il gardé par le système ?
#[tauri::command]
pub async fn mot_de_passe_messagerie_enregistre() -> Result<bool, Erreur> {
    en_arriere_plan(operations::mot_de_passe_messagerie_enregistre).await
}

/// Garde le mot de passe de la messagerie dans le coffre du système ; vide : l'oublie.
#[tauri::command]
pub async fn enregistrer_mot_de_passe_messagerie(mot_de_passe: String) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::enregistrer_mot_de_passe_messagerie(&mot_de_passe)).await
}

/// Documents Office en PDF ; l'avancement est annoncé par l'événement `progression-conversion`.
#[tauri::command]
pub async fn office_en_pdf(
    app: AppHandle,
    fichiers: Vec<String>,
    dossier: String,
) -> Result<Conversions, Erreur> {
    en_arriere_plan(move || {
        operations::office_en_pdf(&fichiers, &dossier, |fait, total| {
            let _ = app.emit("progression-conversion", (fait, total));
        })
    })
    .await
}

/// PDF en documents Word ; l'avancement est annoncé par l'événement `progression-conversion`.
#[tauri::command]
pub async fn pdf_en_word(
    app: AppHandle,
    fichiers: Vec<String>,
    dossier: String,
) -> Result<Conversions, Erreur> {
    en_arriere_plan(move || {
        operations::pdf_en_word(&fichiers, &dossier, |fait, total| {
            let _ = app.emit("progression-conversion", (fait, total));
        })
    })
    .await
}

/// Ce qui permet de convertir les documents Office sur ce PC (Office, LibreOffice, installation
/// possible de LibreOffice).
#[tauri::command]
pub async fn convertisseurs_office() -> Result<Convertisseurs, Erreur> {
    en_arriere_plan(|| Ok(operations::convertisseurs_office())).await
}

/// Installe LibreOffice par son installateur officiel (winget) : plusieurs minutes, et l'accord
/// d'un administrateur demandé par Windows.
#[tauri::command]
pub async fn installer_libreoffice() -> Result<(), Erreur> {
    en_arriere_plan(operations::installer_libreoffice).await
}

/// Ouvre la page de téléchargement officielle de LibreOffice dans le navigateur.
#[tauri::command]
pub async fn ouvrir_page_libreoffice() -> Result<(), Erreur> {
    en_arriere_plan(operations::ouvrir_page_libreoffice).await
}

/// Ouvre un fichier créé par l'application (PDF, document Word, image) dans l'application
/// habituelle, ou un dossier.
#[tauri::command]
pub async fn ouvrir_fichier(chemin: String) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::ouvrir_fichier(&chemin)).await
}

/// Affiche un fichier dans son dossier (Explorateur, Finder), sélectionné.
#[tauri::command]
pub async fn montrer_dans_dossier(chemin: String) -> Result<(), Erreur> {
    en_arriere_plan(move || operations::montrer_dans_dossier(&chemin)).await
}

/// Fichier des enchaînements enregistrés, dans le dossier de l'application.
const ENCHAINEMENTS: &str = "enchainements.json";

fn dossier_reglages(app: &AppHandle) -> Result<PathBuf, Erreur> {
    app.path().app_data_dir().map_err(|erreur| {
        Erreur::simple(format!(
            "Le dossier des réglages est introuvable : {erreur}"
        ))
    })
}

/// Enchaînements enregistrés (JSON ; vide la première fois).
#[tauri::command]
pub async fn lire_enchainements(app: AppHandle) -> Result<String, Erreur> {
    let dossier = dossier_reglages(&app)?;
    en_arriere_plan(move || operations::lire_reglages(&dossier, ENCHAINEMENTS)).await
}

/// Enregistre les enchaînements (JSON).
#[tauri::command]
pub async fn enregistrer_enchainements(app: AppHandle, contenu: String) -> Result<(), Erreur> {
    let dossier = dossier_reglages(&app)?;
    en_arriere_plan(move || operations::enregistrer_reglages(&dossier, ENCHAINEMENTS, &contenu))
        .await
}

/// Fichier de la signature et des initiales dessinées dans le lecteur (0.7.0), gardées sur cet
/// ordinateur seulement, pour la prochaine fois.
const SIGNATURES: &str = "signatures.json";
/// Taille d'un fichier de signatures au plus (deux dessins : bien moins).
const SIGNATURES_MAX: usize = 2_000_000;

/// Signature et initiales gardées (JSON ; vide s'il n'y en a pas).
#[tauri::command]
pub async fn lire_signatures(app: AppHandle) -> Result<String, Erreur> {
    let dossier = dossier_reglages(&app)?;
    en_arriere_plan(move || operations::lire_reglages(&dossier, SIGNATURES)).await
}

/// Garde la signature et les initiales (JSON) sur cet ordinateur ; vide : les oublie.
#[tauri::command]
pub async fn enregistrer_signatures(app: AppHandle, contenu: String) -> Result<(), Erreur> {
    if contenu.len() > SIGNATURES_MAX {
        return Err(Erreur::simple(
            "Cette signature est trop grande pour être gardée.",
        ));
    }
    let dossier = dossier_reglages(&app)?;
    en_arriere_plan(move || operations::enregistrer_reglages(&dossier, SIGNATURES, &contenu)).await
}

/// Fichier des réglages de la messagerie (moyen d'envoi, compte ; jamais le mot de passe).
const MESSAGERIE: &str = "messagerie.json";

/// Réglages de la messagerie (JSON ; vide la première fois).
#[tauri::command]
pub async fn lire_messagerie(app: AppHandle) -> Result<String, Erreur> {
    let dossier = dossier_reglages(&app)?;
    en_arriere_plan(move || operations::lire_reglages(&dossier, MESSAGERIE)).await
}

/// Enregistre les réglages de la messagerie (JSON).
#[tauri::command]
pub async fn enregistrer_messagerie(app: AppHandle, contenu: String) -> Result<(), Erreur> {
    let dossier = dossier_reglages(&app)?;
    en_arriere_plan(move || operations::enregistrer_reglages(&dossier, MESSAGERIE, &contenu)).await
}

/// Réglages de confidentialité : recherche des mises à jour, statistiques anonymes.
#[tauri::command]
pub async fn lire_confidentialite(app: AppHandle) -> Result<Confidentialite, Erreur> {
    let dossier = dossier_reglages(&app)?;
    en_arriere_plan(move || operations::confidentialite(&dossier)).await
}

/// Change les réglages de confidentialité donnés ; les autres restent tels quels.
#[tauri::command]
pub async fn regler_confidentialite(
    app: AppHandle,
    mises_a_jour: Option<bool>,
    statistiques: Option<bool>,
) -> Result<Confidentialite, Erreur> {
    let dossier = dossier_reglages(&app)?;
    en_arriere_plan(move || {
        operations::regler_confidentialite(&dossier, mises_a_jour, statistiques)
    })
    .await
}

/// Envoie le signal d'ouverture si l'utilisateur a accepté les statistiques anonymes ; `true` si
/// un signal est parti.
#[tauri::command]
pub async fn signaler_ouverture(app: AppHandle, langue: String) -> Result<bool, Erreur> {
    let dossier = dossier_reglages(&app)?;
    en_arriere_plan(move || operations::signaler_ouverture(&dossier, &langue)).await
}

/// Ouvre la page de la dernière version publiée dans le navigateur.
#[tauri::command]
pub async fn ouvrir_page_des_versions() -> Result<(), Erreur> {
    en_arriere_plan(operations::ouvrir_page_des_versions).await
}

/// Relance l'application : sous macOS, la mise à jour remplace l'application sans la fermer, et la
/// nouvelle version ne sert qu'une fois rouverte.
#[tauri::command]
pub fn relancer(app: AppHandle) {
    app.restart()
}

/// Fichiers confiés par le système (clic droit, « Ouvrir avec »), gardés jusqu'à ce que
/// l'interface les prenne.
pub struct Lancement(pub Mutex<Reception>);

/// Fichiers confiés avant que l'interface soit prête : au lancement, et par les lancements suivants
/// arrivés entre-temps. Les autres arrivent par l'événement `fichiers-recus`.
#[tauri::command]
pub fn recus_au_lancement(lancement: State<'_, Lancement>) -> Vec<Recus> {
    lancement
        .0
        .lock()
        .map(|mut reception| reception.remettre())
        .unwrap_or_default()
}
