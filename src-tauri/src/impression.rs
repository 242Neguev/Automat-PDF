//! Impression d'un PDF depuis le lecteur, par la fenêtre d'impression du système : imprimante,
//! pages, copies, recto verso.
//!
//! - **Mac** : PDFKit imprime le PDF lui-même, net à toute résolution ; la fenêtre d'impression de
//!   macOS montre l'aperçu.
//! - **Windows** : la fenêtre d'impression de Windows (`PrintDlgEx`), puis chaque page choisie,
//!   dessinée par hayro à la résolution de l'imprimante (300 ppp au plus), tournée si elle n'a pas
//!   l'orientation de la feuille, réduite si elle dépasse, et envoyée à l'imprimante par GDI. Copies,
//!   assemblage et recto verso : le pilote s'en charge, d'après la fenêtre.
//!
//! Le PDF imprimé est celui que montre le lecteur : déchiffré s'il a été ouvert avec son mot de
//! passe, avec les annotations en cours (`operations::imprimer_pdf`).

use std::sync::Arc;

use serde::Serialize;
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

/// Imprime le PDF par la fenêtre d'impression du système. `Ok(false)` : l'utilisateur a annulé.
pub fn imprimer(
    app: &AppHandle,
    fenetre: &WebviewWindow,
    octets: Arc<Vec<u8>>,
    nom: &str,
) -> Result<bool, Erreur> {
    systeme::imprimer(app, fenetre, octets, nom)
}

/// Pages (à partir de 0) des plages choisies dans la fenêtre d'impression (à partir de 1), dans
/// l'ordre des plages, sans doublon et dans les limites du document.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn pages_choisies(plages: &[(u32, u32)], nombre: usize) -> Vec<usize> {
    let mut pages = Vec::new();
    for &(debut, fin) in plages {
        let (debut, fin) = (debut.min(fin).max(1) as usize, debut.max(fin) as usize);
        for page in debut..=fin.min(nombre) {
            if !pages.contains(&(page - 1)) {
                pages.push(page - 1);
            }
        }
    }
    pages
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

/// Place d'une page sur la zone imprimable (pixels de l'imprimante) : à sa taille réelle si elle
/// tient, réduite sinon, sans déformation, centrée. Renvoie (x, y, largeur, hauteur).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn place_sur_la_feuille(
    image: (u32, u32),
    ppp_image: u32,
    zone: (i32, i32),
    ppp_imprimante: u32,
) -> (i32, i32, i32, i32) {
    let echelle = f64::from(ppp_imprimante) / f64::from(ppp_image.max(1));
    let (largeur, hauteur) = (f64::from(image.0) * echelle, f64::from(image.1) * echelle);
    let reduction = (f64::from(zone.0) / largeur)
        .min(f64::from(zone.1) / hauteur)
        .min(1.0);
    let (l, h) = (
        (largeur * reduction).round() as i32,
        (hauteur * reduction).round() as i32,
    );
    ((zone.0 - l) / 2, (zone.1 - h) / 2, l, h)
}

#[cfg(target_os = "macos")]
mod systeme {
    use super::*;
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::NSPrintInfo;
    use objc2_foundation::{NSData, NSString};
    use objc2_pdf_kit::{PDFDocument, PDFPrintScalingMode};

    pub fn imprimer(
        app: &AppHandle,
        _fenetre: &WebviewWindow,
        octets: Arc<Vec<u8>>,
        nom: &str,
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
        Ok(operation.runOperation())
    }
}

