//! Numériser depuis le scanner. Le système liste les scanners de l'ordinateur, branchés ou sur le
//! réseau ; chaque numérisation écrit ses pages dans un dossier de travail, en JPEG (en PNG en noir
//! et blanc). Le PDF est fait ensuite des pages gardées, chacune à sa taille réelle d'après sa
//! résolution, avec son texte reconnu si on le demande (`ocr`).
//!
//! - Mac : ImageCaptureCore, qui voit les mêmes scanners que Transfert d'images (USB, réseau,
//!   AirScan). Ses réponses n'arrivent que sur le fil principal : tout s'y passe, l'appelant attend.
//! - Windows 10 et 11 : Windows.Devices.Scanners, au-dessus de WIA : les scanners installés avec
//!   leur pilote WIA, comme presque tous.
//!
//! Le dossier de travail est dans le cache de l'application. Il est vidé au démarrage, et quand
//! l'utilisateur efface ses pages : les pages numérisées ne restent pas sur l'ordinateur.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::erreur::Erreur;
use crate::{fichiers, images, ocr};

/// Un scanner : son identifiant pour le système, son nom, et ses sources quand on les connaît
/// (Windows les donne d'emblée ; sur Mac, on les apprend à la première numérisation).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scanner {
    pub id: String,
    pub nom: String,
    pub sources: Option<Sources>,
}

/// Ce qu'un scanner sait faire : vitre, chargeur de documents, recto verso par le chargeur.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sources {
    pub vitre: bool,
    pub chargeur: bool,
    pub recto_verso: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    Vitre,
    Chargeur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Couleur {
    Couleur,
    Gris,
    NoirEtBlanc,
}

/// Les réglages d'une numérisation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reglages {
    /// L'identifiant du scanner (`Scanner::id`).
    pub scanner: String,
    pub source: Source,
    /// Recto verso, par le chargeur, si le scanner le peut.
    #[serde(default)]
    pub recto_verso: bool,
    pub couleur: Couleur,
    /// Résolution voulue, en points par pouce : le scanner prend la plus proche des siennes.
    pub resolution: u32,
}

/// Une page numérisée : son image, dans le dossier de travail, et sa résolution réelle.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageNumerisee {
    pub chemin: String,
    pub resolution: u32,
}

/// Ce qu'a donné une numérisation.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Numerisation {
    pub pages: Vec<PageNumerisee>,
    /// Ce que sait faire le scanner, appris en numérisant.
    pub sources: Option<Sources>,
    /// Arrêtée par l'utilisateur : les pages déjà numérisées sont gardées.
    pub annulee: bool,
    /// Ce qui a arrêté le scanner après quelques pages (feuille coincée…), s'il y a eu un problème.
    pub interrompue: Option<String>,
}

/// Les scanners de l'ordinateur, branchés ou sur le réseau.
pub fn scanners() -> Result<Vec<Scanner>, Erreur> {
    systeme::scanners()
}

/// Numérise selon `reglages`, dans un nouveau sous-dossier du dossier de travail `travail`.
/// `avancement(pages numérisées)` est appelé à chaque page.
pub fn numeriser(
    reglages: &Reglages,
    travail: &Path,
    avancement: Arc<dyn Fn(usize) + Send + Sync>,
) -> Result<Numerisation, Erreur> {
    let dossier = nouveau_dossier(travail)?;
    let numerisation = systeme::numeriser(reglages, &dossier, avancement)?;
    if numerisation.pages.is_empty() && !numerisation.annulee {
        return Err(Erreur::simple(numerisation.interrompue.unwrap_or_else(
            || {
                "Le scanner n'a rendu aucune page. Vérifiez que le document est bien posé, puis \
             recommencez."
                    .into()
            },
        )));
    }
    Ok(numerisation)
}

/// Arrête la numérisation en cours. Les pages déjà numérisées sont gardées.
pub fn annuler() {
    systeme::annuler();
}

/// Efface les pages numérisées : tout le dossier de travail.
pub fn oublier(travail: &Path) -> Result<(), Erreur> {
    match std::fs::remove_dir_all(travail) {
        Err(erreur) if erreur.kind() != std::io::ErrorKind::NotFound => Err(Erreur::simple(
            format!("Les pages numérisées n'ont pas pu être effacées : {erreur}"),
        )),
        _ => Ok(()),
    }
}

/// Un sous-dossier neuf du dossier de travail, pour les pages d'une numérisation.
fn nouveau_dossier(travail: &Path) -> Result<PathBuf, Erreur> {
    static COMPTEUR: AtomicU64 = AtomicU64::new(0);
    let horodatage = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duree| duree.as_millis())
        .unwrap_or(0);
    let numero = COMPTEUR.fetch_add(1, Ordering::Relaxed);
    let dossier = travail.join(format!("{horodatage}-{numero}"));
    std::fs::create_dir_all(&dossier).map_err(|erreur| {
        Erreur::simple(format!(
            "Le dossier des pages numérisées n'a pas pu être créé : {erreur}"
        ))
    })?;
    Ok(dossier)
}

/// Une page à mettre dans le PDF : son image, la rotation ajoutée par l'utilisateur (degrés, sens
/// horaire) et sa résolution.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagePdf {
    pub chemin: String,
    pub rotation: i64,
    pub resolution: u32,
}

/// Ce qu'a donné l'enregistrement du PDF.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanPdf {
    pub pages: usize,
    /// Le texte reconnu, quand on l'a demandé.
    pub texte: Option<ocr::Bilan>,
    /// Pourquoi le texte n'a pas été reconnu ; le PDF est enregistré sans.
    pub sans_texte: Option<String>,
}

