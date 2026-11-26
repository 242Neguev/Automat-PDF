// Accès typé aux commandes du cœur Rust (voir src-tauri/src/commandes.rs).
import { invoke } from "@tauri-apps/api/core";
import type { pourLeCoeur, ValeurChamp } from "./donnees/annotations";

export interface Erreur {
  message: string;
  details: string[];
  /** Ce que l'interface peut en faire : `MOT_DE_PASSE`, le PDF demande son mot de passe d'ouverture. */
  motif?: string;
}

/** Motif d'une erreur du cœur : le PDF ne s'ouvre qu'avec son mot de passe. */
export const MOT_DE_PASSE = "mot_de_passe";

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

/** Enregistre dans `sortie` le PDF mis en page ; avec `repeter` (2 ou 4 par feuille), chaque page est répétée sur sa propre feuille. */
export const imposerPdf = (chemin: string, disposition: Disposition, repeter: boolean, sortie: string) =>
  invoke<void>("imposer_pdf", { chemin, disposition, repeter, sortie });

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

/**
 * Enregistre dans `sortie` le PDF annoté dans le lecteur (surlignages, notes, texte, images), avec
 * ses retouches (commentaires du PDF, champs remplis) ; `aplatir` : son formulaire rejoint ses
 * pages et n'est plus modifiable. Avec `retouches`, le cœur les refait depuis le PDF d'origine, et
 * un PDF signé numériquement s'enregistre à la suite de l'original : ses signatures restent valables.
 */
export const annoterPdf = (chemin: string, annotations: ReturnType<typeof pourLeCoeur>, sortie: string, aplatir = false, retouches?: RetouchesPdf) =>
  invoke<void>("annoter_pdf", { chemin, annotations, sortie, aplatir, retouches });

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

/** Où mène un signet ou un lien interne : la page (0 : la première) et la hauteur visée, en points depuis le haut. */
export interface DestinationPdf {
  page: number;
  y: number | null;
}

/** Un signet du sommaire, à plat : son niveau dit sa place dans l'arbre (0 : premier niveau). */
export interface Signet {
  titre: string;
  niveau: number;
  destination: DestinationPdf | null;
  /** Page web ou e-mail, pour un signet qui y mène. */
  adresse: string | null;
  /** Ses sous-signets sont dépliés à l'ouverture. */
  ouvert: boolean;
}

/** Un lien d'une page, en points de la page affichée depuis son coin en haut à gauche. */
export interface LienPdf {
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
  destination: DestinationPdf | null;
  adresse: string | null;
}

export interface NavigationPdf {
  signets: Signet[];
  /** Les liens de chaque page. */
  liens: LienPdf[][];
}

/** Un format de papier de l'imprimante, en millimètres (code : celui de Windows, 9 pour A4). */
export interface Papier {
  code: number;
  nom: string;
  largeur: number;
  hauteur: number;
}
export type RectoVerso = "non" | "bordLong" | "bordCourt";
/** Taille réelle réduite si besoin, toujours réelle, ou ajustée à la feuille. */
export type Ajustement = "reduire" | "reelle" | "ajuster";
export type Orientation = "auto" | "portrait" | "paysage";
/** Réglages de l'imprimante : par défaut, ou laissés par « Propriétés de l'imprimante… » (pilote). */
export interface ReglagesImprimante {
  couleur: boolean;
  rectoVerso: RectoVerso;
  papier: number | null;
  paysage: boolean;
  /** Réglages complets du pilote, à rendre pour l'impression ; vide : ceux par défaut. */
  pilote: number[];
}
export interface Imprimante {
  nom: string;
  parDefaut: boolean;
  /** Elle sait imprimer en couleur, en recto verso. */
  couleur: boolean;
  rectoVerso: boolean;
  papiers: Papier[];
  reglages: ReglagesImprimante;
}
/** La feuille, en millimètres : taille, et marges non imprimables (gauche, haut, droite, bas). */
export interface Feuille {
  largeur: number;
  hauteur: number;
  marges: [number, number, number, number];
}
/** Les choix de la fenêtre d'impression d'Automat-PDF (Windows). */
export interface OptionsImpression {
  imprimante: string;
  /** Pages (à partir de 0) ; vide : toutes. */
  pages: number[];
  copies: number;
  assembler: boolean;
  couleur: boolean;
  rectoVerso: RectoVerso;
  ajustement: Ajustement;
  orientation: Orientation;
  papier: number | null;
  pilote: number[];
}

