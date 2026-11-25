//! Impression d'un PDF depuis le lecteur.
//!
//! - **Mac** : PDFKit imprime le PDF lui-même, net à toute résolution, par la fenêtre d'impression
//!   de macOS (imprimante, pages, copies, recto verso, aperçu).
//! - **Windows** : la fenêtre d'impression d'Automat-PDF (aperçu, imprimante, pages, copies,
//!   couleur ou noir et blanc, recto verso, taille, orientation, papier), puis chaque page choisie,
//!   dessinée par hayro à la résolution de l'imprimante (300 ppp au plus) et envoyée par GDI. Les
//!   réglages avancés du pilote restent à un clic (« Propriétés de l'imprimante… »). La fenêtre
//!   classique de Windows n'a pas d'aperçu, et ses réglages de couleur dépendent de chaque pilote.
//!
//! Le PDF imprimé est celui que montre le lecteur : déchiffré s'il a été ouvert avec son mot de
//! passe, avec les annotations en cours (`operations::imprimer_pdf`).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, WebviewWindow};

use crate::erreur::Erreur;

/// Résolution de dessin des pages au plus (ppp) : au-delà, des images très lourdes pour un gain
/// invisible sur un document de bureau.
#[cfg_attr(not(windows), allow(dead_code))]
const PPP_MAX: u32 = 300;

/// Événement d'avancement, pendant l'envoi des pages à l'imprimante (Windows).
#[cfg_attr(not(windows), allow(dead_code))]
pub const EVENEMENT_AVANCEMENT: &str = "impression-avancement";

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(not(windows), allow(dead_code))]
pub struct Avancement {
    pub faites: usize,
    pub total: usize,
}

/// Un format de papier de l'imprimante, en millimètres.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Papier {
    /// Code du format pour Windows (DMPAPER_…, 9 : A4).
    pub code: i16,
    pub nom: String,
    pub largeur: f64,
    pub hauteur: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RectoVerso {
    Non,
    BordLong,
    BordCourt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ajustement {
    /// Taille réelle ; une page plus grande que la feuille est réduite.
    Reduire,
    /// Taille réelle, toujours.
    Reelle,
    /// Agrandie ou réduite pour remplir la feuille.
    Ajuster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Orientation {
    /// Chaque page tournée pour suivre le sens de la feuille.
    Auto,
    Portrait,
    Paysage,
}

/// Réglages de l'imprimante que la fenêtre d'Automat-PDF affiche : ceux par défaut, ou ceux laissés
/// par « Propriétés de l'imprimante… ».
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reglages {
    pub couleur: bool,
    pub recto_verso: RectoVerso,
    pub papier: Option<i16>,
    pub paysage: bool,
    /// Réglages complets du pilote (DEVMODE), à rendre pour l'impression ; vide : ceux par défaut.
    pub pilote: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imprimante {
    pub nom: String,
    pub par_defaut: bool,
    /// L'imprimante sait imprimer en couleur, et en recto verso.
    pub couleur: bool,
    pub recto_verso: bool,
    pub papiers: Vec<Papier>,
    pub reglages: Reglages,
}

/// La feuille, pour l'aperçu : taille et marges non imprimables (gauche, haut, droite, bas), en
/// millimètres, dans le sens choisi.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Feuille {
    pub largeur: f64,
    pub hauteur: f64,
    pub marges: [f64; 4],
}

/// Ce qui a été choisi dans la fenêtre d'impression d'Automat-PDF (Windows).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsImpression {
    pub imprimante: String,
    /// Pages à imprimer (à partir de 0), dans l'ordre ; vide : toutes.
    #[serde(default)]
    pub pages: Vec<usize>,
    pub copies: u32,
    pub assembler: bool,
    pub couleur: bool,
    pub recto_verso: RectoVerso,
    pub ajustement: Ajustement,
    pub orientation: Orientation,
    pub papier: Option<i16>,
    /// Réglages complets du pilote, après « Propriétés de l'imprimante… » ; vide : ceux par défaut.
    #[serde(default)]
    pub pilote: Vec<u8>,
}

/// Imprime le PDF : sur Mac par la fenêtre de macOS (les options n'y servent pas), sous Windows avec
/// les options de la fenêtre d'Automat-PDF. `Ok(false)` : l'utilisateur a annulé.
pub fn imprimer(
    app: &AppHandle,
    fenetre: &WebviewWindow,
    octets: Arc<Vec<u8>>,
    nom: &str,
    options: Option<&OptionsImpression>,
) -> Result<bool, Erreur> {
    systeme::imprimer(app, fenetre, octets, nom, options)
}

/// Les imprimantes de l'ordinateur, celle par défaut en tête (Windows).
pub fn imprimantes() -> Result<Vec<Imprimante>, Erreur> {
    systeme::imprimantes()
}

/// La feuille telle que l'imprimante la prendra, pour l'aperçu (Windows).
pub fn feuille(options: &OptionsImpression) -> Result<Feuille, Erreur> {
    systeme::feuille(options)
}

/// La fenêtre de réglages du pilote de l'imprimante ; `None` : annulée (Windows).
pub fn proprietes(
    app: &AppHandle,
    fenetre: &WebviewWindow,
    imprimante: &str,
    pilote: &[u8],
) -> Result<Option<Reglages>, Erreur> {
    systeme::proprietes(app, fenetre, imprimante, pilote)
}

/// Pages à envoyer quand le pilote ne fait pas lui-même les copies : le document entier `copies`
/// fois (assemblées), ou chaque page `copies` fois. En recto verso assemblé, une page blanche
/// (`None`) termine une copie impaire, pour que la suivante commence sur une nouvelle feuille.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn sequence(
    pages: &[usize],
    copies: u32,
    assembler: bool,
    recto_verso: bool,
) -> Vec<Option<usize>> {
    let copies = copies.max(1) as usize;
    if !assembler {
        return pages
            .iter()
            .flat_map(|&page| std::iter::repeat_n(Some(page), copies))
            .collect();
    }
    let mut suite = Vec::new();
    for copie in 0..copies {
        suite.extend(pages.iter().map(|&page| Some(page)));
        if recto_verso && pages.len() % 2 == 1 && copie + 1 < copies {
            suite.push(None);
        }
    }
    suite
}

