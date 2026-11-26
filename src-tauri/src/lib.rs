mod annotations;
#[cfg(target_os = "macos")]
mod applescript;
mod caviardage;
mod cms;
mod commandes;
mod commentaires;
mod compression;
mod confiance;
mod confidentialite;
mod contenu;
mod docx;
mod envoi;
mod erreur;
#[cfg(test)]
mod essais;
mod fichiers;
mod formulaires;
mod geometrie;
mod images;
mod imposition;
mod impression;
mod lecteur_defaut;
mod libreoffice;
#[cfg(target_os = "macos")]
mod mail;
mod marques;
mod memoire;
mod messagerie;
#[cfg(test)]
mod mesures;
mod navigation;
mod ocr;
mod office;
pub mod operations;
mod outlook;
mod ouverture;
mod pdf;
mod pdf_en_word;
mod pilotage;
mod pixels;
mod police;
mod powershell;
mod rendu;
mod securite;
#[cfg(target_os = "macos")]
mod services;
mod signatures;
mod texte;
mod volume;

use tauri::{AppHandle, Emitter, Manager, Runtime};

use operations::Recus;

/// Fichiers confiés à l'application par le système : clic droit (Explorateur, Finder), « Ouvrir
/// avec », icône du Dock. Tant que l'interface se charge, ils sont gardés : elle les prendra avec
/// ceux du lancement ; ensuite, ils lui sont remis aussitôt.
pub(crate) fn confier<R: Runtime>(app: &AppHandle<R>, recus: Recus) {
    let a_remettre = match app.try_state::<commandes::Lancement>() {
        Some(lancement) => lancement
            .0
            .lock()
            .ok()
            .and_then(|mut reception| reception.recevoir(recus)),
        None => Some(recus),
    };
    if let Some(recus) = a_remettre {
        let _ = app.emit("fichiers-recus", recus);
    }
    au_premier_plan(app);
}

/// La fenêtre de l'application passe au premier plan.
fn au_premier_plan<R: Runtime>(app: &AppHandle<R>) {
    if let Some(fenetre) = app.get_webview_window("main") {
        let _ = fenetre.unminimize();
        let _ = fenetre.set_focus();
    }
}

/// macOS : la barre des menus, en français. Celle que Tauri propose d'office est en anglais ; elle
/// garde ici son ordre et ses raccourcis (⌘C, ⌘V, ⌘Q…), qui servent aussi dans les champs de saisie.
#[cfg(target_os = "macos")]
fn menus<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    use tauri::menu::{
        AboutMetadata, HELP_SUBMENU_ID, Menu, PredefinedMenuItem as Element, Submenu,
        WINDOW_SUBMENU_ID,
    };
    let nom = app.package_info().name.clone();
    let a_propos = AboutMetadata {
        name: Some(nom.clone()),
        version: Some(app.package_info().version.to_string()),
        copyright: app.config().bundle.copyright.clone(),
        authors: app
            .config()
            .bundle
            .publisher
            .clone()
            .map(|editeur| vec![editeur]),
        ..Default::default()
    };
    Menu::with_items(
        app,
        &[
            &Submenu::with_items(
                app,
                &nom,
                true,
                &[
                    &Element::about(app, Some(&format!("À propos d'{nom}")), Some(a_propos))?,
                    &Element::separator(app)?,
                    &Element::services(app, Some("Services"))?,
                    &Element::separator(app)?,
                    &Element::hide(app, Some(&format!("Masquer {nom}")))?,
                    &Element::hide_others(app, Some("Masquer les autres"))?,
                    &Element::show_all(app, Some("Tout afficher"))?,
                    &Element::separator(app)?,
                    &Element::quit(app, Some(&format!("Quitter {nom}")))?,
                ],
            )?,
            &Submenu::with_items(
                app,
                "Fichier",
                true,
                &[&Element::close_window(app, Some("Fermer la fenêtre"))?],
            )?,
            &Submenu::with_items(
                app,
                "Édition",
                true,
                &[
                    &Element::undo(app, Some("Annuler"))?,
                    &Element::redo(app, Some("Rétablir"))?,
                    &Element::separator(app)?,
                    &Element::cut(app, Some("Couper"))?,
                    &Element::copy(app, Some("Copier"))?,
                    &Element::paste(app, Some("Coller"))?,
                    &Element::select_all(app, Some("Tout sélectionner"))?,
                ],
            )?,
            &Submenu::with_items(
                app,
                "Présentation",
                true,
                &[&Element::fullscreen(app, Some("Plein écran"))?],
            )?,
            &Submenu::with_id_and_items(
                app,
                WINDOW_SUBMENU_ID,
                "Fenêtre",
                true,
                &[
                    &Element::minimize(app, Some("Placer dans le Dock"))?,
                    &Element::maximize(app, Some("Agrandir"))?,
                    &Element::separator(app)?,
                    &Element::close_window(app, Some("Fermer"))?,
                ],
            )?,
            &Submenu::with_id_and_items(app, HELP_SUBMENU_ID, "Aide", true, &[])?,
        ],
    )
}

