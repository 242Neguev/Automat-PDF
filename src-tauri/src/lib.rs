mod caviardage;
mod commandes;
mod compression;
mod confidentialite;
mod contenu;
mod docx;
mod envoi;
mod erreur;
#[cfg(test)]
mod essais;
mod fichiers;
mod geometrie;
mod images;
mod imposition;
mod libreoffice;
mod marques;
mod memoire;
mod messagerie;
mod office;
pub mod operations;
mod outlook;
mod ouverture;
mod pdf;
mod pdf_en_word;
mod pixels;
mod police;
mod powershell;
mod rendu;
mod securite;
mod texte;

use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let au_lancement = operations::recus_de(std::env::args());
    tauri::Builder::default()
        // Une seule fenêtre : un nouveau lancement (clic droit sur d'autres fichiers) confie ses
        // fichiers à la fenêtre déjà ouverte, qui passe au premier plan. Tant que l'interface se
        // charge, ils sont gardés : elle les prendra avec ceux du lancement.
        .plugin(tauri_plugin_single_instance::init(|app, arguments, _| {
            if let Some(recus) = operations::recus_de(arguments) {
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
            }
            if let Some(fenetre) = app.get_webview_window("main") {
                let _ = fenetre.unminimize();
                let _ = fenetre.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        // Mises à jour : la version publiée sur GitHub est comparée à celle-ci ; une nouvelle
        // version n'est installée qu'après vérification de sa signature (clé publique dans
        // tauri.conf.json).
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commandes::Lancement(std::sync::Mutex::new(
            operations::Reception::new(au_lancement),
        )))
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
            commandes::caviarder_pdf,
            commandes::proteger_pdf,
            commandes::deverrouiller_pdf,
            commandes::compresser_pdf,
            commandes::taille_fichier,
            commandes::textes_pdf,
            commandes::separer_par_personne,
            commandes::envoyer_par_outlook,
            commandes::envoyer_par_messagerie,
            commandes::tester_messagerie,
            commandes::mot_de_passe_messagerie_enregistre,
            commandes::enregistrer_mot_de_passe_messagerie,
            commandes::office_en_pdf,
            commandes::pdf_en_word,
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
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement d'Automat-PDF");
}