/// Image tournée d'un quart de tour dans le sens des aiguilles d'une montre (pixels RVB).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn quart_de_tour(largeur: u32, hauteur: u32, rvb: &[u8]) -> (u32, u32, Vec<u8>) {
    let (l, h) = (largeur as usize, hauteur as usize);
    let mut tournee = vec![0u8; rvb.len()];
    for y in 0..h {
        for x in 0..l {
            // Le pixel (x, y) va en (h − 1 − y, x) dans l'image de largeur h.
            let source = (y * l + x) * 3;
            let cible = (x * h + (h - 1 - y)) * 3;
            tournee[cible..cible + 3].copy_from_slice(&rvb[source..source + 3]);
        }
    }
    (hauteur, largeur, tournee)
}

/// Pixels RVB en niveaux de gris (luminance), pour le noir et blanc même si le pilote l'ignore.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn en_gris(rvb: &mut [u8]) {
    for pixel in rvb.as_chunks_mut::<3>().0 {
        let [rouge, vert, bleu] = pixel.map(u32::from);
        let gris = ((299 * rouge + 587 * vert + 114 * bleu + 500) / 1000) as u8;
        *pixel = [gris; 3];
    }
}

/// Pixels RVB vers le format des images de Windows (DIB 24 bits) : bleu, vert, rouge, chaque ligne
/// complétée jusqu'à un multiple de 4 octets.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn en_dib(largeur: u32, hauteur: u32, rvb: &[u8]) -> Vec<u8> {
    let ligne = (largeur as usize * 3).div_ceil(4) * 4;
    let mut dib = vec![0u8; ligne * hauteur as usize];
    for (y, pixels) in rvb.chunks_exact(largeur as usize * 3).enumerate() {
        for (x, &[rouge, vert, bleu]) in pixels.as_chunks::<3>().0.iter().enumerate() {
            let cible = y * ligne + x * 3;
            dib[cible..cible + 3].copy_from_slice(&[bleu, vert, rouge]);
        }
    }
    dib
}

/// La page doit-elle être tournée d'un quart de tour ? En orientation automatique, quand elle n'a
/// pas le sens de la feuille ; jamais quand l'orientation est imposée.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn a_tourner(page: (u32, u32), zone: (i32, i32), orientation: Orientation) -> bool {
    orientation == Orientation::Auto && page.0 != page.1 && (page.0 > page.1) != (zone.0 > zone.1)
}

/// Place d'une page dessinée à `ppp_image` sur la zone imprimable (pixels de l'imprimante, qui
/// peuvent ne pas être carrés), selon l'ajustement, centrée. Renvoie (x, y, largeur, hauteur).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn place_sur_la_feuille(
    image: (u32, u32),
    ppp_image: u32,
    zone: (i32, i32),
    ppp_imprimante: (u32, u32),
    ajustement: Ajustement,
) -> (i32, i32, i32, i32) {
    let ppp_image = f64::from(ppp_image.max(1));
    let largeur = f64::from(image.0) * f64::from(ppp_imprimante.0) / ppp_image;
    let hauteur = f64::from(image.1) * f64::from(ppp_imprimante.1) / ppp_image;
    let remplir = (f64::from(zone.0) / largeur).min(f64::from(zone.1) / hauteur);
    let echelle = match ajustement {
        Ajustement::Reduire => remplir.min(1.0),
        Ajustement::Reelle => 1.0,
        Ajustement::Ajuster => remplir,
    };
    let (l, h) = (
        (largeur * echelle).round() as i32,
        (hauteur * echelle).round() as i32,
    );
    ((zone.0 - l) / 2, (zone.1 - h) / 2, l, h)
}

