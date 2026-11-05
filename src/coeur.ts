// Accès typé aux commandes du cœur Rust (voir src-tauri/src/commandes.rs).
import { invoke } from "@tauri-apps/api/core";

export interface Erreur {
  message: string;
  details: string[];
}

/** Contenu brut d'une liste (Excel, CSV, ODS), décodée ensuite dans l'interface. */
export const lireListe = (chemin: string) => invoke<ArrayBuffer>("lire_liste", { chemin });

/** Nombre de pages d'un PDF. */
export const infosPdf = (chemin: string) => invoke<number>("infos_pdf", { chemin });

/** Fusionne des PDF, dans l'ordre donné, dans le fichier `sortie`. */
export const fusionnerPdf = (chemins: string[], sortie: string) => invoke<void>("fusionner_pdf", { chemins, sortie });

/** Sépare un PDF en fichiers de `pagesParFichier` pages, nommés d'après `noms` ; renvoie le dossier créé. */
export const separerPdf = (chemin: string, pagesParFichier: number, noms: string[], dossier: string, sousDossier: string) =>
  invoke<string>("separer_pdf", { chemin, pagesParFichier, noms, dossier, sousDossier });

/** Taille d'une page telle qu'elle s'affiche (points, rotation comprise). */
export interface FormatPage {
  largeur: number;
  hauteur: number;
}

/** Une page à reprendre : un PDF, sa page (à partir de 0) et la rotation ajoutée (degrés, sens horaire). */
export interface PageChoisie {
  chemin: string;
  page: number;
  rotation: number;
}

/** Taille de chaque page d'un PDF. */
export const pagesPdf = (chemin: string) => invoke<FormatPage[]>("pages_pdf", { chemin });

/** Image JPEG d'une page, large de `largeur` pixels. */
export const vignettePdf = (chemin: string, page: number, largeur: number) => invoke<ArrayBuffer>("vignette_pdf", { chemin, page, largeur });

/** Enregistre dans `sortie` un PDF fait des pages choisies, dans l'ordre. */
export const enregistrerPages = (pages: PageChoisie[], sortie: string) => invoke<void>("enregistrer_pages", { pages, sortie });

/** Vignette JPEG d'une image (redressée), large de `largeur` pixels au plus. */
export const vignetteImage = (chemin: string, largeur: number) => invoke<ArrayBuffer>("vignette_image", { chemin, largeur });

/** Une image à mettre en PDF : son fichier et la rotation ajoutée (degrés, sens horaire). */
export interface ImageChoisie {
  chemin: string;
  rotation: number;
}

export interface OptionsImages {
  format: "a4" | "ajustee";
  margeMm: number;
}

/** Enregistre dans `sortie` un PDF fait des images, une par page. */
export const imagesEnPdf = (images: ImageChoisie[], options: OptionsImages, sortie: string) => invoke<void>("images_en_pdf", { images, options, sortie });

/** Pages d'un PDF (numéros à partir de 0, toutes si la liste est vide) en images ; renvoie le dossier créé. */
export const pdfEnImages = (chemin: string, pages: number[], format: "png" | "jpeg", dpi: number, dossier: string) =>
  invoke<string>("pdf_en_images", { chemin, pages, format, dpi, dossier });

export type Disposition = "deux-par-feuille" | "quatre-par-feuille" | "livret" | "format-a4";

/** Enregistre dans `sortie` le PDF mis en page. */
export const imposerPdf = (chemin: string, disposition: Disposition, sortie: string) => invoke<void>("imposer_pdf", { chemin, disposition, sortie });

/** Contenu brut d'une image, pour l'afficher telle quelle. */
export const lireImage = (chemin: string) => invoke<ArrayBuffer>("lire_image", { chemin });

export type Position = "haut-gauche" | "haut-centre" | "haut-droite" | "bas-gauche" | "bas-centre" | "bas-droite";

export interface OptionsNumeros {
  format: "n" | "page-n" | "page-n-sur-total" | "n-sur-total";
  position: Position;
  taille: number;
  premier: number;
  margeMm: number;
}

export interface OptionsTampon {
  texte: string;
  style: "filigrane" | "tampon";
  couleur: "gris" | "rouge" | "bleu" | "vert";
  opacite: number;
}

export interface OptionsMarques {
  numeros: OptionsNumeros | null;
  tampon: OptionsTampon | null;
  /** Pages concernées (à partir de 0) ; vide : toutes. */
  pages: number[];
}

