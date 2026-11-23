//! Clic droit dans le Finder (macOS) : les « Services » d'Automat-PDF, déclarés dans `Info.plist`
//! (`NSServices`), proposent chaque outil sur les fichiers choisis (« Automat-PDF : Compresser »…),
//! comme le sous-menu de l'Explorateur sous Windows. macOS lance l'application si besoin, puis lui
//! confie les fichiers par le presse-papiers du service : ils vont à l'outil demandé (`NSUserData`),
//! dans la fenêtre déjà ouverte.

use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSPasteboard, NSPasteboardTypeFileURL};
use objc2_foundation::{NSString, NSURL};
use tauri::AppHandle;

use crate::operations::Recus;

/// L'application, pour lui remettre les fichiers reçus.
static APPLICATION: OnceLock<AppHandle> = OnceLock::new();

define_class!(
    // SAFETY : NSObject n'impose rien à ses sous-classes ; `Fournisseur` n'implémente pas `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "AutomatPdfFournisseurDeServices"]
    struct Fournisseur;

    impl Fournisseur {
        /// Message de chaque service (`NSMessage` dans Info.plist) ; `outil` : son `NSUserData`.
        // SAFETY : la signature est celle qu'attend AppKit pour un service
        // (`- (void)ouvrirAvecOutil:(NSPasteboard *)pboard userData:(NSString *)userData
        // error:(NSString **)error`).
        #[unsafe(method(ouvrirAvecOutil:userData:error:))]
        fn ouvrir_avec_outil(
            &self,
            presse_papiers: &NSPasteboard,
            outil: Option<&NSString>,
            _erreur: *mut *mut NSString,
        ) {
            let fichiers = fichiers_de(presse_papiers);
            if let (Some(app), false) = (APPLICATION.get(), fichiers.is_empty()) {
                crate::confier(
                    app,
                    Recus {
                        outil: outil.map(|outil| outil.to_string()),
                        fichiers,
                    },
                );
            }
        }
    }
);

impl Fournisseur {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let objet = Self::alloc(mtm).set_ivars(());
        // SAFETY : `init` de NSObject, sans argument.
        unsafe { msg_send![super(objet), init] }
    }
}

/// Chemins des fichiers confiés par le Finder. Il les donne en références de fichier
/// (« file:///.file/id=… ») : elles sont ramenées à leur chemin.
fn fichiers_de(presse_papiers: &NSPasteboard) -> Vec<String> {
    // SAFETY : constante d'AppKit, lue seulement.
    let type_fichier = unsafe { NSPasteboardTypeFileURL };
    presse_papiers
        .pasteboardItems()
        .map(|elements| {
            elements
                .iter()
                .filter_map(|element| element.stringForType(type_fichier))
                .filter_map(|adresse| NSURL::URLWithString(&adresse))
                .filter_map(|url| url.filePathURL())
                .filter_map(|url| url.path())
                .map(|chemin| chemin.to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Inscrit Automat-PDF comme fournisseur de ses services, au lancement : macOS lui remet alors les
/// demandes du clic droit, y compris celle qui l'a lancé.
pub fn installer(app: AppHandle) {
    let _ = APPLICATION.set(app);
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let fournisseur = Fournisseur::new(mtm);
    let objet: &AnyObject = &fournisseur;
    // SAFETY : le fournisseur répond au message déclaré par chaque service.
    unsafe { NSApplication::sharedApplication(mtm).setServicesProvider(Some(objet)) };
    // Le fournisseur sert jusqu'à la fin de l'application.
    std::mem::forget(fournisseur);
}

#[cfg(test)]
mod tests {
    /// Chaque service de l'Info.plist appelle le message du fournisseur, avec la clé d'un des
    /// outils de l'interface.
    #[test]
    fn les_services_d_info_plist_menent_aux_outils() {
        let plist: String = include_str!("../Info.plist")
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let interface = include_str!("../../src/outils.tsx");
        let outils = [
            "organiser",
            "fusionner",
            "compresser",
            "motDePasse",
            "parPersonne",
            "pdfEnWord",
            "enImages",
            "images",
            "officeEnPdf",
        ];
        assert_eq!(
            plist
                .matches("<key>NSMessage</key><string>ouvrirAvecOutil</string>")
                .count(),
            outils.len()
        );
        for outil in outils {
            assert!(
                plist.contains(&format!("<key>NSUserData</key><string>{outil}</string>")),
                "{outil} : absent de l'Info.plist"
            );
            assert!(
                interface.contains(&format!("cle: \"{outil}\"")),
                "{outil} : outil inconnu de l'interface"
            );
        }
        // Sans NSRequiredContext, macOS n'active pas un service de lui-même.
        assert_eq!(
            plist.matches("<key>NSRequiredContext</key><dict/>").count(),
            outils.len()
        );
        assert_eq!(
            plist
                .matches("<key>NSPortName</key><string>Automat-PDF</string>")
                .count(),
            outils.len(),
            "NSPortName : le nom de l'application"
        );
    }
}