#[cfg(target_os = "macos")]
mod systeme {
    use super::*;
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSPrintInfo, NSPrintPanelOptions};
    use objc2_foundation::{NSData, NSString};
    use objc2_pdf_kit::{PDFDocument, PDFPrintScalingMode};

    pub fn imprimer(
        app: &AppHandle,
        _fenetre: &WebviewWindow,
        octets: Arc<Vec<u8>>,
        nom: &str,
        _options: Option<&OptionsImpression>,
    ) -> Result<bool, Erreur> {
        let (envoi, reception) = std::sync::mpsc::channel();
        let titre = nom.to_string();
        // AppKit ne s'utilise que depuis le fil principal ; la fenêtre d'impression y attend
        // l'utilisateur, l'application restant à l'écran.
        app.run_on_main_thread(move || {
            let _ = envoi.send(sur_le_fil_principal(&octets, &titre));
        })
        .map_err(|erreur| {
            Erreur::simple(format!("L'impression n'a pas pu commencer : {erreur}"))
        })?;
        reception
            .recv()
            .map_err(|_| Erreur::simple("L'impression n'a pas pu commencer."))?
    }

    fn sur_le_fil_principal(octets: &[u8], titre: &str) -> Result<bool, Erreur> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| Erreur::simple("L'impression n'a pas pu commencer (fil principal)."))?;
        let donnees = NSData::with_bytes(octets);
        // SAFETY : PDFKit recopie les octets du PDF, valables pendant l'appel.
        let document = unsafe { PDFDocument::initWithData(PDFDocument::alloc(), &donnees) }
            .ok_or_else(|| {
                Erreur::simple(format!(
                    "« {titre} » n'a pas pu être préparé pour l'impression."
                ))
            })?;
        let reglages = NSPrintInfo::sharedPrintInfo();
        // SAFETY : document ouvert ci-dessus, réglages d'impression partagés de l'application.
        let operation = unsafe {
            document.printOperationForPrintInfo_scalingMode_autoRotate(
                Some(&reglages),
                PDFPrintScalingMode::PageScaleDownToFit,
                true,
                mtm,
            )
        }
        .ok_or_else(|| Erreur::simple(format!("« {titre} » ne peut pas être imprimé.")))?;
        operation.setJobTitle(Some(&NSString::from_str(titre)));
        operation.setShowsPrintPanel(true);
        operation.setShowsProgressPanel(true);
        // L'aperçu, le papier, l'orientation et l'échelle, en plus des copies et des pages.
        let panneau = operation.printPanel();
        panneau.setOptions(
            panneau.options()
                | NSPrintPanelOptions::ShowsCopies
                | NSPrintPanelOptions::ShowsPageRange
                | NSPrintPanelOptions::ShowsPaperSize
                | NSPrintPanelOptions::ShowsOrientation
                | NSPrintPanelOptions::ShowsScaling
                | NSPrintPanelOptions::ShowsPreview,
        );
        Ok(operation.runOperation())
    }

    pub fn imprimantes() -> Result<Vec<Imprimante>, Erreur> {
        Err(sous_windows())
    }

    pub fn feuille(_options: &OptionsImpression) -> Result<Feuille, Erreur> {
        Err(sous_windows())
    }

    pub fn proprietes(
        _app: &AppHandle,
        _fenetre: &WebviewWindow,
        _imprimante: &str,
        _pilote: &[u8],
    ) -> Result<Option<Reglages>, Erreur> {
        Err(sous_windows())
    }

    fn sous_windows() -> Erreur {
        Erreur::simple("Sur Mac, l'impression passe par la fenêtre d'impression de macOS.")
    }
}