/// macOS : fichiers ouverts depuis le Finder (« Ouvrir avec », glissés sur l'icône du Dock). Ils
/// arrivent par un événement de l'application, jamais par la ligne de commande.
#[cfg(target_os = "macos")]
fn au_fil_des_evenements(app: &AppHandle, evenement: tauri::RunEvent) {
    if let tauri::RunEvent::Opened { urls } = evenement {
        let fichiers: Vec<String> = urls
            .iter()
            .filter_map(|url| url.to_file_path().ok())
            .map(|chemin| chemin.to_string_lossy().into_owned())
            .collect();
        if !fichiers.is_empty() {
            confier(
                app,
                Recus {
                    outil: None,
                    fichiers,
                },
            );
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let au_lancement = operations::recus_de(std::env::args());
    let constructeur = tauri::Builder::default()
        // Une seule fenêtre : un nouveau lancement (clic droit sur d'autres fichiers) confie ses
        // fichiers à la fenêtre déjà ouverte, qui passe au premier plan.
        .plugin(tauri_plugin_single_instance::init(
            |app, arguments, _| match operations::recus_de(arguments) {
                Some(recus) => confier(app, recus),
                None => au_premier_plan(app),
            },
        ))
        .plugin(tauri_plugin_dialog::init())
        // Mises à jour : la version publiée sur GitHub est comparée à celle-ci ; une nouvelle
        // version n'est installée qu'après vérification de sa signature (clé publique dans
        // tauri.conf.json).
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commandes::Lancement(std::sync::Mutex::new(
            operations::Reception::new(au_lancement),
        )))
        .setup(|app| {
            // macOS : les services du clic droit dans le Finder sont remis à cette application.
            #[cfg(target_os = "macos")]
            services::installer(app.handle().clone());
            #[cfg(not(target_os = "macos"))]
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commandes::lire_liste,
            commandes::infos_pdf,
            commandes::fusionner_pdf,
            commandes::separer_pdf,
            commandes::pages_pdf,
            commandes::vignette_pdf,
            commandes::enregistrer_pages,
            commandes::vignette_image,
            commandes::lire_image,
            commandes::images_en_pdf,
            commandes::pdf_en_images,
            commandes::imposer_pdf,
            commandes::marquer_pdf,
            commandes::apercu_marques,
            commandes::apposer_image,
            commandes::annoter_pdf,
            commandes::caviarder_pdf,
            commandes::proteger_pdf,
            commandes::deverrouiller_pdf,
            commandes::deverrouiller_lecture,
            commandes::reverrouiller_lecture,
            commandes::navigation_pdf,
            commandes::commentaires_pdf,
            commandes::formulaire_pdf,
            commandes::signatures_pdf,
            commandes::version_signee,
            commandes::retoucher_lecture,
            commandes::lire_signatures,
            commandes::enregistrer_signatures,
            commandes::imprimer_pdf,
            commandes::imprimantes,
            commandes::feuille_imprimante,
            commandes::proprietes_imprimante,
            commandes::lecteur_par_defaut,
            commandes::devenir_lecteur_par_defaut,
            commandes::ouvrir_lien,
            commandes::compresser_pdf,
            commandes::taille_fichier,
            commandes::textes_pdf,
            commandes::mots_page,
            commandes::separer_par_personne,
            commandes::envoyer_par_outlook,
            commandes::envoyer_par_messagerie,
            commandes::tester_messagerie,
            commandes::mot_de_passe_messagerie_enregistre,
            commandes::enregistrer_mot_de_passe_messagerie,
            commandes::office_en_pdf,
            commandes::pdf_en_word,
            commandes::reconnaitre_texte,
            commandes::semble_numerise,
            commandes::reconnaitre_lecture,
            commandes::enregistrer_lecture,
            commandes::convertisseurs_office,
            commandes::installer_libreoffice,
            commandes::ouvrir_page_libreoffice,
            commandes::ouvrir_fichier,
            commandes::montrer_dans_dossier,
            commandes::recus_au_lancement,
            commandes::lire_enchainements,
            commandes::enregistrer_enchainements,
            commandes::lire_messagerie,
            commandes::enregistrer_messagerie,
            commandes::lire_confidentialite,
            commandes::regler_confidentialite,
            commandes::signaler_ouverture,
            commandes::ouvrir_page_des_versions,
            commandes::relancer,
        ]);
    #[cfg(not(target_os = "macos"))]
    constructeur
        .run(tauri::generate_context!())
        .expect("erreur au lancement d'Automat-PDF");
    #[cfg(target_os = "macos")]
    constructeur
        .menu(menus)
        .build(tauri::generate_context!())
        .expect("erreur au lancement d'Automat-PDF")
        .run(au_fil_des_evenements);
}