/** Enregistre dans `sortie` le PDF avec numéros de page et tampon. */
export const marquerPdf = (chemin: string, options: OptionsMarques, sortie: string) => invoke<void>("marquer_pdf", { chemin, options, sortie });

/** Aperçu (JPEG) d'une page avec son numéro et son tampon. */
export const apercuMarques = (chemin: string, page: number, options: OptionsMarques, largeur: number) =>
  invoke<ArrayBuffer>("apercu_marques", { chemin, page, options, largeur });

/** Emplacement d'une image, en fractions de la page affichée, depuis son coin en haut à gauche. */
export interface Placement {
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
}

/** Enregistre dans `sortie` le PDF avec une image posée sur les pages choisies (vide : toutes). */
export const apposerImage = (chemin: string, image: string, placement: Placement, pages: number[], sortie: string) =>
  invoke<void>("apposer_image", { chemin, image, placement, pages, sortie });

/** Zone à noircir, en fractions de la page affichée, depuis son coin en haut à gauche. */
export interface Zone {
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
}

/** Enregistre dans `sortie` le PDF caviardé ; renvoie les pages (à partir de 1) remplacées par leur image. */
export const caviarderPdf = (chemin: string, pages: { page: number; zones: Zone[] }[], mode: "texte" | "image", sortie: string) =>
  invoke<{ pagesEnImage: number[] }>("caviarder_pdf", { chemin, pages, mode, sortie });

/** Taille d'un fichier, en octets. */
export const tailleFichier = (chemin: string) => invoke<number>("taille_fichier", { chemin });

export interface OptionsProtection {
  motDePasse: string;
  /** Mot de passe du propriétaire ; vide : tiré au hasard. */
  proprietaire: string;
  impression: boolean;
  copie: boolean;
  modification: boolean;
}

/** Enregistre dans `sortie` le PDF protégé par mot de passe (AES-256). */
export const protegerPdf = (chemin: string, options: OptionsProtection, sortie: string) => invoke<void>("proteger_pdf", { chemin, options, sortie });

/** Enregistre dans `sortie` le PDF sans son mot de passe. */
export const deverrouillerPdf = (chemin: string, motDePasse: string, sortie: string) => invoke<void>("deverrouiller_pdf", { chemin, motDePasse, sortie });

/** Ce que la compression a fait : tailles (octets), images allégées, objets en double retirés. */
export interface Bilan {
  avant: number;
  apres: number;
  images: number;
  doublons: number;
}

/** Enregistre dans `sortie` le PDF allégé. */
export const compresserPdf = (chemin: string, niveau: "legere" | "moyenne" | "forte", sortie: string) => invoke<Bilan>("compresser_pdf", { chemin, niveau, sortie });

/** Texte de chaque page (lignes, mots, colonnes séparées par des tabulations ; vide si numérisée). */
export const textesPdf = (chemin: string) => invoke<string[]>("textes_pdf", { chemin });

/** Le fichier d'une personne : ses pages (à partir de 0), et son mot de passe s'il faut le protéger. */
export interface FichierPersonne {
  nom: string;
  pages: number[];
  motDePasse?: string;
}

/** Fichiers créés : le dossier, et le chemin de chaque fichier dans l'ordre demandé. */
export interface Separation {
  dossier: string;
  fichiers: string[];
}

/** Un PDF par personne, dans le dossier `sousDossier` de `dossier`. */
export const separerParPersonne = (chemin: string, personnes: FichierPersonne[], dossier: string, sousDossier: string) =>
  invoke<Separation>("separer_par_personne", { chemin, personnes, dossier, sousDossier });

/** Un message à envoyer : destinataire(s), objet, texte, pièces jointes (chemins complets). */
export interface Message {
  a: string;
  objet: string;
  texte: string;
  pieces: string[];
}

export type ModeEnvoi = "brouillons" | "direct";

/** Ce que l'envoi a fait : messages réussis, et ceux qui ont échoué (rang, raison). */
export interface BilanEnvoi {
  reussis: number;
  echecs: { rang: number; message: string }[];
}

/** Prépare dans Outlook (Brouillons) ou envoie les messages ; l'avancement arrive par l'événement « progression-envoi ». */
export const envoyerParOutlook = (messages: Message[], mode: ModeEnvoi) => invoke<BilanEnvoi>("envoyer_par_outlook", { messages, mode });