/**
 * Imprime le PDF du lecteur tel qu'il s'affiche (déchiffré s'il a été ouvert avec son mot de passe, avec
 * les annotations en cours) : sous Windows avec les choix de la fenêtre d'Automat-PDF, sur Mac (options
 * nulles) par la fenêtre d'impression de macOS. `false` : l'utilisateur a annulé.
 */
export const imprimerPdf = (chemin: string, annotations: ReturnType<typeof pourLeCoeur>, options: OptionsImpression | null) =>
  invoke<boolean>("imprimer_pdf", { chemin, annotations, options });

/** Les imprimantes de l'ordinateur, celle par défaut en tête (Windows). */
export const imprimantes = () => invoke<Imprimante[]>("imprimantes");

/** La feuille telle que l'imprimante la prendra avec ces choix, pour l'aperçu (Windows). */
export const feuilleImprimante = (options: OptionsImpression) => invoke<Feuille>("feuille_imprimante", { options });

/** Ouvre la fenêtre de réglages du pilote de l'imprimante ; `null` : annulée (Windows). */
export const proprietesImprimante = (imprimante: string, pilote: number[]) => invoke<ReglagesImprimante | null>("proprietes_imprimante", { imprimante, pilote });

/** Automat-PDF comme lecteur des PDF du système : le proposer est-il possible (Windows, macOS 12 et plus), est-ce déjà fait ? */
export interface EtatLecteurPdf {
  possible: boolean;
  parDefaut: boolean;
}

export const lecteurParDefaut = () => invoke<EtatLecteurPdf>("lecteur_par_defaut");

/** Un commentaire déjà présent dans le PDF (lu par le cœur) ; place en points de la page affichée. */
export interface CommentairePdf {
  /** Repère stable dans le PDF d'origine (« 12 0 », ou « p2-5 »). */
  id: string;
  page: number;
  /** Sorte PDF : Text (note), Highlight, Underline, StrikeOut, FreeText, Square, Ink, Stamp… */
  sorte: string;
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
  texte: string;
  auteur: string;
  date: string | null;
  couleur: [number, number, number] | null;
  reponse: boolean;
  /** Son texte peut changer (tous, sauf le texte posé sur la page). */
  modifiable: boolean;
}

/** Les commentaires déjà présents dans un PDF reçu, d'après son contenu d'origine. */
export const commentairesPdf = (chemin: string) => invoke<CommentairePdf[]>("commentaires_pdf", { chemin });

/**
 * Retouches du PDF dans le lecteur : ses commentaires supprimés ou changés, ses champs remplis.
 * Toutes à la fois, appliquées en mémoire depuis le contenu d'origine.
 */
export interface RetouchesPdf {
  supprimes: string[];
  textes: [string, string][];
  champs: [string, ValeurChamp][];
}

export const retoucherLecture = (chemin: string, retouches: RetouchesPdf) => invoke<void>("retoucher_lecture", { chemin, retouches });

/** Sorte d'un champ de formulaire. */
export type SorteChamp = "texte" | "case" | "radio" | "liste" | "combo" | "bouton" | "signature";

/** Un widget d'un champ : sa place sur la page affichée, en points depuis son coin en haut à gauche. */
export interface WidgetChamp {
  page: number;
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
  /** État « coché » de ce widget (case, bouton radio) : « Yes », « Oui », « 1 »… */
  etat: string | null;
  /** Taille du texte demandée par le PDF (0 : automatique). */
  taille: number;
  /** 0 : à gauche ; 1 : centré ; 2 : à droite. */
  alignement: number;
}

/** Un champ du formulaire d'un PDF (valeur reçue, avant les retouches du lecteur). */
export interface ChampFormulaire {
  /** Nom complet (« Identité.Nom ») : son repère. */
  nom: string;
  /** Infobulle donnée par le PDF. */
  libelle: string | null;
  sorte: SorteChamp;
  valeur: ValeurChamp;
  lectureSeule: boolean;
  obligatoire: boolean;
  multiligne: boolean;
  /** Mot de passe : le texte s'affiche masqué. */
  secret: boolean;
  longueurMax: number | null;
  /** Une case par caractère. */
  peigne: boolean;
  options: { valeur: string; libelle: string }[];
  multiple: boolean;
  /** Liste déroulante où l'on peut aussi écrire sa propre valeur. */
  saisieLibre: boolean;
  /** Un bouton radio coché ne se décoche pas d'un clic. */
  sansDecocher: boolean;
  /** Champ de signature déjà signé (signature numérique) : pas de signature dessinée dessus. */
  signe: boolean;
  widgets: WidgetChamp[];
}

