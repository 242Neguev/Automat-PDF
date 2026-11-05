// Empêche l'ouverture d'une console en plus de la fenêtre sous Windows (version finale) : NE PAS RETIRER.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    automat_pdf_lib::run()
}