/** Compte d'envoi de la messagerie (SMTP) ; le mot de passe, lui, est gardé par le système. */
export interface CompteMessagerie {
  serveur: string;
  port: number;
  chiffrement: "starttls" | "ssl" | "aucun";
  /** Identifiant de connexion, le plus souvent l'adresse ; vide : serveur sans mot de passe. */
  identifiant: string;
  /** Adresse de l'expéditeur. */
  adresse: string;
  /** Nom affiché aux destinataires (facultatif). */
  nom: string;
}

/** Envoie les messages par la messagerie, sans Outlook ; l'avancement arrive par l'événement « progression-envoi ». */
export const envoyerParMessagerie = (compte: CompteMessagerie, messages: Message[]) => invoke<BilanEnvoi>("envoyer_par_messagerie", { compte, messages });

/** Vérifie la connexion et l'identification, sans rien envoyer ; `motDePasse` : celui saisi, sinon celui gardé par le système. */
export const testerMessagerie = (compte: CompteMessagerie, motDePasse: string | null) => invoke<void>("tester_messagerie", { compte, motDePasse });

/** Un mot de passe de messagerie est-il gardé par le système ? */
export const motDePasseMessagerieEnregistre = () => invoke<boolean>("mot_de_passe_messagerie_enregistre");

/** Garde le mot de passe de la messagerie dans le coffre du système ; vide : l'oublie. */
export const enregistrerMotDePasseMessagerie = (motDePasse: string) => invoke<void>("enregistrer_mot_de_passe_messagerie", { motDePasse });

/** Réglages de la messagerie (JSON ; vide la première fois). */
export const lireMessagerie = () => invoke<string>("lire_messagerie");

/** Enregistre les réglages de la messagerie (JSON, sans le mot de passe). */
export const enregistrerMessagerie = (contenu: string) => invoke<void>("enregistrer_messagerie", { contenu });

/** Enchaînements enregistrés (JSON ; vide la première fois). */
export const lireEnchainements = () => invoke<string>("lire_enchainements");

/** Enregistre les enchaînements (JSON). */
export const enregistrerEnchainements = (contenu: string) => invoke<void>("enregistrer_enchainements", { contenu });

/** Fichiers créés par une conversion, et ceux qui n'ont pas pu l'être (rang dans la liste donnée, raison). */
export interface Conversions {
  crees: string[];
  echecs: { rang: number; message: string }[];
}

/** Documents Word, Excel, PowerPoint en PDF dans `dossier` ; avancement par l'événement « progression-conversion ». */
export const officeEnPdf = (fichiers: string[], dossier: string) => invoke<Conversions>("office_en_pdf", { fichiers, dossier });

/** PDF en documents Word modifiables dans `dossier` ; avancement par l'événement « progression-conversion ». */
export const pdfEnWord = (fichiers: string[], dossier: string) => invoke<Conversions>("pdf_en_word", { fichiers, dossier });

/** Ce qui permet de convertir les documents Office sur ce PC. */
export interface Convertisseurs {
  /** Microsoft Word est inscrit (il peut toutefois ne pas répondre). */
  word: boolean;
  /** Word, Excel ou PowerPoint est inscrit. */
  office: boolean;
  libreoffice: boolean;
  /** LibreOffice peut être installé d'ici (winget, Windows). */
  installable: boolean;
}

/** Ce qui permet de convertir les documents Office sur ce PC (Office, LibreOffice, installation possible de LibreOffice). */
export const convertisseursOffice = () => invoke<Convertisseurs>("convertisseurs_office");

/** Installe LibreOffice par son installateur officiel : plusieurs minutes, et l'accord d'un administrateur demandé par Windows. */
export const installerLibreoffice = () => invoke<void>("installer_libreoffice");

/** Ouvre la page de téléchargement officielle de LibreOffice dans le navigateur. */
export const ouvrirPageLibreoffice = () => invoke<void>("ouvrir_page_libreoffice");

/** Fichiers confiés par l'Explorateur : l'outil demandé (clic droit) et les fichiers. */
export interface Recus {
  outil: string | null;
  fichiers: string[];
}

/** Fichiers confiés avant que l'interface soit prête (au lancement, et par les lancements suivants arrivés entre-temps), remis une seule fois ; les autres arrivent par l'événement « fichiers-recus ». */
export const recusAuLancement = () => invoke<Recus[]>("recus_au_lancement");

/** Erreur du cœur (message et détails) ou de l'interface, sous une même forme. */
export function enErreur(erreur: unknown): Erreur {
  if (typeof erreur === "object" && erreur !== null && "message" in erreur) {
    const { message, details } = erreur as Partial<Erreur>;
    return { message: String(message), details: Array.isArray(details) ? details : [] };
  }
  return { message: String(erreur), details: [] };
}