export interface FormulairePdf {
  champs: ChampFormulaire[];
  /** Le formulaire a aussi une version XFA (retirée du PDF rempli) ; seulement elle. */
  xfa: boolean;
  xfaSeul: boolean;
  /** Des calculs ou des contrôles (JavaScript) qu'Automat-PDF ne fait pas. */
  calculs: boolean;
}

/** Les champs du formulaire d'un PDF, d'après son contenu d'origine. */
export const formulairePdf = (chemin: string) => invoke<FormulairePdf>("formulaire_pdf", { chemin });

/** Ce que le système (Windows, macOS) dit du certificat d'une signature numérique. */
export type ConfianceCertificat = "reconnue" | "nonReconnue" | "horsValidite" | "revoquee" | "inconnue";

/** La signature correspond-elle au document signé ? */
export type IntegriteSignature = "intacte" | "alteree" | "fausse" | "nonPriseEnCharge" | "illisible";

/** Signature ordinaire, certification (l'auteur fixe les modifications permises), horodatage du document. */
export type SorteSignature = "approbation" | "certification" | "horodatage";

/** Ce qui a changé depuis la version signée. */
export interface ModificationsSignature {
  formulaire: boolean;
  /** Autres signatures, horodatages ou données de validation. */
  signatures: boolean;
  commentaires: boolean;
  /** Contenu des pages, pages ajoutées, retirées ou déplacées. */
  pages: boolean;
  /** Actions à l'ouverture, scripts, signets… */
  autres: boolean;
}

export interface CertificatSignature {
  sujet: string | null;
  organisation: string | null;
  courriel: string | null;
  emetteur: string | null;
  /** Valable du … au … (ISO 8601). */
  debut: string | null;
  fin: string | null;
  /** « RSA 2048 bits », « ECDSA P-256 ». */
  cle: string;
  numero: string;
  /** Empreinte SHA-256 du certificat, en hexadécimal. */
  empreinte: string;
}

export interface HorodatageSignature {
  date: string | null;
  autorite: string | null;
  valide: boolean;
  confiance: ConfianceCertificat;
}

/** Une signature numérique d'un PDF, vérifiée par le cœur. */
export interface SignaturePdf {
  champ: string;
  /** Page (à partir de 0) et zone de la signature visible, en points depuis le haut à gauche ; null : invisible. */
  page: number | null;
  zone: [number, number, number, number] | null;
  sorte: SorteSignature;
  /** Modifications permises par une certification : 1 aucune ; 2 remplir et signer ; 3 aussi commenter. */
  permissions: number | null;
  signataire: string | null;
  raison: string | null;
  lieu: string | null;
  contact: string | null;
  /** Date (ISO 8601) : celle de l'horodatage vérifié, sinon celle de l'ordinateur du signataire. */
  date: string | null;
  dateHorodatee: boolean;
  horodatage: HorodatageSignature | null;
  integrite: IntegriteSignature;
  nonPrisEnCharge: string | null;
  empreinte: string | null;
  /** SHA-1 : ancienne, ne protège plus assez. */
  empreinteAncienne: boolean;
  confiance: ConfianceCertificat;
  certificat: CertificatSignature | null;
  /** La signature porte sur tout le fichier ; sinon, ce qui a changé depuis. */
  entiere: boolean;
  modifications: ModificationsSignature;
  /** Taille de la version signée, en octets. */
  fin: number;
}

/** Les signatures numériques d'un PDF, vérifiées (le PDF est relu tel qu'il est sur le disque). */
export const signaturesPdf = (chemin: string) => invoke<SignaturePdf[]>("signatures_pdf", { chemin });

/** Écrit, dans un dossier temporaire, la version d'un PDF qu'a signée une signature ; rend son chemin. */
export const versionSignee = (chemin: string, fin: number) => invoke<string>("version_signee", { chemin, fin });

/** La signature et les initiales dessinées, gardées sur cet ordinateur (JSON ; vide : aucune). */
export const signaturesGardees = () => invoke<string>("lire_signatures");

/** Garde la signature et les initiales sur cet ordinateur (JSON ; vide : les oublie). */
export const garderSignatures = (contenu: string) => invoke<void>("enregistrer_signatures", { contenu });

/**
 * Propose de faire d'Automat-PDF le lecteur des PDF : la fenêtre de choix de Windows (ou, à défaut,
 * ses Paramètres : `parametres`), la confirmation de macOS. Renvoie l'état après coup.
 */
export const devenirLecteurParDefaut = () => invoke<{ etat: EtatLecteurPdf; parametres: boolean }>("devenir_lecteur_par_defaut");