#[cfg(windows)]
mod systeme {
    use super::*;
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};

    use tauri::Emitter;
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDCW, DEVMODEW, DIB_RGB_COLORS, DM_COLLATE,
        DM_COLOR, DM_COPIES, DM_DUPLEX, DM_IN_BUFFER, DM_IN_PROMPT, DM_ORIENTATION, DM_OUT_BUFFER,
        DM_PAPERSIZE, DMCOLLATE_FALSE, DMCOLLATE_TRUE, DMCOLOR_COLOR, DMCOLOR_MONOCHROME,
        DMDUP_HORIZONTAL, DMDUP_SIMPLEX, DMDUP_VERTICAL, DMORIENT_LANDSCAPE, DMORIENT_PORTRAIT,
        DeleteDC, GetDeviceCaps, HALFTONE, HDC, HORZRES, LOGPIXELSX, LOGPIXELSY, PHYSICALHEIGHT,
        PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, SRCCOPY, SetBrushOrgEx, SetStretchBltMode,
        StretchDIBits, VERTRES,
    };
    use windows_sys::Win32::Graphics::Printing::{
        ClosePrinter, DocumentPropertiesW, EnumPrintersW, GetDefaultPrinterW, OpenPrinterW,
        PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_HANDLE, PRINTER_INFO_4W,
    };
    use windows_sys::Win32::Storage::Xps::{
        AbortDoc, DC_COLLATE, DC_COLORDEVICE, DC_COPIES, DC_DUPLEX, DC_PAPERNAMES, DC_PAPERS,
        DC_PAPERSIZE, DOCINFOW, DeviceCapabilitiesW, EndDoc, EndPage, StartDocW, StartPage,
    };

    /// Réponse de la fenêtre du pilote quand l'utilisateur l'a fermée par « Annuler ».
    const ANNULEE: i32 = 2;

    fn large(texte: &str) -> Vec<u16> {
        texte.encode_utf16().chain([0]).collect()
    }

    /// Texte terminé par un zéro, écrit par Windows.
    unsafe fn lire_large(texte: *const u16) -> String {
        if texte.is_null() {
            return String::new();
        }
        // SAFETY : chaîne terminée par un zéro, rendue par Windows.
        unsafe {
            let mut longueur = 0;
            while *texte.add(longueur) != 0 {
                longueur += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(texte, longueur))
        }
    }

    /// Mémoire alignée pour les structures de Windows (DEVMODEW, PRINTER_INFO_4W).
    struct Tampon(Vec<u64>);

    impl Tampon {
        fn new(octets: usize) -> Self {
            Tampon(vec![0; octets.div_ceil(8).max(1)])
        }

        fn depuis(octets: &[u8]) -> Self {
            let mut tampon = Tampon::new(octets.len());
            tampon.octets_mut()[..octets.len()].copy_from_slice(octets);
            tampon
        }

        fn ptr<T>(&mut self) -> *mut T {
            self.0.as_mut_ptr() as *mut T
        }

        fn octets_mut(&mut self) -> &mut [u8] {
            // SAFETY : la mémoire du vecteur, vue octet par octet.
            unsafe {
                std::slice::from_raw_parts_mut(self.0.as_mut_ptr() as *mut u8, self.0.len() * 8)
            }
        }

        fn octets(&self, longueur: usize) -> Vec<u8> {
            // SAFETY : la mémoire du vecteur, vue octet par octet.
            unsafe {
                std::slice::from_raw_parts(
                    self.0.as_ptr() as *const u8,
                    longueur.min(self.0.len() * 8),
                )
            }
            .to_vec()
        }
    }

    /// Une imprimante ouverte, refermée à la fin.
    struct Ouverte(PRINTER_HANDLE);

    impl Ouverte {
        fn ouvrir(nom: &[u16]) -> Result<Self, Erreur> {
            let mut imprimante = PRINTER_HANDLE::default();
            // SAFETY : nom terminé par un zéro ; poignée rendue par Windows.
            if unsafe { OpenPrinterW(nom.as_ptr(), &mut imprimante, null()) } == 0 {
                return Err(Erreur::simple("L'imprimante n'a pas pu être ouverte."));
            }
            Ok(Ouverte(imprimante))
        }
    }

    impl Drop for Ouverte {
        fn drop(&mut self) {
            // SAFETY : poignée ouverte par OpenPrinterW.
            unsafe { ClosePrinter(self.0) };
        }
    }

    /// Réglages complets du pilote (DEVMODE) : ceux donnés, sinon ceux par défaut ; et leur taille.
    fn pilote(imprimante: &Ouverte, nom: &[u16], donnes: &[u8]) -> Result<(Tampon, usize), Erreur> {
        let illisibles = || Erreur::simple("Les réglages de l'imprimante n'ont pas pu être lus.");
        // SAFETY : sans tampon, Windows rend la taille des réglages.
        let taille = unsafe {
            DocumentPropertiesW(
                null_mut(),
                imprimante.0,
                nom.as_ptr(),
                null_mut(),
                null(),
                0,
            )
        };
        if taille <= 0 {
            return Err(illisibles());
        }
        let taille = taille as usize;
        let mut sortie = Tampon::new(taille.max(donnes.len()));
        let retour = if donnes.len() >= std::mem::size_of::<DEVMODEW>() {
            let entree = Tampon::depuis(donnes);
            // SAFETY : tampons alignés, d'au moins la taille demandée par le pilote.
            unsafe {
                DocumentPropertiesW(
                    null_mut(),
                    imprimante.0,
                    nom.as_ptr(),
                    sortie.ptr(),
                    entree.0.as_ptr() as *const DEVMODEW,
                    DM_IN_BUFFER | DM_OUT_BUFFER,
                )
            }
        } else {
            // SAFETY : tampon aligné, de la taille demandée par le pilote.
            unsafe {
                DocumentPropertiesW(
                    null_mut(),
                    imprimante.0,
                    nom.as_ptr(),
                    sortie.ptr(),
                    null(),
                    DM_OUT_BUFFER,
                )
            }
        };
        if retour < 0 {
            return Err(illisibles());
        }
        Ok((sortie, taille))
    }

    fn lire_reglages(dm: &DEVMODEW, pilote: Vec<u8>) -> Reglages {
        // SAFETY : réglages d'imprimante : la partie « papier » de l'union.
        let general = unsafe { dm.Anonymous1.Anonymous1 };
        Reglages {
            couleur: dm.dmFields & DM_COLOR == 0 || dm.dmColor == DMCOLOR_COLOR,
            recto_verso: match dm.dmDuplex {
                DMDUP_VERTICAL if dm.dmFields & DM_DUPLEX != 0 => RectoVerso::BordLong,
                DMDUP_HORIZONTAL if dm.dmFields & DM_DUPLEX != 0 => RectoVerso::BordCourt,
                _ => RectoVerso::Non,
            },
            papier: (dm.dmFields & DM_PAPERSIZE != 0 && general.dmPaperSize > 0)
                .then_some(general.dmPaperSize),
            paysage: dm.dmFields & DM_ORIENTATION != 0
                && general.dmOrientation == DMORIENT_LANDSCAPE as i16,
            pilote,
        }
    }

    /// Les choix de la fenêtre d'Automat-PDF, portés dans les réglages du pilote.
    fn appliquer(
        dm: &mut DEVMODEW,
        options: &OptionsImpression,
        copies_pilote: bool,
        recto_verso: bool,
    ) {
        dm.dmFields |= DM_COLOR | DM_ORIENTATION | DM_COLLATE | DM_COPIES;
        dm.dmColor = if options.couleur {
            DMCOLOR_COLOR
        } else {
            DMCOLOR_MONOCHROME
        };
        dm.dmCollate = if options.assembler {
            DMCOLLATE_TRUE
        } else {
            DMCOLLATE_FALSE
        };
        if recto_verso {
            dm.dmFields |= DM_DUPLEX;
            dm.dmDuplex = match options.recto_verso {
                RectoVerso::Non => DMDUP_SIMPLEX,
                RectoVerso::BordLong => DMDUP_VERTICAL,
                RectoVerso::BordCourt => DMDUP_HORIZONTAL,
            };
        }
        // SAFETY : réglages d'imprimante : la partie « papier » de l'union.
        let general = unsafe { &mut dm.Anonymous1.Anonymous1 };
        general.dmOrientation = if options.orientation == Orientation::Paysage {
            DMORIENT_LANDSCAPE as i16
        } else {
            DMORIENT_PORTRAIT as i16
        };
        general.dmCopies = if copies_pilote {
            options.copies.clamp(1, i16::MAX as u32) as i16
        } else {
            1
        };
        if let Some(papier) = options.papier {
            dm.dmFields |= DM_PAPERSIZE;
            general.dmPaperSize = papier;
        }
    }

    fn par_defaut() -> Option<String> {
        let mut longueur = 0u32;
        // SAFETY : sans tampon, Windows rend la longueur du nom.
        unsafe { GetDefaultPrinterW(null_mut(), &mut longueur) };
        if longueur == 0 {
            return None;
        }
        let mut nom = vec![0u16; longueur as usize];
        // SAFETY : tampon de la longueur demandée.
        if unsafe { GetDefaultPrinterW(nom.as_mut_ptr(), &mut longueur) } == 0 {
            return None;
        }
        let fin = nom.iter().position(|&c| c == 0).unwrap_or(nom.len());
        Some(String::from_utf16_lossy(&nom[..fin]))
    }

    pub fn imprimantes() -> Result<Vec<Imprimante>, Erreur> {
        let drapeaux = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let (mut besoin, mut nombre) = (0u32, 0u32);
        // SAFETY : sans tampon, Windows rend la taille de la liste.
        unsafe { EnumPrintersW(drapeaux, null(), 4, null_mut(), 0, &mut besoin, &mut nombre) };
        if besoin == 0 {
            return Ok(Vec::new());
        }
        let mut tampon = Tampon::new(besoin as usize);
        // SAFETY : tampon aligné, de la taille demandée.
        if unsafe {
            EnumPrintersW(
                drapeaux,
                null(),
                4,
                tampon.ptr(),
                besoin,
                &mut besoin,
                &mut nombre,
            )
        } == 0
        {
            return Err(Erreur::simple(
                "La liste des imprimantes n'a pas pu être lue.",
            ));
        }
        // SAFETY : Windows a écrit `nombre` structures au début du tampon.
        let infos = unsafe {
            std::slice::from_raw_parts(
                tampon.ptr::<PRINTER_INFO_4W>() as *const PRINTER_INFO_4W,
                nombre as usize,
            )
        };
        let defaut = par_defaut();
        let mut liste: Vec<Imprimante> = infos
            .iter()
            // SAFETY : noms écrits par Windows dans le tampon, encore vivant.
            .map(|info| unsafe { lire_large(info.pPrinterName) })
            .filter(|nom| !nom.is_empty())
            .map(|nom| {
                let par_defaut = defaut.as_deref() == Some(nom.as_str());
                decrire(&nom, par_defaut)
            })
            .collect();
        liste.sort_by_key(|imprimante| !imprimante.par_defaut);
        Ok(liste)
    }

    fn capacite(nom: &[u16], quoi: u16, sortie: *mut u16) -> i32 {
        // SAFETY : nom terminé par un zéro ; `sortie` nul, ou assez grand (taille demandée avant).
        unsafe { DeviceCapabilitiesW(nom.as_ptr(), null(), quoi, sortie, null()) }
    }

    fn decrire(nom: &str, par_defaut: bool) -> Imprimante {
        let large = large(nom);
        let couleur = capacite(&large, DC_COLORDEVICE, null_mut()) == 1;
        let recto_verso = capacite(&large, DC_DUPLEX, null_mut()) == 1;
        let reglages = Ouverte::ouvrir(&large)
            .and_then(|imprimante| pilote(&imprimante, &large, &[]))
            // SAFETY : réglages rendus par le pilote.
            .map(|(tampon, _)| {
                lire_reglages(
                    unsafe { &*(tampon.0.as_ptr() as *const DEVMODEW) },
                    Vec::new(),
                )
            })
            .unwrap_or(Reglages {
                couleur,
                recto_verso: RectoVerso::Non,
                papier: None,
                paysage: false,
                pilote: Vec::new(),
            });
        Imprimante {
            nom: nom.to_string(),
            par_defaut,
            couleur,
            recto_verso,
            papiers: papiers(&large),
            reglages: Reglages {
                couleur: couleur && reglages.couleur,
                ..reglages
            },
        }
    }

    fn papiers(nom: &[u16]) -> Vec<Papier> {
        let nombre = capacite(nom, DC_PAPERS, null_mut());
        if nombre <= 0 {
            return Vec::new();
        }
        let nombre = nombre as usize;
        let mut codes = vec![0u16; nombre];
        let mut noms = vec![0u16; nombre * 64];
        let mut tailles = vec![POINT { x: 0, y: 0 }; nombre];
        capacite(nom, DC_PAPERS, codes.as_mut_ptr());
        capacite(nom, DC_PAPERNAMES, noms.as_mut_ptr());
        capacite(nom, DC_PAPERSIZE, tailles.as_mut_ptr() as *mut u16);
        (0..nombre)
            .map(|rang| {
                let brut = &noms[rang * 64..(rang + 1) * 64];
                let fin = brut.iter().position(|&c| c == 0).unwrap_or(64);
                Papier {
                    code: codes[rang] as i16,
                    nom: String::from_utf16_lossy(&brut[..fin]),
                    // Dixièmes de millimètre.
                    largeur: f64::from(tailles[rang].x) / 10.0,
                    hauteur: f64::from(tailles[rang].y) / 10.0,
                }
            })
            .filter(|papier| papier.largeur > 0.0 && papier.hauteur > 0.0 && !papier.nom.is_empty())
            .collect()
    }

    /// Contexte de dessin de l'imprimante, avec les choix de la fenêtre ; refermé à la fin.
    struct Contexte(HDC);

    impl Contexte {
        fn ouvrir(
            options: &OptionsImpression,
            copies_pilote: bool,
            recto_verso: bool,
        ) -> Result<Self, Erreur> {
            let large = large(&options.imprimante);
            let imprimante = Ouverte::ouvrir(&large)?;
            let (mut reglages, _) = pilote(&imprimante, &large, &options.pilote)?;
            // SAFETY : réglages rendus par le pilote, dans un tampon aligné.
            let dm = unsafe { &mut *reglages.ptr::<DEVMODEW>() };
            appliquer(dm, options, copies_pilote, recto_verso);
            // SAFETY : nom terminé par un zéro, réglages valables pendant l'appel.
            let hdc = unsafe { CreateDCW(null(), large.as_ptr(), null(), dm) };
            if hdc.is_null() {
                return Err(Erreur::simple(format!(
                    "L'imprimante « {} » n'a pas pu être préparée.",
                    options.imprimante
                )));
            }
            Ok(Contexte(hdc))
        }

        fn mesure(&self, quoi: u32) -> i32 {
            // SAFETY : contexte ouvert.
            unsafe { GetDeviceCaps(self.0, quoi as i32) }
        }
    }

    impl Drop for Contexte {
        fn drop(&mut self) {
            // SAFETY : contexte créé par CreateDCW.
            unsafe { DeleteDC(self.0) };
        }
    }

    pub fn feuille(options: &OptionsImpression) -> Result<Feuille, Erreur> {
        let contexte = Contexte::ouvrir(options, true, false)?;
        let mesure = |quoi| f64::from(contexte.mesure(quoi));
        let (ppp_x, ppp_y) = (mesure(LOGPIXELSX).max(1.0), mesure(LOGPIXELSY).max(1.0));
        let (largeur, hauteur) = (mesure(PHYSICALWIDTH), mesure(PHYSICALHEIGHT));
        let (gauche, haut) = (mesure(PHYSICALOFFSETX), mesure(PHYSICALOFFSETY));
        let (zone_l, zone_h) = (mesure(HORZRES), mesure(VERTRES));
        let mm = |valeur: f64, ppp: f64| (valeur / ppp * 25.4).max(0.0);
        Ok(Feuille {
            largeur: mm(largeur, ppp_x),
            hauteur: mm(hauteur, ppp_y),
            marges: [
                mm(gauche, ppp_x),
                mm(haut, ppp_y),
                mm(largeur - gauche - zone_l, ppp_x),
                mm(hauteur - haut - zone_h, ppp_y),
            ],
        })
    }

    pub fn proprietes(
        app: &AppHandle,
        fenetre: &WebviewWindow,
        imprimante: &str,
        pilote: &[u8],
    ) -> Result<Option<Reglages>, Erreur> {
        let hwnd = fenetre
            .hwnd()
            .map_err(|erreur| {
                Erreur::simple(format!(
                    "Les propriétés de l'imprimante n'ont pas pu s'ouvrir : {erreur}"
                ))
            })?
            .0 as usize;
        let (envoi, reception) = std::sync::mpsc::channel();
        let (nom, donnes) = (imprimante.to_string(), pilote.to_vec());
        // La fenêtre du pilote s'ouvre au-dessus de celle de l'application, depuis le fil principal.
        app.run_on_main_thread(move || {
            let _ = envoi.send(fenetre_pilote(hwnd, &nom, &donnes));
        })
        .map_err(|erreur| {
            Erreur::simple(format!(
                "Les propriétés de l'imprimante n'ont pas pu s'ouvrir : {erreur}"
            ))
        })?;
        reception
            .recv()
            .map_err(|_| Erreur::simple("Les propriétés de l'imprimante n'ont pas pu s'ouvrir."))?
    }

    fn fenetre_pilote(hwnd: usize, nom: &str, donnes: &[u8]) -> Result<Option<Reglages>, Erreur> {
        let large = large(nom);
        let imprimante = Ouverte::ouvrir(&large)?;
        let (entree, taille) = pilote(&imprimante, &large, donnes)?;
        let mut sortie = Tampon::new(taille);
        // SAFETY : tampons alignés, de la taille demandée par le pilote ; fenêtre de l'application.
        let retour = unsafe {
            DocumentPropertiesW(
                hwnd as HWND,
                imprimante.0,
                large.as_ptr(),
                sortie.ptr(),
                entree.0.as_ptr() as *const DEVMODEW,
                DM_IN_BUFFER | DM_IN_PROMPT | DM_OUT_BUFFER,
            )
        };
        if retour < 0 {
            return Err(Erreur::simple(
                "Les propriétés de l'imprimante n'ont pas pu s'ouvrir.",
            ));
        }
        if retour == ANNULEE {
            return Ok(None);
        }
        // SAFETY : réglages écrits par le pilote.
        let dm = unsafe { &*(sortie.0.as_ptr() as *const DEVMODEW) };
        Ok(Some(lire_reglages(dm, sortie.octets(taille))))
    }

    pub fn imprimer(
        app: &AppHandle,
        _fenetre: &WebviewWindow,
        octets: Arc<Vec<u8>>,
        nom: &str,
        options: Option<&OptionsImpression>,
    ) -> Result<bool, Erreur> {
        let options = options.ok_or_else(|| Erreur::simple("Choisissez d'abord l'imprimante."))?;
        let nombre = crate::rendu::formats(octets.clone(), nom)?.len();
        let pages: Vec<usize> = if options.pages.is_empty() {
            (0..nombre).collect()
        } else {
            options
                .pages
                .iter()
                .copied()
                .filter(|&page| page < nombre)
                .collect()
        };
        if pages.is_empty() {
            return Err(Erreur::simple("Aucune page à imprimer."));
        }
        let large = large(&options.imprimante);
        let copies = options.copies.max(1);
        // Le pilote fait les copies quand il le sait (assemblées si besoin) ; sinon, Automat-PDF.
        let copies_pilote = copies == 1
            || (capacite(&large, DC_COPIES, null_mut()) >= copies as i32
                && (!options.assembler || capacite(&large, DC_COLLATE, null_mut()) == 1));
        let recto_verso = capacite(&large, DC_DUPLEX, null_mut()) == 1;
        let contexte = Contexte::ouvrir(options, copies_pilote, recto_verso)?;
        let suite = if copies_pilote {
            pages.iter().map(|&page| Some(page)).collect()
        } else {
            sequence(
                &pages,
                copies,
                options.assembler,
                options.recto_verso != RectoVerso::Non,
            )
        };
        let total = suite.len();
        envoyer(&contexte, &octets, nom, &suite, options, |faites| {
            let _ = app.emit(EVENEMENT_AVANCEMENT, Avancement { faites, total });
        })?;
        Ok(true)
    }

    fn envoyer(
        contexte: &Contexte,
        octets: &Arc<Vec<u8>>,
        nom: &str,
        suite: &[Option<usize>],
        options: &OptionsImpression,
        avancement: impl Fn(usize),
    ) -> Result<(), Erreur> {
        let hdc = contexte.0;
        let titre: Vec<u16> = large(nom);
        let document = DOCINFOW {
            cbSize: std::mem::size_of::<DOCINFOW>() as i32,
            lpszDocName: titre.as_ptr(),
            lpszOutput: null(),
            lpszDatatype: null(),
            fwType: 0,
        };
        // SAFETY : contexte d'imprimante ouvert ; titre terminé par un zéro, gardé pendant l'appel.
        if unsafe { StartDocW(hdc, &document) } <= 0 {
            return Err(Erreur::simple("L'imprimante a refusé le document."));
        }
        let zone = (
            contexte.mesure(HORZRES).max(1),
            contexte.mesure(VERTRES).max(1),
        );
        let ppp = (
            contexte.mesure(LOGPIXELSX).max(72) as u32,
            contexte.mesure(LOGPIXELSY).max(72) as u32,
        );
        for (rang, page) in suite.iter().enumerate() {
            let envoi = match page {
                Some(page) => imprimer_page(hdc, octets, nom, *page, zone, ppp, options),
                None => page_blanche(hdc),
            };
            if let Err(erreur) = envoi {
                // SAFETY : document commencé ci-dessus.
                unsafe { AbortDoc(hdc) };
                return Err(erreur);
            }
            avancement(rang + 1);
        }
        // SAFETY : document commencé ci-dessus.
        if unsafe { EndDoc(hdc) } <= 0 {
            return Err(Erreur::simple("L'impression n'a pas pu se terminer."));
        }
        Ok(())
    }

    fn page_blanche(hdc: HDC) -> Result<(), Erreur> {
        // SAFETY : document commencé.
        if unsafe { StartPage(hdc) } <= 0 || unsafe { EndPage(hdc) } <= 0 {
            return Err(Erreur::simple("Une page blanche n'a pas pu être imprimée."));
        }
        Ok(())
    }

    /// Une page : dessinée, en gris au besoin, tournée au besoin, placée, envoyée.
    fn imprimer_page(
        hdc: HDC,
        octets: &Arc<Vec<u8>>,
        nom: &str,
        page: usize,
        zone: (i32, i32),
        ppp: (u32, u32),
        options: &OptionsImpression,
    ) -> Result<(), Erreur> {
        let refusee = || Erreur::simple(format!("La page {} n'a pas pu être imprimée.", page + 1));
        let ppp_dessin = ppp.0.max(ppp.1).min(PPP_MAX);
        let (largeur, hauteur, mut rvb) =
            crate::rendu::pixels_page(octets.clone(), nom, page, ppp_dessin)?;
        if !options.couleur {
            en_gris(&mut rvb);
        }
        let (largeur, hauteur, rvb) = if a_tourner((largeur, hauteur), zone, options.orientation) {
            quart_de_tour(largeur, hauteur, &rvb)
        } else {
            (largeur, hauteur, rvb)
        };
        let dib = en_dib(largeur, hauteur, &rvb);
        let (x, y, l, h) = place_sur_la_feuille(
            (largeur, hauteur),
            ppp_dessin,
            zone,
            ppp,
            options.ajustement,
        );
        let mut info: BITMAPINFO = BITMAPINFO::default();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = largeur as i32;
        // Hauteur négative : lignes de haut en bas.
        info.bmiHeader.biHeight = -(hauteur as i32);
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 24;
        info.bmiHeader.biCompression = BI_RGB;
        // SAFETY : document commencé ; image et en-tête valables pendant l'appel.
        unsafe {
            if StartPage(hdc) <= 0 {
                return Err(refusee());
            }
            SetStretchBltMode(hdc, HALFTONE);
            SetBrushOrgEx(hdc, 0, 0, null_mut());
            let lignes = StretchDIBits(
                hdc,
                x,
                y,
                l,
                h,
                0,
                0,
                largeur as i32,
                hauteur as i32,
                dib.as_ptr() as *const c_void,
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            if lignes == 0 || EndPage(hdc) <= 0 {
                return Err(refusee());
            }
        }
        Ok(())
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod systeme {
    use super::*;

    fn absente() -> Erreur {
        Erreur::simple("L'impression depuis le lecteur n'existe que sous Windows et sur Mac.")
    }

    pub fn imprimer(
        _app: &AppHandle,
        _fenetre: &WebviewWindow,
        _octets: Arc<Vec<u8>>,
        _nom: &str,
        _options: Option<&OptionsImpression>,
    ) -> Result<bool, Erreur> {
        Err(absente())
    }

    pub fn imprimantes() -> Result<Vec<Imprimante>, Erreur> {
        Err(absente())
    }

    pub fn feuille(_options: &OptionsImpression) -> Result<Feuille, Erreur> {
        Err(absente())
    }

    pub fn proprietes(
        _app: &AppHandle,
        _fenetre: &WebviewWindow,
        _imprimante: &str,
        _pilote: &[u8],
    ) -> Result<Option<Reglages>, Erreur> {
        Err(absente())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fait_les_copies_assemblees_ou_page_par_page() {
        assert_eq!(
            sequence(&[0, 1], 2, true, false),
            vec![Some(0), Some(1), Some(0), Some(1)]
        );
        assert_eq!(
            sequence(&[0, 1], 2, false, false),
            vec![Some(0), Some(0), Some(1), Some(1)]
        );
        // Recto verso, nombre impair de pages : chaque copie commence sur une nouvelle feuille.
        assert_eq!(
            sequence(&[0, 1, 2], 2, true, true),
            vec![Some(0), Some(1), Some(2), None, Some(0), Some(1), Some(2)]
        );
        assert_eq!(
            sequence(&[3], 0, true, false),
            vec![Some(3)],
            "Au moins une copie."
        );
    }

    #[test]
    fn tourne_une_image_d_un_quart_de_tour() {
        // 2 × 1 : rouge, vert → 1 × 2 : rouge en haut, vert en bas.
        let (l, h, pixels) = quart_de_tour(2, 1, &[255, 0, 0, 0, 255, 0]);
        assert_eq!((l, h), (1, 2));
        assert_eq!(pixels, vec![255, 0, 0, 0, 255, 0]);
        // 1 × 2 : rouge en haut, vert en bas → 2 × 1 : vert à gauche, rouge à droite.
        let (l, h, pixels) = quart_de_tour(1, 2, &[255, 0, 0, 0, 255, 0]);
        assert_eq!((l, h), (2, 1));
        assert_eq!(pixels, vec![0, 255, 0, 255, 0, 0]);
    }

    #[test]
    fn passe_en_niveaux_de_gris() {
        let mut pixels = vec![255, 0, 0, 255, 255, 255, 0, 0, 0];
        en_gris(&mut pixels);
        assert_eq!(pixels, vec![76, 76, 76, 255, 255, 255, 0, 0, 0]);
    }

    #[test]
    fn ecrit_les_pixels_au_format_de_windows() {
        // Une ligne de 2 pixels (6 octets) complétée à 8 ; bleu, vert, rouge.
        assert_eq!(
            en_dib(2, 1, &[1, 2, 3, 4, 5, 6]),
            vec![3, 2, 1, 6, 5, 4, 0, 0]
        );
    }

    #[test]
    fn ne_tourne_que_si_l_orientation_est_automatique() {
        let paysage = (3508, 2480);
        let feuille_portrait = (4800, 6800);
        assert!(a_tourner(paysage, feuille_portrait, Orientation::Auto));
        assert!(!a_tourner(
            (2480, 3508),
            feuille_portrait,
            Orientation::Auto
        ));
        assert!(!a_tourner(paysage, feuille_portrait, Orientation::Portrait));
        assert!(
            !a_tourner((1000, 1000), feuille_portrait, Orientation::Auto),
            "Page carrée."
        );
    }

    #[test]
    fn place_la_page_selon_l_ajustement() {
        // A4 dessinée à 300 ppp (2480 × 3508) sur une imprimante à 600 ppp, zone 4800 × 6800 : à sa
        // taille réelle (4960 × 7016) elle dépasse.
        let (x, y, l, h) = place_sur_la_feuille(
            (2480, 3508),
            300,
            (4800, 6800),
            (600, 600),
            Ajustement::Reduire,
        );
        assert!(l <= 4800 && h <= 6800 && (l == 4800 || h == 6800));
        assert_eq!((x, y), ((4800 - l) / 2, (6800 - h) / 2));
        assert_eq!(
            place_sur_la_feuille(
                (2480, 3508),
                300,
                (4800, 6800),
                (600, 600),
                Ajustement::Reelle
            ),
            ((4800 - 4960) / 2, (6800 - 7016) / 2, 4960, 7016),
            "Taille réelle : elle dépasse, centrée."
        );
        // Une petite page (A6) : à sa taille si elle tient, agrandie si l'on ajuste.
        assert_eq!(
            place_sur_la_feuille(
                (1240, 1748),
                300,
                (2400, 3400),
                (300, 300),
                Ajustement::Reduire
            ),
            (580, 826, 1240, 1748)
        );
        let (_, _, l, h) = place_sur_la_feuille(
            (1240, 1748),
            300,
            (2400, 3400),
            (300, 300),
            Ajustement::Ajuster,
        );
        assert!(l == 2400 || h == 3400);
        // Imprimante à 600 × 300 ppp : la page n'est pas déformée.
        assert_eq!(
            place_sur_la_feuille(
                (300, 300),
                300,
                (6000, 3000),
                (600, 300),
                Ajustement::Reelle
            ),
            (2700, 1350, 600, 300)
        );
    }
}