/// Le PDF des pages numérisées, dans l'ordre, chacune à sa taille réelle, écrit dans `sortie`. Son
/// texte est reconnu si `reconnaitre` : une reconnaissance impossible n'empêche pas d'enregistrer
/// le PDF, le bilan dit pourquoi. `avancement(pages lues, pages à lire)`.
pub fn enregistrer_pdf(
    pages: &[PagePdf],
    reconnaitre: bool,
    sortie: &str,
    avancement: &(dyn Fn(usize, usize) + Sync),
) -> Result<BilanPdf, Erreur> {
    if pages.is_empty() {
        return Err(Erreur::simple(
            "Aucune page à enregistrer : numérisez d'abord.",
        ));
    }
    let contenus: Vec<Vec<u8>> = pages
        .iter()
        .map(|page| {
            std::fs::read(&page.chemin).map_err(|erreur| {
                Erreur::simple(format!(
                    "Une page numérisée est introuvable ({erreur}) : numérisez-la de nouveau."
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    let sources: Vec<images::ImageSource> = pages
        .iter()
        .zip(&contenus)
        .enumerate()
        .map(|(rang, (page, octets))| images::ImageSource {
            nom: format!("page {}", rang + 1),
            octets,
            rotation: page.rotation,
            ppp: Some(page.resolution as f32),
        })
        .collect();
    let pdf = images::images_en_pdf(
        &sources,
        images::OptionsImages {
            format: images::FormatFeuille::Reelle,
            marge_mm: 0.0,
        },
    )?;
    let mut bilan = BilanPdf {
        pages: pages.len(),
        ..BilanPdf::default()
    };
    let pdf = if reconnaitre {
        let nom = crate::operations::nom_de(sortie);
        let pdf = Arc::new(pdf);
        match ocr::disponible().and_then(|()| ocr::reconnaitre_pdf(pdf.clone(), &nom, avancement)) {
            Ok((avec_texte, lu)) => {
                bilan.texte = Some(lu);
                avec_texte
            }
            Err(erreur) => {
                bilan.sans_texte = Some(erreur.message);
                Arc::unwrap_or_clone(pdf)
            }
        }
    } else {
        pdf
    };
    fichiers::enregistrer(sortie, &pdf)?;
    Ok(bilan)
}

#[cfg(target_os = "macos")]
mod systeme {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{self, RecvTimeoutError, Sender};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use dispatch2::DispatchQueue;
    use objc2::rc::Retained;
    use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
    use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
    use objc2_foundation::{
        NSArray, NSError, NSIndexSet, NSNumber, NSPoint, NSRect, NSSize, NSString, NSURL,
    };
    use objc2_image_capture_core::{
        ICDevice, ICDeviceBrowser, ICDeviceBrowserDelegate, ICDeviceDelegate,
        ICDeviceLocationTypeMask, ICDeviceType, ICDeviceTypeMask, ICScannerBitDepth,
        ICScannerDevice, ICScannerDeviceDelegate, ICScannerDocumentType, ICScannerFunctionalUnit,
        ICScannerFunctionalUnitDocumentFeeder, ICScannerFunctionalUnitType,
        ICScannerMeasurementUnit, ICScannerPixelDataType, ICScannerTransferMode,
    };

    use super::{Couleur, Numerisation, PageNumerisee, Reglages, Scanner, Source, Sources};
    use crate::erreur::Erreur;

    /// Ce qu'on a appris des scanners en numérisant : leurs sources, par identifiant.
    static SOURCES_CONNUES: Mutex<Vec<(String, Sources)>> = Mutex::new(Vec::new());
    /// Le navigateur d'appareils a-t-il été lancé ?
    static LANCE: AtomicBool = AtomicBool::new(false);
    /// L'utilisateur a demandé d'arrêter la numérisation en cours.
    static ARRET: AtomicBool = AtomicBool::new(false);
    /// Silence du scanner au-delà duquel on abandonne : à l'ouverture, puis entre deux pages (un
    /// scanner qui chauffe sa lampe met parfois une minute à rendre la première).
    const SILENCE_OUVERTURE: Duration = Duration::from_secs(90);
    const SILENCE_PAGES: Duration = Duration::from_secs(300);
    /// Sans choix de la vitre ou du chargeur par le scanner lui-même, peu après l'ouverture de la
    /// session, on le lui demande.
    const CHOIX_SPONTANE: Duration = Duration::from_secs(3);
    /// Après une demande d'arrêt sans réponse du scanner, on n'attend plus.
    const ARRET_MAX: Duration = Duration::from_secs(10);
    /// Recherches du scanner choisi avant d'abandonner, une par seconde.
    const ESSAIS_MAX: u32 = 5;
    /// `NSNotFound` (NSIntegerMax) : pas de tel index.
    const PAS_TROUVE: usize = isize::MAX as usize;
    const ANNULEE: isize = -9924;

    /// Ce que le fil principal annonce à celui qui attend la numérisation.
    enum Message {
        /// La vitre ou le chargeur est choisi et réglé : la numérisation commence.
        Prete {
            resolution: u32,
            sources: Sources,
        },
        Page(String),
        /// Le navigateur ne voit pas (encore) ce scanner.
        Introuvable,
        /// Fin : annulée ou non, ou l'erreur qui l'a arrêtée.
        Fin(Result<bool, Erreur>),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Etape {
        Ouverture,
        Choix,
        Numerisation,
    }

    /// La numérisation en cours (fil principal).
    struct Travail {
        scanner: Retained<ICScannerDevice>,
        reglages: Reglages,
        dossier: PathBuf,
        envoi: Sender<Message>,
        etape: Etape,
        /// Session ouverte à cet instant.
        ouverte: Option<Instant>,
        /// La vitre ou le chargeur a été demandé au scanner.
        demande: bool,
    }

    #[derive(Default)]
    struct EtatDelegue {
        travail: RefCell<Option<Travail>>,
    }

    define_class!(
        // SAFETY : NSObject n'impose rien à ses sous-classes ; `Delegue` n'implémente pas `Drop`.
        #[unsafe(super = NSObject)]
        #[thread_kind = MainThreadOnly]
        #[name = "AutomatPdfDelegueNumerisation"]
        #[ivars = EtatDelegue]
        struct Delegue;

        unsafe impl NSObjectProtocol for Delegue {}

        // SAFETY : signatures des messages d'ICDeviceBrowserDelegate.
        unsafe impl ICDeviceBrowserDelegate for Delegue {
            #[unsafe(method(deviceBrowser:didAddDevice:moreComing:))]
            fn appareil_ajoute(
                &self,
                _navigateur: &ICDeviceBrowser,
                _appareil: &ICDevice,
                _suite: bool,
            ) {
            }

            #[unsafe(method(deviceBrowser:didRemoveDevice:moreGoing:))]
            fn appareil_retire(
                &self,
                _navigateur: &ICDeviceBrowser,
                appareil: &ICDevice,
                _suite: bool,
            ) {
                self.retire(appareil);
            }
        }

        // SAFETY : signatures des messages d'ICDeviceDelegate.
        unsafe impl ICDeviceDelegate for Delegue {
            #[unsafe(method(didRemoveDevice:))]
            fn retire_du_systeme(&self, appareil: &ICDevice) {
                self.retire(appareil);
            }

            #[unsafe(method(device:didOpenSessionWithError:))]
            fn session_ouverte(&self, _appareil: &ICDevice, erreur: Option<&NSError>) {
                self.apres_ouverture(erreur);
            }

            #[unsafe(method(device:didCloseSessionWithError:))]
            fn session_fermee(&self, _appareil: &ICDevice, _erreur: Option<&NSError>) {}
        }

        // SAFETY : signatures des messages d'ICScannerDeviceDelegate.
        unsafe impl ICScannerDeviceDelegate for Delegue {
            #[unsafe(method(scannerDevice:didSelectFunctionalUnit:error:))]
            fn unite_choisie(
                &self,
                _scanner: &ICScannerDevice,
                unite: &ICScannerFunctionalUnit,
                erreur: Option<&NSError>,
            ) {
                self.apres_choix(unite, erreur);
            }

            #[unsafe(method(scannerDevice:didScanToURL:))]
            fn page_numerisee(&self, _scanner: &ICScannerDevice, url: &NSURL) {
                let envoi = self
                    .ivars()
                    .travail
                    .borrow()
                    .as_ref()
                    .map(|travail| travail.envoi.clone());
                if let (Some(envoi), Some(chemin)) = (envoi, url.path()) {
                    let _ = envoi.send(Message::Page(chemin.to_string()));
                }
            }

            #[unsafe(method(scannerDevice:didCompleteScanWithError:))]
            fn numerisation_finie(&self, _scanner: &ICScannerDevice, erreur: Option<&NSError>) {
                self.terminer(match erreur {
                    None => Ok(false),
                    Some(erreur) if erreur.code() == ANNULEE => Ok(true),
                    Some(erreur) => Err(erreur_du_scanner(erreur, "La numérisation a échoué")),
                });
            }
        }
    );

    impl Delegue {
        fn new(mtm: MainThreadMarker) -> Retained<Self> {
            let objet = Self::alloc(mtm).set_ivars(EtatDelegue::default());
            // SAFETY : `init` de NSObject, sans argument.
            unsafe { msg_send![super(objet), init] }
        }

        /// Termine la numérisation en cours : son résultat part à l'appelant, la session du
        /// scanner est fermée.
        fn terminer(&self, resultat: Result<bool, Erreur>) {
            let Some(travail) = self.ivars().travail.borrow_mut().take() else {
                return;
            };
            let _ = travail.envoi.send(Message::Fin(resultat));
            // SAFETY : scanner de la numérisation, sur le fil principal.
            unsafe {
                if travail.scanner.hasOpenSession() {
                    travail.scanner.requestCloseSession();
                }
            }
        }

        fn retire(&self, appareil: &ICDevice) {
            let concerne = self
                .ivars()
                .travail
                .borrow()
                .as_ref()
                .is_some_and(|travail| std::ptr::eq::<ICDevice>(&**travail.scanner, appareil));
            if concerne {
                self.terminer(Err(Erreur::simple(
                    "Le scanner a été débranché ou éteint pendant la numérisation.",
                )));
            }
        }

        fn apres_ouverture(&self, erreur: Option<&NSError>) {
            if let Some(erreur) = erreur {
                self.terminer(Err(erreur_du_scanner(
                    erreur,
                    "Le scanner n'a pas pu être ouvert",
                )));
                return;
            }
            if let Some(travail) = self.ivars().travail.borrow_mut().as_mut() {
                travail.etape = Etape::Choix;
                travail.ouverte = Some(Instant::now());
            }
        }

        /// Demande la vitre ou le chargeur au scanner, s'il ne l'a pas choisi lui-même peu après
        /// l'ouverture de la session.
        fn demander_l_unite(&self) {
            let demande = {
                let mut travail = self.ivars().travail.borrow_mut();
                match travail.as_mut() {
                    Some(travail)
                        if travail.etape == Etape::Choix
                            && !travail.demande
                            && travail
                                .ouverte
                                .is_some_and(|ouverte| ouverte.elapsed() >= CHOIX_SPONTANE) =>
                    {
                        travail.demande = true;
                        Some((
                            travail.scanner.clone(),
                            unite_voulue(travail.reglages.source),
                        ))
                    }
                    _ => None,
                }
            };
            if let Some((scanner, unite)) = demande {
                // SAFETY : scanner de la numérisation, session ouverte, sur le fil principal.
                unsafe { scanner.requestSelectFunctionalUnit(unite) };
            }
        }

        /// Le scanner a choisi une unité (vitre, chargeur…) : de lui-même après l'ouverture de la
        /// session, ou à notre demande. La bonne est réglée et la numérisation commence ; une
        /// autre, on demande la bonne une fois ; un refus à notre demande : le scanner ne l'a pas.
        fn apres_choix(&self, unite: &ICScannerFunctionalUnit, erreur: Option<&NSError>) {
            enum Suite {
                Rien,
                Demander(Retained<ICScannerDevice>, ICScannerFunctionalUnitType),
                Absente(Source),
                Numeriser(
                    Retained<ICScannerDevice>,
                    Reglages,
                    PathBuf,
                    Sender<Message>,
                ),
            }
            let suite = {
                let mut travail = self.ivars().travail.borrow_mut();
                match travail.as_mut() {
                    Some(travail) if travail.etape == Etape::Choix => {
                        let voulue = unite_voulue(travail.reglages.source);
                        // SAFETY : unité donnée par le scanner, lue sur le fil principal.
                        let choisie = unsafe { unite.r#type() };
                        match (erreur, choisie == voulue, travail.demande) {
                            // Une réponse en erreur arrive parfois avant le choix spontané.
                            (Some(_), _, false) => Suite::Rien,
                            (Some(_), _, true) | (None, false, true) => {
                                Suite::Absente(travail.reglages.source)
                            }
                            (None, false, false) => {
                                travail.demande = true;
                                Suite::Demander(travail.scanner.clone(), voulue)
                            }
                            (None, true, _) => {
                                travail.etape = Etape::Numerisation;
                                Suite::Numeriser(
                                    travail.scanner.clone(),
                                    travail.reglages.clone(),
                                    travail.dossier.clone(),
                                    travail.envoi.clone(),
                                )
                            }
                        }
                    }
                    _ => Suite::Rien,
                }
            };
            match suite {
                Suite::Rien => {}
                // SAFETY : scanner de la numérisation, session ouverte, sur le fil principal.
                Suite::Demander(scanner, voulue) => unsafe {
                    scanner.requestSelectFunctionalUnit(voulue)
                },
                Suite::Absente(source) => self.terminer(Err(Erreur::simple(match source {
                    Source::Vitre => {
                        "Ce scanner n'a pas de vitre : choisissez le chargeur de documents."
                    }
                    Source::Chargeur => {
                        "Ce scanner n'a pas de chargeur de documents : choisissez la vitre."
                    }
                }))),
                Suite::Numeriser(scanner, reglages, dossier, envoi) => {
                    let (resolution, sources) = regler(&scanner, unite, &reglages, &dossier);
                    let _ = envoi.send(Message::Prete {
                        resolution,
                        sources,
                    });
                    // SAFETY : scanner réglé ci-dessus, session ouverte, sur le fil principal.
                    unsafe { scanner.requestScan() };
                }
            }
        }
    }

    thread_local! {
        /// Fil principal : le navigateur d'appareils et son délégué, lancés une fois, gardés
        /// jusqu'à la fermeture de l'application.
        static NAVIGATION: RefCell<Option<(Retained<ICDeviceBrowser>, Retained<Delegue>)>> =
            const { RefCell::new(None) };
    }

    /// Confie `travail` au fil principal, où ImageCaptureCore répond.
    fn sur_le_fil_principal(travail: impl FnOnce(MainThreadMarker) + Send + 'static) {
        DispatchQueue::main().exec_async(move || {
            if let Some(mtm) = MainThreadMarker::new() {
                travail(mtm);
            }
        });
    }

    /// Le navigateur d'appareils et son délégué, créés et lancés au premier appel.
    fn navigation(mtm: MainThreadMarker) -> (Retained<ICDeviceBrowser>, Retained<Delegue>) {
        NAVIGATION.with(|navigation| {
            let mut navigation = navigation.borrow_mut();
            if let Some((navigateur, delegue)) = navigation.as_ref() {
                return (navigateur.clone(), delegue.clone());
            }
            let delegue = Delegue::new(mtm);
            let masque = ICDeviceTypeMask::Scanner.0
                | ICDeviceLocationTypeMask::Local.0
                | ICDeviceLocationTypeMask::Shared.0
                | ICDeviceLocationTypeMask::Bonjour.0
                | ICDeviceLocationTypeMask::Remote.0;
            // SAFETY : navigateur créé, réglé et lancé sur le fil principal, où arrivent ses
            // messages ; son délégué (référence faible) est gardé avec lui.
            let navigateur = unsafe {
                let navigateur = ICDeviceBrowser::new();
                navigateur.setDelegate(Some(ProtocolObject::from_ref(&*delegue)));
                navigateur.setBrowsedDeviceTypeMask(ICDeviceTypeMask(masque));
                navigateur.start();
                navigateur
            };
            *navigation = Some((navigateur.clone(), delegue.clone()));
            (navigateur, delegue)
        })
    }

    fn est_un_scanner(appareil: &ICDevice) -> bool {
        // SAFETY : appareil du navigateur, lu sur le fil principal.
        unsafe { appareil.r#type() }.0 & ICDeviceType::Scanner.0 != 0
    }

    fn identifiant(appareil: &ICDevice) -> Option<String> {
        // SAFETY : appareil du navigateur, lu sur le fil principal.
        unsafe { appareil.UUIDString() }.map(|id| id.to_string())
    }

    /// Les scanners que voit le navigateur, avec les sources apprises en numérisant.
    fn liste(navigateur: &ICDeviceBrowser) -> Vec<Scanner> {
        let connues = SOURCES_CONNUES
            .lock()
            .map(|connues| connues.clone())
            .unwrap_or_default();
        // SAFETY : navigateur lu sur le fil principal.
        let appareils = unsafe { navigateur.devices() };
        appareils
            .map(|appareils| {
                appareils
                    .iter()
                    .filter(|appareil| est_un_scanner(appareil))
                    .filter_map(|appareil| {
                        let id = identifiant(&appareil)?;
                        // SAFETY : appareil du navigateur, lu sur le fil principal.
                        let nom = unsafe { appareil.name() }
                            .map(|nom| nom.to_string())
                            .unwrap_or_else(|| "Scanner".into());
                        let sources = connues
                            .iter()
                            .find(|(connu, _)| *connu == id)
                            .map(|(_, sources)| *sources);
                        Some(Scanner { id, nom, sources })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn retenir(id: &str, sources: Sources) {
        if let Ok(mut connues) = SOURCES_CONNUES.lock() {
            connues.retain(|(connu, _)| connu != id);
            connues.push((id.to_string(), sources));
        }
    }

    pub fn scanners() -> Result<Vec<Scanner>, Erreur> {
        let premiere = !LANCE.swap(true, Ordering::SeqCst);
        sur_le_fil_principal(|mtm| {
            navigation(mtm);
        });
        // Les scanners de l'ordinateur arrivent aussitôt, ceux du réseau peu à peu.
        std::thread::sleep(Duration::from_millis(if premiere { 2500 } else { 800 }));
        let (envoi, reception) = mpsc::channel();
        sur_le_fil_principal(move |mtm| {
            let (navigateur, _) = navigation(mtm);
            let _ = envoi.send(liste(&navigateur));
        });
        reception
            .recv_timeout(Duration::from_secs(20))
            .map_err(|_| Erreur::simple("La recherche des scanners n'a pas abouti."))
    }

    pub fn numeriser(
        reglages: &Reglages,
        dossier: &Path,
        avancement: Arc<dyn Fn(usize) + Send + Sync>,
    ) -> Result<Numerisation, Erreur> {
        // Navigateur pas encore lancé : il trouve d'abord les scanners.
        if !LANCE.load(Ordering::SeqCst) {
            scanners()?;
        }
        ARRET.store(false, Ordering::SeqCst);
        let (envoi, reception) = mpsc::channel();
        let lancer = {
            let (reglages, dossier, envoi) =
                (reglages.clone(), dossier.to_path_buf(), envoi.clone());
            move || {
                let (reglages, dossier, envoi) = (reglages.clone(), dossier.clone(), envoi.clone());
                sur_le_fil_principal(move |mtm| commencer(mtm, reglages, dossier, envoi));
            }
        };
        lancer();
        let mut essais = 1;
        let mut numerisation = Numerisation::default();
        let mut resolution = reglages.resolution;
        let mut silence_max = SILENCE_OUVERTURE;
        let mut dernier_signe = Instant::now();
        let mut arret: Option<Instant> = None;
        let fin = loop {
            match reception.recv_timeout(Duration::from_millis(500)) {
                Ok(message) => {
                    dernier_signe = Instant::now();
                    match message {
                        Message::Prete {
                            resolution: reelle,
                            sources,
                        } => {
                            resolution = reelle;
                            numerisation.sources = Some(sources);
                            retenir(&reglages.scanner, sources);
                            silence_max = SILENCE_PAGES;
                        }
                        Message::Page(chemin) => {
                            numerisation
                                .pages
                                .push(PageNumerisee { chemin, resolution });
                            avancement(numerisation.pages.len());
                        }
                        // Un scanner du réseau s'annonce de nouveau, parfois après quelques secondes.
                        Message::Introuvable if essais < ESSAIS_MAX => {
                            essais += 1;
                            std::thread::sleep(Duration::from_secs(1));
                            lancer();
                        }
                        Message::Introuvable => {
                            break Err(Erreur::simple(
                                "Ce scanner n'est plus disponible. Vérifiez qu'il est allumé et \
                                 branché, puis cherchez de nouveau les scanners.",
                            ));
                        }
                        Message::Fin(resultat) => break resultat,
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if numerisation.sources.is_none() {
                        sur_le_fil_principal(|mtm| navigation(mtm).1.demander_l_unite());
                    }
                    if ARRET.load(Ordering::SeqCst) {
                        let depuis = *arret.get_or_insert_with(Instant::now);
                        if depuis.elapsed() > ARRET_MAX {
                            sur_le_fil_principal(|mtm| navigation(mtm).1.terminer(Ok(true)));
                            break Ok(true);
                        }
                    }
                    if dernier_signe.elapsed() > silence_max {
                        // La session est fermée ; plus personne n'attend le résultat.
                        sur_le_fil_principal(|mtm| navigation(mtm).1.terminer(Ok(true)));
                        break Err(Erreur::simple(
                            "Le scanner ne répond plus. Vérifiez qu'il est allumé et branché, \
                             puis recommencez.",
                        ));
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    break Err(Erreur::simple(
                        "La numérisation s'est arrêtée sans réponse du scanner.",
                    ));
                }
            }
        };
        match fin {
            // Certains scanners finissent sans erreur quand on les arrête.
            Ok(annulee) => numerisation.annulee = annulee || ARRET.load(Ordering::SeqCst),
            Err(erreur) if numerisation.pages.is_empty() => return Err(erreur),
            Err(erreur) => numerisation.interrompue = Some(erreur.message),
        }
        Ok(numerisation)
    }

    /// Fil principal : ouvre une session sur le scanner, qui répond au délégué.
    fn commencer(
        mtm: MainThreadMarker,
        reglages: Reglages,
        dossier: PathBuf,
        envoi: Sender<Message>,
    ) {
        let (navigateur, delegue) = navigation(mtm);
        if delegue.ivars().travail.borrow().is_some() {
            let _ = envoi.send(Message::Fin(Err(Erreur::simple(
                "Une numérisation est déjà en cours.",
            ))));
            return;
        }
        // SAFETY : navigateur lu sur le fil principal.
        let scanner = unsafe { navigateur.devices() }.and_then(|appareils| {
            appareils
                .iter()
                .filter(|appareil| est_un_scanner(appareil))
                .find(|appareil| {
                    identifiant(appareil).as_deref() == Some(reglages.scanner.as_str())
                })
                .and_then(|appareil| appareil.downcast::<ICScannerDevice>().ok())
        });
        let Some(scanner) = scanner else {
            let _ = envoi.send(Message::Introuvable);
            return;
        };
        // SAFETY : scanner du navigateur, sur le fil principal.
        let ouverte = unsafe { scanner.hasOpenSession() };
        let voulue = unite_voulue(reglages.source);
        *delegue.ivars().travail.borrow_mut() = Some(Travail {
            scanner: scanner.clone(),
            reglages,
            dossier,
            envoi,
            etape: if ouverte {
                Etape::Choix
            } else {
                Etape::Ouverture
            },
            ouverte: ouverte.then(Instant::now),
            demande: ouverte,
        });
        // SAFETY : scanner du navigateur, sur le fil principal ; son délégué (référence faible) est
        // gardé par `NAVIGATION`.
        unsafe {
            scanner.setDelegate(Some(ProtocolObject::from_ref(&*delegue)));
            if ouverte {
                scanner.requestSelectFunctionalUnit(voulue);
            } else {
                scanner.requestOpenSession();
            }
        }
    }

    pub fn annuler() {
        ARRET.store(true, Ordering::SeqCst);
        sur_le_fil_principal(|mtm| {
            let (_, delegue) = navigation(mtm);
            let en_cours = delegue
                .ivars()
                .travail
                .borrow()
                .as_ref()
                .map(|travail| (travail.scanner.clone(), travail.etape));
            match en_cours {
                // SAFETY : scanner de la numérisation, sur le fil principal.
                Some((scanner, Etape::Numerisation)) => unsafe { scanner.cancelScan() },
                Some(_) => delegue.terminer(Ok(true)),
                None => {}
            }
        });
    }

    fn unite_voulue(source: Source) -> ICScannerFunctionalUnitType {
        match source {
            Source::Vitre => ICScannerFunctionalUnitType::Flatbed,
            Source::Chargeur => ICScannerFunctionalUnitType::DocumentFeeder,
        }
    }

    /// Règle l'unité choisie (couleur, résolution, feuille A4, recto verso) et le transfert des
    /// pages (fichiers dans `dossier`). Rend la résolution réelle et ce que sait faire le scanner.
    fn regler(
        scanner: &ICScannerDevice,
        unite: &ICScannerFunctionalUnit,
        reglages: &Reglages,
        dossier: &Path,
    ) -> (u32, Sources) {
        // SAFETY : unité que le scanner vient de choisir et scanner de la numérisation, réglés sur
        // le fil principal avant la numérisation.
        unsafe {
            let (pixels, profondeur) = match reglages.couleur {
                Couleur::Couleur => (ICScannerPixelDataType::RGB, ICScannerBitDepth::Depth8Bits),
                Couleur::NoirEtBlanc if unite.supportedBitDepths().containsIndex(1) => {
                    (ICScannerPixelDataType::BW, ICScannerBitDepth::Depth1Bit)
                }
                Couleur::Gris | Couleur::NoirEtBlanc => {
                    (ICScannerPixelDataType::Gray, ICScannerBitDepth::Depth8Bits)
                }
            };
            unite.setPixelDataType(pixels);
            unite.setBitDepth(profondeur);
            unite.setResolution(la_plus_proche(
                &unite.supportedResolutions(),
                reglages.resolution as usize,
            ));
            let physique = unite.physicalSize();
            if let Some((largeur, hauteur)) = feuille_a4(unite.measurementUnit()) {
                unite.setScanArea(NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(largeur.min(physique.width), hauteur.min(physique.height)),
                ));
            }
            let mut sources = Sources::default();
            if let Some(chargeur) = unite.downcast_ref::<ICScannerFunctionalUnitDocumentFeeder>() {
                chargeur.setDocumentType(ICScannerDocumentType::TypeA4);
                sources.chargeur = true;
                sources.recto_verso = chargeur.supportsDuplexScanning();
                chargeur.setDuplexScanningEnabled(reglages.recto_verso && sources.recto_verso);
            } else {
                sources.vitre = true;
            }
            // Les autres unités du scanner, quand il les donne (la liste peut manquer).
            let unites: Option<Retained<NSArray<NSNumber>>> =
                msg_send![scanner, availableFunctionalUnitTypes];
            for unite in unites.iter().flat_map(|unites| unites.iter()) {
                match ICScannerFunctionalUnitType(unite.unsignedIntegerValue()) {
                    ICScannerFunctionalUnitType::Flatbed => sources.vitre = true,
                    ICScannerFunctionalUnitType::DocumentFeeder => sources.chargeur = true,
                    _ => {}
                }
            }
            scanner.setTransferMode(ICScannerTransferMode::FileBased);
            scanner.setDownloadsDirectory(&NSURL::fileURLWithPath(&NSString::from_str(
                &dossier.to_string_lossy(),
            )));
            scanner.setDocumentName(&NSString::from_str("page"));
            scanner.setDocumentUTI(&NSString::from_str(
                if pixels == ICScannerPixelDataType::BW {
                    "public.png"
                } else {
                    "public.jpeg"
                },
            ));
            (unite.resolution().max(1) as u32, sources)
        }
    }

    /// La résolution du scanner la plus proche de celle voulue.
    fn la_plus_proche(resolutions: &NSIndexSet, voulue: usize) -> usize {
        let (au_dessus, au_dessous) = (
            resolutions.indexGreaterThanOrEqualToIndex(voulue),
            resolutions.indexLessThanOrEqualToIndex(voulue),
        );
        match (au_dessus != PAS_TROUVE, au_dessous != PAS_TROUVE) {
            (true, true) if au_dessus - voulue <= voulue - au_dessous => au_dessus,
            (_, true) => au_dessous,
            (true, false) => au_dessus,
            (false, false) => voulue,
        }
    }

    /// Une feuille A4 dans l'unité de mesure du scanner.
    fn feuille_a4(unite: ICScannerMeasurementUnit) -> Option<(f64, f64)> {
        const A4_POINTS: (f64, f64) = (595.28, 841.89);
        match unite {
            ICScannerMeasurementUnit::Inches => Some((210.0 / 25.4, 297.0 / 25.4)),
            ICScannerMeasurementUnit::Centimeters => Some((21.0, 29.7)),
            ICScannerMeasurementUnit::Points => Some(A4_POINTS),
            ICScannerMeasurementUnit::Picas => Some((A4_POINTS.0 / 12.0, A4_POINTS.1 / 12.0)),
            ICScannerMeasurementUnit::Twips => Some((A4_POINTS.0 * 20.0, A4_POINTS.1 * 20.0)),
            _ => None,
        }
    }

    fn erreur_du_scanner(erreur: &NSError, contexte: &str) -> Erreur {
        let code = erreur.code();
        let raison = match code {
            -9923 => "le scanner ne répond pas. Vérifiez qu'il est allumé et branché.".to_string(),
            -9925 | -9926 => "il est déjà utilisé, sur cet ordinateur ou sur un autre. \
                              Réessayez quand il sera libre."
                .into(),
            -9945 | -9950 => "macOS n'a pas le logiciel de ce scanner. Installez celui du \
                              fabricant, ou ajoutez le scanner dans Réglages Système › \
                              Imprimantes et scanners."
                .into(),
            -9946 => "macOS installe le logiciel de ce scanner : réessayez dans un instant.".into(),
            -9953 => "ce scanner demande un nom d'utilisateur et un mot de passe, ce \
                      qu'Automat-PDF ne sait pas encore faire."
                .into(),
            _ => format!("{} (code {code})", erreur.localizedDescription()),
        };
        Erreur::simple(format!("{contexte} : {raison}"))
    }
}

#[cfg(windows)]
mod systeme {
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use windows::Devices::Enumeration::DeviceInformation;
    use windows::Devices::Scanners::{
        IImageScannerSourceConfiguration, ImageScanner, ImageScannerColorMode, ImageScannerFormat,
        ImageScannerResolution, ImageScannerScanResult, ImageScannerScanSource,
    };
    use windows::Foundation::Rect;
    use windows::Storage::StorageFolder;
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};
    use windows::core::{HRESULT, HSTRING, Interface};
    use windows_future::{AsyncOperationProgressHandler, IAsyncOperationWithProgress};

    use super::{Couleur, Numerisation, PageNumerisee, Reglages, Scanner, Source, Sources};
    use crate::erreur::Erreur;

    /// La numérisation en cours, pour l'arrêter.
    static EN_COURS: Mutex<Option<IAsyncOperationWithProgress<ImageScannerScanResult, u32>>> =
        Mutex::new(None);
    /// Opération arrêtée par l'utilisateur (ERROR_CANCELLED).
    const ARRETEE: HRESULT = HRESULT(0x800704C7_u32 as i32);

    fn initialiser() {
        // SAFETY : sans effet si ce fil est déjà initialisé (l'erreur est alors ignorée).
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    }

    fn echec(erreur: windows::core::Error) -> Erreur {
        Erreur::simple(message_wia(&erreur))
    }

    /// Les erreurs de WIA les plus courantes, dites simplement.
    fn message_wia(erreur: &windows::core::Error) -> String {
        match erreur.code().0 as u32 {
            0x8021_0002 => "Une feuille est coincée dans le scanner : retirez-la, puis recommencez.".into(),
            0x8021_0003 => "Le chargeur de documents est vide : posez-y les feuilles, puis recommencez.".into(),
            0x8021_0004 => "Le scanner signale un problème de papier : vérifiez les feuilles, puis recommencez.".into(),
            0x8021_0005 => "Le scanner est éteint ou débranché.".into(),
            0x8021_0006 => "Le scanner est occupé : réessayez dans un instant.".into(),
            0x8021_0007 => "Le scanner chauffe : réessayez dans un instant.".into(),
            0x8021_0008 => "Le scanner demande une intervention : regardez son écran (capot, papier…).".into(),
            0x8021_000A => "La communication avec le scanner a échoué : vérifiez son câble ou le réseau.".into(),
            0x8021_000D => "Le scanner est verrouillé.".into(),
            0x8021_0015 => "Aucun scanner n'est disponible : vérifiez qu'il est allumé et branché.".into(),
            0x8021_0016 => "Le capot du scanner est ouvert : refermez-le, puis recommencez.".into(),
            0x8021_0017 => "La lampe du scanner est éteinte.".into(),
            _ => format!("La numérisation a échoué : {}", erreur.message()),
        }
    }

    fn sources_de(scanner: &ImageScanner) -> Sources {
        let vitre = scanner
            .IsScanSourceSupported(ImageScannerScanSource::Flatbed)
            .unwrap_or(false);
        let chargeur = scanner
            .IsScanSourceSupported(ImageScannerScanSource::Feeder)
            .unwrap_or(false);
        let recto_verso = chargeur
            && scanner
                .FeederConfiguration()
                .and_then(|chargeur| chargeur.CanScanDuplex())
                .unwrap_or(false);
        Sources {
            vitre,
            chargeur,
            recto_verso,
        }
    }

    pub fn scanners() -> Result<Vec<Scanner>, Erreur> {
        initialiser();
        let selecteur = ImageScanner::GetDeviceSelector().map_err(echec)?;
        let appareils = DeviceInformation::FindAllAsyncAqsFilter(&selecteur)
            .and_then(|operation| operation.join())
            .map_err(echec)?;
        Ok(appareils
            .into_iter()
            .filter_map(|appareil| {
                let id = appareil.Id().ok()?;
                let nom = appareil
                    .Name()
                    .map(|nom| nom.to_string())
                    .unwrap_or_else(|_| "Scanner".into());
                let sources = ImageScanner::FromIdAsync(&id)
                    .and_then(|operation| operation.join())
                    .ok()
                    .map(|scanner| sources_de(&scanner));
                Some(Scanner {
                    id: id.to_string(),
                    nom,
                    sources,
                })
            })
            .collect())
    }

    /// Règle la vitre ou le chargeur : couleur, résolution, feuille A4, format des fichiers. Rend la
    /// résolution réelle.
    fn regler(
        reglage: &IImageScannerSourceConfiguration,
        reglages: &Reglages,
    ) -> windows::core::Result<u32> {
        let mode = match reglages.couleur {
            Couleur::Couleur => ImageScannerColorMode::Color,
            Couleur::Gris => ImageScannerColorMode::Grayscale,
            Couleur::NoirEtBlanc => ImageScannerColorMode::Monochrome,
        };
        if reglage.IsColorModeSupported(mode).unwrap_or(false) {
            reglage.SetColorMode(mode)?;
        }
        let (min, max) = (reglage.MinResolution()?, reglage.MaxResolution()?);
        let voulue =
            (reglages.resolution as f32).clamp(min.DpiX.min(max.DpiX), max.DpiX.max(min.DpiX));
        reglage.SetDesiredResolution(ImageScannerResolution {
            DpiX: voulue,
            DpiY: voulue,
        })?;
        // Une feuille A4, dans la limite de la vitre (en pouces).
        if let Ok(zone) = reglage.MaxScanArea() {
            let _ = reglage.SetSelectedScanRegion(Rect {
                X: 0.0,
                Y: 0.0,
                Width: zone.Width.min(210.0 / 25.4),
                Height: zone.Height.min(297.0 / 25.4),
            });
        }
        let formats: &[ImageScannerFormat] = if mode == ImageScannerColorMode::Monochrome {
            &[
                ImageScannerFormat::Png,
                ImageScannerFormat::Tiff,
                ImageScannerFormat::DeviceIndependentBitmap,
                ImageScannerFormat::Jpeg,
            ]
        } else {
            &[
                ImageScannerFormat::Jpeg,
                ImageScannerFormat::Png,
                ImageScannerFormat::Tiff,
                ImageScannerFormat::DeviceIndependentBitmap,
            ]
        };
        if let Some(format) = formats
            .iter()
            .copied()
            .find(|format| reglage.IsFormatSupported(*format).unwrap_or(false))
        {
            reglage.SetFormat(format)?;
        }
        Ok(reglage
            .ActualResolution()
            .map(|reelle| reelle.DpiX.round() as u32)
            .unwrap_or(reglages.resolution)
            .max(1))
    }

    /// Les images déjà écrites dans le dossier, dans l'ordre (numérisation arrêtée en route).
    fn pages_du_dossier(dossier: &Path, resolution: u32) -> Vec<PageNumerisee> {
        let mut pages: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(dossier)
            .map(|entrees| {
                entrees
                    .flatten()
                    .filter_map(|entree| {
                        let date = entree.metadata().and_then(|m| m.modified()).ok()?;
                        Some((date, entree.path().to_string_lossy().into_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        pages.sort();
        pages
            .into_iter()
            .map(|(_, chemin)| PageNumerisee { chemin, resolution })
            .collect()
    }

    pub fn numeriser(
        reglages: &Reglages,
        dossier: &Path,
        avancement: Arc<dyn Fn(usize) + Send + Sync>,
    ) -> Result<Numerisation, Erreur> {
        initialiser();
        let scanner = ImageScanner::FromIdAsync(&HSTRING::from(reglages.scanner.as_str()))
            .and_then(|operation| operation.join())
            .map_err(|_| {
                Erreur::simple(
                    "Ce scanner n'est plus disponible. Vérifiez qu'il est allumé et branché, puis \
                     cherchez de nouveau les scanners.",
                )
            })?;
        let sources = sources_de(&scanner);
        let (source, reglage): (ImageScannerScanSource, IImageScannerSourceConfiguration) =
            match reglages.source {
                Source::Vitre if sources.vitre => (
                    ImageScannerScanSource::Flatbed,
                    scanner
                        .FlatbedConfiguration()
                        .and_then(|vitre| vitre.cast())
                        .map_err(echec)?,
                ),
                Source::Chargeur if sources.chargeur => {
                    let chargeur = scanner.FeederConfiguration().map_err(echec)?;
                    let _ = chargeur.SetDuplex(reglages.recto_verso && sources.recto_verso);
                    // 0 : toutes les feuilles du chargeur.
                    let _ = chargeur.SetMaxNumberOfPages(0);
                    (
                        ImageScannerScanSource::Feeder,
                        chargeur.cast().map_err(echec)?,
                    )
                }
                Source::Vitre => {
                    return Err(Erreur::simple(
                        "Ce scanner n'a pas de vitre : choisissez le chargeur de documents.",
                    ));
                }
                Source::Chargeur => {
                    return Err(Erreur::simple(
                        "Ce scanner n'a pas de chargeur de documents : choisissez la vitre.",
                    ));
                }
            };
        let resolution = regler(&reglage, reglages).map_err(echec)?;
        let dossier_windows =
            StorageFolder::GetFolderFromPathAsync(&HSTRING::from(dossier.as_os_str()))
                .and_then(|operation| operation.join())
                .map_err(echec)?;
        let operation = scanner
            .ScanFilesToFolderAsync(source, &dossier_windows)
            .map_err(echec)?;
        let _ = operation.SetProgress(&AsyncOperationProgressHandler::new(
            move |_, pages: windows::core::Ref<u32>| {
                avancement(*pages as usize);
                Ok(())
            },
        ));
        if let Ok(mut en_cours) = EN_COURS.lock() {
            *en_cours = Some(operation.clone());
        }
        let resultat = operation.join();
        if let Ok(mut en_cours) = EN_COURS.lock() {
            *en_cours = None;
        }
        let mut numerisation = Numerisation {
            sources: Some(sources),
            ..Numerisation::default()
        };
        match resultat {
            Ok(resultat) => {
                for fichier in resultat.ScannedFiles().map_err(echec)? {
                    if let Ok(chemin) = fichier.Path() {
                        numerisation.pages.push(PageNumerisee {
                            chemin: chemin.to_string(),
                            resolution,
                        });
                    }
                }
            }
            // Arrêtée, ou interrompue (feuille coincée…) : les pages déjà écrites sont gardées.
            Err(erreur) => {
                numerisation.pages = pages_du_dossier(dossier, resolution);
                if erreur.code() == ARRETEE {
                    numerisation.annulee = true;
                } else {
                    numerisation.interrompue = Some(message_wia(&erreur));
                }
            }
        }
        Ok(numerisation)
    }

    pub fn annuler() {
        if let Ok(en_cours) = EN_COURS.lock()
            && let Some(operation) = en_cours.as_ref()
        {
            let _ = operation.Cancel();
        }
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod systeme {
    use std::path::Path;
    use std::sync::Arc;

    use super::{Numerisation, Reglages, Scanner};
    use crate::erreur::Erreur;

    pub fn scanners() -> Result<Vec<Scanner>, Erreur> {
        Ok(Vec::new())
    }

    pub fn numeriser(
        _: &Reglages,
        _: &Path,
        _: Arc<dyn Fn(usize) + Send + Sync>,
    ) -> Result<Numerisation, Erreur> {
        Err(Erreur::simple(
            "La numérisation n'est pas disponible sur ce système.",
        ))
    }

    pub fn annuler() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg(largeur: u32, hauteur: u32) -> Vec<u8> {
        let mut octets = Vec::new();
        image::DynamicImage::new_rgb8(largeur, hauteur)
            .write_to(
                &mut std::io::Cursor::new(&mut octets),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        octets
    }

    #[test]
    fn fait_le_pdf_des_pages_a_leur_taille_reelle() {
        let travail =
            std::env::temp_dir().join(format!("automat-pdf-numerisation-{}", std::process::id()));
        let dossier = nouveau_dossier(&travail).unwrap();
        let autre = nouveau_dossier(&travail).unwrap();
        assert_ne!(dossier, autre);
        // Deux feuilles A4 numérisées, l'une à 30 ppp, l'autre à 15 ppp et tournée.
        std::fs::write(dossier.join("page.jpeg"), jpeg(248, 351)).unwrap();
        std::fs::write(dossier.join("page 1.jpeg"), jpeg(124, 175)).unwrap();
        let pages = [
            PagePdf {
                chemin: dossier.join("page.jpeg").to_string_lossy().into_owned(),
                rotation: 0,
                resolution: 30,
            },
            PagePdf {
                chemin: dossier.join("page 1.jpeg").to_string_lossy().into_owned(),
                rotation: 90,
                resolution: 15,
            },
        ];
        let sortie = travail.join("Numérisation.pdf");
        let sortie = sortie.to_string_lossy();
        let bilan = enregistrer_pdf(&pages, false, &sortie, &|_, _| ()).unwrap();
        assert_eq!(
            (bilan.pages, bilan.texte, bilan.sans_texte),
            (2, None, None)
        );
        let document = lopdf::Document::load(&*sortie).unwrap();
        let boites: Vec<Vec<f32>> = document
            .get_pages()
            .values()
            .map(|id| {
                document
                    .get_dictionary(*id)
                    .unwrap()
                    .get(b"MediaBox")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|nombre| nombre.as_float().unwrap())
                    .collect()
            })
            .collect();
        assert!((boites[0][2] - 595.2).abs() < 0.01 && (boites[0][3] - 842.4).abs() < 0.01);
        assert!((boites[1][2] - 595.2).abs() < 0.01 && (boites[1][3] - 840.0).abs() < 0.01);

        // Une page blanche : pas de texte à reconnaître, le PDF est enregistré quand même.
        let bilan = enregistrer_pdf(&pages[..1], true, &sortie, &|_, _| ()).unwrap();
        assert!(bilan.sans_texte.is_some(), "{bilan:?}");
        assert!(lopdf::Document::load(&*sortie).is_ok());

        // Les pages effacées, rien ne reste.
        oublier(&travail).unwrap();
        assert!(!travail.exists());
        oublier(&travail).unwrap();
        assert!(enregistrer_pdf(&pages, false, &sortie, &|_, _| ()).is_err());
        assert!(enregistrer_pdf(&[], false, &sortie, &|_, _| ()).is_err());
    }
}