/** Événement du cœur pendant l'envoi des pages à l'imprimante (Windows) : pages envoyées, et sur combien. */
export const AVANCEMENT_IMPRESSION = "impression-avancement";
export interface AvancementImpression {
  faites: number;
  total: number;
}

/** Signets et liens d'un PDF. */
export const navigationPdf = (chemin: string) => invoke<NavigationPdf>("navigation_pdf", { chemin });

/** Ouvre le lien d'un PDF (page web, e-mail), après l'accord de l'utilisateur. Le cœur refuse tout le reste. */
export const ouvrirLien = (adresse: string) => invoke<void>("ouvrir_lien", { adresse });

/** Ouvre dans le lecteur un PDF protégé par un mot de passe : déchiffré en mémoire seulement, jamais sur le disque. */
export const deverrouillerLecture = (chemin: string, motDePasse: string) => invoke<void>("deverrouiller_lecture", { chemin, motDePasse });

/** Oublie le PDF déchiffré : son onglet s'est fermé ; à sa prochaine ouverture, il redemandera son mot de passe. */
export const reverrouillerLecture = (chemin: string) => invoke<void>("reverrouiller_lecture", { chemin });

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

/**
 * Un mot d'une page, pour la couche de texte du lecteur. Position en points sur la page affichée (y vers
 * le bas) : coin en haut à gauche du mot dans son sens d'écriture ; `longueur` dans ce sens, `taille` en
 * travers ; `sens` en quarts de tour dans le sens des aiguilles d'une montre.
 */
export interface Mot {
  texte: string;
  x: number;
  y: number;
  longueur: number;
  taille: number;
  sens: number;
  debutLigne: boolean;
  /** Police du mot, nommée comme un traitement de texte (« Arial »), et sa couleur : pour le corriger. */
  famille: string;
  gras: boolean;
  italique: boolean;
  couleur: [number, number, number];
}

/** Les mots d'une page (à partir de 0), ligne par ligne dans l'ordre de lecture. */
export const motsPage = (chemin: string, page: number) => invoke<Mot[]>("mots_page", { chemin, page });

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

/** Prépare dans Outlook (Brouillons), ou dans Mail sous macOS, ou envoie les messages ; l'avancement arrive par l'événement « progression-envoi ». */
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
/** PDF numérisés rendus cherchables (OCR, moteur du système) : chacun devient « … - texte reconnu.pdf » dans `dossier`. */
export const reconnaitreTexte = (fichiers: string[], dossier: string) => invoke<Conversions>("reconnaitre_texte", { fichiers, dossier });
/** Le PDF semble-t-il numérisé (aucun texte sur ses premières pages) ? */
export const sembleNumerise = (chemin: string) => invoke<boolean>("semble_numerise", { chemin });
/** Ce qu'a donné la reconnaissance du texte d'un PDF. */
export interface BilanReconnaissance {
  /** Pages sans texte, confiées au moteur. */
  pagesLues: number;
  /** Parmi elles, celles où il a reconnu du texte. */
  pagesReconnues: number;
  mots: number;
}
/** Reconnaît, en mémoire seulement, le texte des pages numérisées du PDF (progression-reconnaissance : pages lues, pages à lire). */
export const reconnaitreLecture = (chemin: string) => invoke<BilanReconnaissance>("reconnaitre_lecture", { chemin });
/** Enregistre le PDF du lecteur tel qu'il est en mémoire (texte reconnu, retouches). */
export const enregistrerLecture = (chemin: string, sortie: string) => invoke<void>("enregistrer_lecture", { chemin, sortie });