#[cfg(windows)]
mod systeme {
    use super::*;
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};

    use tauri::Emitter;
    use windows_sys::Win32::Foundation::{GlobalFree, HWND, S_OK};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteDC, GetDeviceCaps, HALFTONE,
        HDC, HORZRES, LOGPIXELSX, SRCCOPY, SetBrushOrgEx, SetStretchBltMode, StretchDIBits,
        VERTRES,
    };
    use windows_sys::Win32::Storage::Xps::{
        AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage,
    };
    use windows_sys::Win32::UI::Controls::Dialogs::{
        PD_NOCURRENTPAGE, PD_NOSELECTION, PD_PAGENUMS, PD_RESULT_PRINT, PD_RETURNDC,
        PD_USEDEVMODECOPIESANDCOLLATE, PRINTDLGEXW, PRINTPAGERANGE, PrintDlgExW,
        START_PAGE_GENERAL,
    };

    /// Ce que l'utilisateur a choisi : l'imprimante (son contexte de dessin) et les plages de pages.
    struct Choix {
        hdc: usize,
        plages: Vec<(u32, u32)>,
    }

    pub fn imprimer(
        app: &AppHandle,
        fenetre: &WebviewWindow,
        octets: Arc<Vec<u8>>,
        nom: &str,
    ) -> Result<bool, Erreur> {
        let nombre = crate::rendu::formats(octets.clone(), nom)?.len();
        let hwnd = fenetre
            .hwnd()
            .map_err(|erreur| {
                Erreur::simple(format!(
                    "La fenêtre d'impression n'a pas pu s'ouvrir : {erreur}"
                ))
            })?
            .0 as usize;
        // La fenêtre d'impression s'ouvre depuis le fil principal (fil COM de la fenêtre de
        // l'application) ; les pages, longues à dessiner, partent d'ici.
        let (envoi, reception) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let _ = envoi.send(fenetre_impression(hwnd, nombre as u32));
        })
        .map_err(|erreur| {
            Erreur::simple(format!("L'impression n'a pas pu commencer : {erreur}"))
        })?;
        let Some(choix) = reception
            .recv()
            .map_err(|_| Erreur::simple("L'impression n'a pas pu commencer."))??
        else {
            return Ok(false);
        };
        let pages = pages_choisies(&choix.plages, nombre);
        let hdc = choix.hdc as HDC;
        let resultat = envoyer(hdc, &octets, nom, &pages, |faites| {
            let _ = app.emit(
                EVENEMENT_AVANCEMENT,
                Avancement {
                    faites,
                    total: pages.len(),
                },
            );
        });
        // SAFETY : contexte rendu par la fenêtre d'impression, plus utilisé ensuite.
        unsafe { DeleteDC(hdc) };
        resultat.map(|()| true)
    }

    fn fenetre_impression(hwnd: usize, nombre: u32) -> Result<Option<Choix>, Erreur> {
        let mut plages = vec![
            PRINTPAGERANGE {
                nFromPage: 1,
                nToPage: nombre,
            };
            32
        ];
        // SAFETY : structure remplie selon la documentation de PrintDlgEx ; les plages restent
        // valables pendant l'appel ; les mémoires rendues sont libérées ici.
        unsafe {
            let mut dialogue: PRINTDLGEXW = std::mem::zeroed();
            dialogue.lStructSize = std::mem::size_of::<PRINTDLGEXW>() as u32;
            dialogue.hwndOwner = hwnd as HWND;
            dialogue.Flags =
                PD_RETURNDC | PD_NOSELECTION | PD_NOCURRENTPAGE | PD_USEDEVMODECOPIESANDCOLLATE;
            dialogue.nMinPage = 1;
            dialogue.nMaxPage = nombre.max(1);
            dialogue.nMaxPageRanges = plages.len() as u32;
            dialogue.lpPageRanges = plages.as_mut_ptr();
            dialogue.nCopies = 1;
            dialogue.nStartPage = START_PAGE_GENERAL;
            let resultat = PrintDlgExW(&mut dialogue);
            if !dialogue.hDevMode.is_null() {
                GlobalFree(dialogue.hDevMode);
            }
            if !dialogue.hDevNames.is_null() {
                GlobalFree(dialogue.hDevNames);
            }
            let fermer = |hdc: HDC| {
                if !hdc.is_null() {
                    DeleteDC(hdc);
                }
            };
            if resultat != S_OK {
                fermer(dialogue.hDC);
                return Err(Erreur::simple(format!(
                    "La fenêtre d'impression n'a pas pu s'ouvrir (erreur {resultat:#x})."
                )));
            }
            if dialogue.dwResultAction != PD_RESULT_PRINT {
                fermer(dialogue.hDC);
                return Ok(None);
            }
            if dialogue.hDC.is_null() {
                return Err(Erreur::simple("Aucune imprimante n'a été choisie."));
            }
            let choisies = if dialogue.Flags & PD_PAGENUMS != 0 {
                plages[..(dialogue.nPageRanges as usize).min(plages.len())]
                    .iter()
                    .map(|plage| (plage.nFromPage, plage.nToPage))
                    .collect()
            } else {
                vec![(1, nombre)]
            };
            Ok(Some(Choix {
                hdc: dialogue.hDC as usize,
                plages: choisies,
            }))
        }
    }

    fn envoyer(
        hdc: HDC,
        octets: &Arc<Vec<u8>>,
        nom: &str,
        pages: &[usize],
        avancement: impl Fn(usize),
    ) -> Result<(), Erreur> {
        let titre: Vec<u16> = nom.encode_utf16().chain([0]).collect();
        // SAFETY : contexte d'imprimante valable ; titre terminé par un zéro, gardé pendant l'appel.
        unsafe {
            let document = DOCINFOW {
                cbSize: std::mem::size_of::<DOCINFOW>() as i32,
                lpszDocName: titre.as_ptr(),
                lpszOutput: null(),
                lpszDatatype: null(),
                fwType: 0,
            };
            if StartDocW(hdc, &document) <= 0 {
                return Err(Erreur::simple("L'imprimante a refusé le document."));
            }
            let zone = (
                GetDeviceCaps(hdc, HORZRES as i32).max(1),
                GetDeviceCaps(hdc, VERTRES as i32).max(1),
            );
            let ppp = GetDeviceCaps(hdc, LOGPIXELSX as i32).max(72) as u32;
            for (rang, &page) in pages.iter().enumerate() {
                if let Err(erreur) = imprimer_page(hdc, octets, nom, page, zone, ppp) {
                    AbortDoc(hdc);
                    return Err(erreur);
                }
                avancement(rang + 1);
            }
            if EndDoc(hdc) <= 0 {
                return Err(Erreur::simple("L'impression n'a pas pu se terminer."));
            }
        }
        Ok(())
    }

    /// Une page : dessinée, tournée au besoin, placée sur la feuille, envoyée.
    unsafe fn imprimer_page(
        hdc: HDC,
        octets: &Arc<Vec<u8>>,
        nom: &str,
        page: usize,
        zone: (i32, i32),
        ppp: u32,
    ) -> Result<(), Erreur> {
        let ppp_dessin = ppp.min(PPP_MAX);
        let (largeur, hauteur, rvb) =
            crate::rendu::pixels_page(octets.clone(), nom, page, ppp_dessin)?;
        let (largeur, hauteur, rvb) =
            if (largeur > hauteur) != (zone.0 > zone.1) && largeur != hauteur {
                quart_de_tour(largeur, hauteur, &rvb)
            } else {
                (largeur, hauteur, rvb)
            };
        let dib = en_dib(largeur, hauteur, &rvb);
        let (x, y, l, h) = place_sur_la_feuille((largeur, hauteur), ppp_dessin, zone, ppp);
        // SAFETY : image et en-tête valables pendant l'appel ; hauteur négative : lignes de haut en bas.
        unsafe {
            if StartPage(hdc) <= 0 {
                return Err(Erreur::simple(format!(
                    "La page {} n'a pas pu être imprimée.",
                    page + 1
                )));
            }
            SetStretchBltMode(hdc, HALFTONE);
            SetBrushOrgEx(hdc, 0, 0, null_mut());
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = largeur as i32;
            info.bmiHeader.biHeight = -(hauteur as i32);
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 24;
            info.bmiHeader.biCompression = BI_RGB;
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
            if lignes == 0 {
                return Err(Erreur::simple(format!(
                    "La page {} n'a pas pu être imprimée.",
                    page + 1
                )));
            }
            if EndPage(hdc) <= 0 {
                return Err(Erreur::simple(format!(
                    "La page {} n'a pas pu être imprimée.",
                    page + 1
                )));
            }
        }
        Ok(())
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod systeme {
    use super::*;

    pub fn imprimer(
        _app: &AppHandle,
        _fenetre: &WebviewWindow,
        _octets: Arc<Vec<u8>>,
        _nom: &str,
    ) -> Result<bool, Erreur> {
        Err(Erreur::simple(
            "L'impression depuis le lecteur n'existe que sous Windows et sur Mac.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suit_les_plages_choisies_sans_doublon_ni_page_absente() {
        assert_eq!(pages_choisies(&[(1, 3)], 10), vec![0, 1, 2]);
        assert_eq!(
            pages_choisies(&[(5, 5), (2, 3), (3, 4)], 10),
            vec![4, 1, 2, 3]
        );
        assert_eq!(
            pages_choisies(&[(8, 12)], 9),
            vec![7, 8],
            "Rien au-delà de la dernière page."
        );
        assert_eq!(
            pages_choisies(&[(4, 2)], 9),
            vec![1, 2, 3],
            "Plage à l'envers."
        );
        assert!(pages_choisies(&[(0, 0)], 9).is_empty(), "Page 0 : aucune.");
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
    fn ecrit_les_pixels_au_format_de_windows() {
        // Une ligne de 2 pixels (6 octets) complétée à 8 ; bleu, vert, rouge.
        assert_eq!(
            en_dib(2, 1, &[1, 2, 3, 4, 5, 6]),
            vec![3, 2, 1, 6, 5, 4, 0, 0]
        );
    }

    #[test]
    fn place_la_page_a_sa_taille_ou_reduite_et_centree() {
        // A4 dessinée à 300 ppp (2480 × 3508) sur une imprimante à 600 ppp, zone de 4800 × 6800 :
        // à sa taille réelle (4960 × 7016) elle dépasse, donc réduite de façon à tenir.
        let (x, y, l, h) = place_sur_la_feuille((2480, 3508), 300, (4800, 6800), 600);
        assert!(l <= 4800 && h <= 6800 && (l == 4800 || h == 6800));
        assert_eq!((x, y), ((4800 - l) / 2, (6800 - h) / 2));
        // Une petite page (A6) tient : taille réelle, centrée.
        assert_eq!(
            place_sur_la_feuille((1240, 1748), 300, (2400, 3400), 300),
            (580, 826, 1240, 1748)
        );
    }
}