/** Ce qu'un scanner sait faire : vitre, chargeur de documents, recto verso par le chargeur. */
export interface SourcesScanner {
  vitre: boolean;
  chargeur: boolean;
  rectoVerso: boolean;
}
/** Un scanner de l'ordinateur ; ses sources quand on les connaît (sur Mac, après une première numérisation). */
export interface Scanner {
  id: string;
  nom: string;
  sources: SourcesScanner | null;
}
export type SourceNumerisation = "vitre" | "chargeur";
export type CouleurNumerisation = "couleur" | "gris" | "noir-et-blanc";
export interface ReglagesNumerisation {
  scanner: string;
  source: SourceNumerisation;
  rectoVerso: boolean;
  couleur: CouleurNumerisation;
  /** Points par pouce ; le scanner prend la plus proche des siennes. */
  resolution: number;
}
/** Une page numérisée : son image (dossier de travail) et sa résolution réelle. */
export interface PageNumerisee {
  chemin: string;
  resolution: number;
}
export interface Numerisation {
  pages: PageNumerisee[];
  sources: SourcesScanner | null;
  /** Arrêtée par l'utilisateur ; les pages déjà numérisées sont gardées. */
  annulee: boolean;
  /** Ce qui a arrêté le scanner après quelques pages. */
  interrompue: string | null;
}
export interface BilanNumerisation {
  pages: number;
  texte: BilanReconnaissance | null;
  /** Pourquoi le texte n'a pas été reconnu ; le PDF est enregistré sans. */
  sansTexte: string | null;
}
/** Les scanners de l'ordinateur, branchés ou sur le réseau. */
export const scanners = () => invoke<Scanner[]>("scanners");
/** Numérise (avancement par l'événement « progression-numerisation » : pages numérisées). */
export const numeriser = (reglages: ReglagesNumerisation) => invoke<Numerisation>("numeriser", { reglages });
/** Arrête la numérisation en cours ; les pages déjà numérisées sont gardées. */
export const annulerNumerisation = () => invoke<void>("annuler_numerisation");
/** Le PDF des pages numérisées, à leur taille réelle, avec leur texte reconnu si demandé (progression-reconnaissance). */
export const enregistrerNumerisation = (pages: { chemin: string; rotation: number; resolution: number }[], reconnaitre: boolean, sortie: string) =>
  invoke<BilanNumerisation>("enregistrer_numerisation", { pages, reconnaitre, sortie });
/** Efface les pages numérisées. */
export const oublierNumerisation = () => invoke<void>("oublier_numerisation");

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

/** Ouvre un fichier créé par l'application (PDF, document Word, image) dans l'application habituelle, ou un dossier. */
export const ouvrirFichier = (chemin: string) => invoke<void>("ouvrir_fichier", { chemin });

/** Affiche un fichier dans son dossier (Explorateur, Finder), sélectionné ; un dossier est simplement ouvert. */
export const montrerDansDossier = (chemin: string) => invoke<void>("montrer_dans_dossier", { chemin });

/** Réglages de confidentialité : recherche des mises à jour, statistiques anonymes. */
export interface Confidentialite {
  /** Chercher les nouvelles versions au démarrage, puis chaque jour. */
  misesAJour: boolean;
  /** Statistiques anonymes acceptées ; null tant que la question n'a pas reçu de réponse. */
  statistiques: boolean | null;
  /** Cette copie d'Automat-PDF peut envoyer des statistiques ; sinon, la question n'est pas posée. */
  statistiquesPossibles: boolean;
}

/** Réglages de confidentialité. */
export const lireConfidentialite = () => invoke<Confidentialite>("lire_confidentialite");

/** Change les réglages donnés (les autres restent tels quels) ; renvoie les réglages à jour. */
export const reglerConfidentialite = (reglage: { misesAJour?: boolean; statistiques?: boolean }) =>
  invoke<Confidentialite>("regler_confidentialite", { misesAJour: reglage.misesAJour ?? null, statistiques: reglage.statistiques ?? null });

/** Envoie le signal d'ouverture si l'utilisateur a accepté les statistiques (une fois par lancement) ; true s'il est parti. */
export const signalerOuverture = (langue: string) => invoke<boolean>("signaler_ouverture", { langue });

/** Ouvre la page de la dernière version publiée dans le navigateur. */
export const ouvrirPageDesVersions = () => invoke<void>("ouvrir_page_des_versions");

/** Relance l'application (macOS : la mise à jour installée ne sert qu'une fois l'application rouverte). */
export const relancer = () => invoke<void>("relancer");

/** Fichiers confiés par le système (clic droit dans l'Explorateur ou le Finder, « Ouvrir avec », icône du Dock) : l'outil demandé et les fichiers. */
export interface Recus {
  outil: string | null;
  fichiers: string[];
}

/** Fichiers confiés avant que l'interface soit prête (au lancement, et par les lancements suivants arrivés entre-temps), remis une seule fois ; les autres arrivent par l'événement « fichiers-recus ». */
export const recusAuLancement = () => invoke<Recus[]>("recus_au_lancement");

/** Erreur du cœur (message et détails) ou de l'interface, sous une même forme. */
export function enErreur(erreur: unknown): Erreur {
  if (typeof erreur === "object" && erreur !== null && "message" in erreur) {
    const { message, details, motif } = erreur as Partial<Erreur>;
    return { message: String(message), details: Array.isArray(details) ? details : [], ...(typeof motif === "string" ? { motif } : {}) };
  }
  return { message: String(erreur), details: [] };
}
