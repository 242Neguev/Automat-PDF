// Lecteur de PDF (plan du 25/11/2026) : les pages à la suite, que l'on fait défiler et agrandir, une
// colonne de vignettes, la recherche dans le texte, et à côté les outils qui s'appliquent au document.
// Les pages sont les images que dessine le cœur (hayro), comme les vignettes : demandées quand elles
// approchent, à la largeur affichée. Par-dessus, une couche de texte invisible, placée mot par mot,
// permet de sélectionner et de copier le texte, et porte les surlignages de la recherche.
import { Fragment, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ActionIcon, Alert, Badge, Box, Button, Center, CloseButton, Group, Loader, Modal, NumberInput, SegmentedControl, Stack, Text, TextInput, Title, Tooltip, UnstyledButton } from "@mantine/core";
import {
  IconBook2,
  IconChevronDown,
  IconChevronUp,
  IconCircleCheck,
  IconExternalLink,
  IconFileText,
  IconFolder,
  IconLayoutSidebar,
  IconMaximize,
  IconPencil,
  IconPlus,
  IconPrinter,
  IconRotateClockwise,
  IconScreenshot,
  IconSearch,
  IconZoomIn,
  IconZoomOut,
} from "@tabler/icons-react";
import { listen } from "@tauri-apps/api/event";
import { notifications } from "@mantine/notifications";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  annoterPdf,
  AVANCEMENT_IMPRESSION,
  copierTexte,
  copierZone,
  enErreur,
  enregistrerZoneImage,
  enregistrerZonePdf,
  imprimerPdf,
  lireImage,
  montrerDansDossier,
  MOT_DE_PASSE,
  navigationPdf,
  ouvrirFichier,
  ouvrirLien,
  commentairesPdf,
  formulairePdf,
  retoucherLecture,
  enregistrerLecture,
  sembleNumerise,
  signaturesPdf,
  type ChampFormulaire,
  type CommentairePdf,
  type FormulairePdf,
  pagesPdf,
  tailleFichier,
  type DestinationPdf,
  type FormatPage,
  type LienPdf,
  type Mot,
  type AvancementImpression,
  type NavigationPdf,
  type OptionsImpression,
  type BilanReconnaissance,
  type SignaturePdf,
} from "../../coeur";
import { echelleAjustee, hautAffiche, premiereDeRangee, rangees, tourner, transformation, type Disposition, type Rotation } from "../../donnees/affichage";
import {
  annuler,
  appliquer,
  HISTORIQUE_VIDE,
  nomAnnote,
  nomTexteReconnu,
  nombreDeModifications,
  nomRempli,
  nomSigne,
  nouvelId,
  pourLeCoeur,
  retablir,
  retouchesDe,
  sans,
  sorteDEnregistrement,
  valeursChamps,
  zoneDe,
  type Annotation,
  type Historique,
  type Teinte,
  type ValeurChamp,
  type Zone,
} from "../../donnees/annotations";
import { enFractions, texteDansZone, type ZonePoints } from "../../donnees/capture";
import { aCote, nomDe } from "../../donnees/chemins";
import { initialesDe, remplissable, signable } from "../../donnees/formulaire";
import { signaturesDeLaPage } from "../../donnees/signatures";
import { zoneDansCadre, type SignatureChoisie } from "../../donnees/signature";
import { motsCouverts, occurrences, type Occurrence } from "../../donnees/recherche-texte";
import { formaterTaille } from "../../donnees/tailles";
import { GROUPE_DE, type DefinitionOutil } from "../../outils";
import { textes } from "../../textes";
import { varsTeinte } from "../../theme";
import { confierPage } from "../commun/reception";
import Vignette from "../commun/Vignette";
import { imagePage, relire } from "../commun/vignettes";
import { signalerErreur } from "../commun/signalerErreur";
import { ZoneDepot } from "../commun/ZoneFichier";
import { BarreAnnotations, CoucheAnnotations, type ImageChargee, type OutilAnnotation, type ReglagesCommentaires } from "./Annotations";
import { BarreCapture, CoucheCapture, type QualiteCapture, type TravailCapture } from "./Capture";
import ListeCommentaires from "./Commentaires";
import { BandeauFormulaire, CoucheFormulaire } from "./Formulaire";
import FenetreSignature from "./FenetreSignature";
import { BandeauSignatures, CoucheSignatures, PanneauSignatures } from "./Signatures";
import { BandeauNumerise } from "./Numerise";
import MotDePasseLecture from "./MotDePasseLecture";
import ColonneSignets from "./Signets";
import FenetreImpression from "./Impression";
import { PropositionLecteurParDefaut } from "./LecteurParDefaut";
import PleinEcran from "./PleinEcran";
import { MAC } from "../../systeme";
import { motsDe, oublierTextes, textesDe } from "./textes";
import { useReconnaissance } from "../commun/reconnaissance";

const t = textes.lecteur;

/** Pixels CSS par point du PDF à 100 % (un pouce : 96 pixels, 72 points). */
const TAILLE_REELLE = 96 / 72;
const ECHELLE_MIN = 0.25 * TAILLE_REELLE;
const ECHELLE_MAX = 4 * TAILLE_REELLE;
/** Marge autour des pages, dans la zone qui défile. */
const MARGE = 16;
/** Préférences gardées : la colonne des vignettes affichée ou non ; une page ou deux côte à côte. */
const CLE_VIGNETTES = "automat-pdf.lecteur-vignettes";
const CLE_DISPOSITION = "automat-pdf.lecteur-disposition";
/** Images qu'un navigateur affiche telles quelles (pour l'aperçu d'une image posée). */
const TYPES_IMAGES: Record<string, string> = { png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", webp: "image/webp", gif: "image/gif", bmp: "image/bmp" };

/** Même fichier, aux barres et aux majuscules près. */
const memeFichier = (a: string, b: string) => a.replace(/\\/g, "/").toLowerCase() === b.replace(/\\/g, "/").toLowerCase();

/** Lit une image choisie : son adresse dans la page, et sa largeur sur sa hauteur. */
async function chargerImage(chemin: string): Promise<ImageChargee> {
  const octets = await lireImage(chemin);
  const extension = chemin.split(".").pop()?.toLowerCase() ?? "";
  const adresse = URL.createObjectURL(new Blob([octets], { type: TYPES_IMAGES[extension] ?? "application/octet-stream" }));
  const proportion = await new Promise<number>((resoudre, rejeter) => {
    const image = new Image();
    image.onload = () => resoudre(image.naturalWidth / Math.max(1, image.naturalHeight));
    image.onerror = () => rejeter(new Error(textes.lecteur.illisible));
    image.src = adresse;
  });
  return { adresse, proportion };
}

type Mode = "largeur" | "page" | "libre";

function lirePreference(): boolean {
  try {
    return localStorage.getItem(CLE_VIGNETTES) !== "non";
  } catch {
    return true;
  }
}

function lireDisposition(): Disposition {
  try {
    return localStorage.getItem(CLE_DISPOSITION) === "deux" ? "deux" : "une";
  } catch {
    return "une";
  }
}

/** Où l'on en était dans un document : retrouvé en revenant à son onglet. */
interface EtatDocument {
  page: number;
  fraction: number;
  mode: Mode;
  echelle: number;
  rotation: Rotation;
}

interface Props {
  /** Le PDF ouvert, ou null. */
  chemin: string | null;
  /** Le lecteur est-il l'écran affiché ? */
  actif: boolean;
  /** Les outils qui prennent un PDF, proposés à côté du document. */
  outils: DefinitionOutil[];
  onOutil: (definition: DefinitionOutil) => void;
  /** Un autre PDF choisi dans la fenêtre d'ouverture. */
  onChoisir: (chemin: string) => void;
  /** Les PDF ouverts, en onglets (celui qui s'affiche compris) ; passer à l'un, en fermer un. */
  ouverts: string[];
  onOnglet: (chemin: string) => void;
  onFermer: (chemin: string) => void;
}

export default function Lecteur({ chemin, actif, outils, onOutil, onChoisir, ouverts, onOnglet, onFermer }: Props) {
  const [formats, setFormats] = useState<FormatPage[] | null>(null);
  const [erreur, setErreur] = useState<string | null>(null);
  /** Le PDF demande son mot de passe d'ouverture ; puis il a été ouvert avec (déchiffré en mémoire). */
  const [protege, setProtege] = useState(false);
  const [deverrouille, setDeverrouille] = useState(false);
  /** Relance la lecture du document (après son mot de passe). */
  const [ouverture, setOuverture] = useState(0);
  /** Signets et liens du document ; le panneau de gauche (vignettes ou signets) ; le lien vers un site à confirmer. */
  const [navigation, setNavigation] = useState<NavigationPdf | null>(null);
  /**
   * Commentaires déjà présents dans le PDF (null : pas encore lus) ; celui choisi dans la liste,
   * et le cadre qui le montre un instant sur sa page ; version des images des pages, relues après
   * chaque retouche de ces commentaires.
   */
  const [existants, setExistants] = useState<CommentairePdf[] | null>(null);
  const [choisiPdf, setChoisiPdf] = useState<string | null>(null);
  const [repere, setRepere] = useState<{ page: number; zone: Zone } | null>(null);
  const [versionImages, setVersionImages] = useState(0);
  /** Les retouches envoyées au cœur pour chaque document (pour ne pas les refaire pour rien). */
  const retouchesEnvoyees = useRef<Record<string, string>>({});
  const [panneau, setPanneau] = useState<"pages" | "signets">("pages");
  const [lienExterne, setLienExterne] = useState<string | null>(null);
  /** Impression en cours : pages envoyées à l'imprimante, et sur combien (0 : la fenêtre d'impression est ouverte). */
  const [impression, setImpression] = useState<AvancementImpression | null>(null);
  /** Fenêtre d'impression d'Automat-PDF ouverte (Windows ; sur Mac, celle de macOS). */
  const [fenetreImpression, setFenetreImpression] = useState(false);
  /** Taille du fichier, écrite pour l'en-tête (« 3,9 Mo »). */
  const [taille, setTaille] = useState<string | null>(null);
  const [mode, setMode] = useState<Mode>("largeur");
  const [echelle, setEchelle] = useState(TAILLE_REELLE);
  const [courante, setCourante] = useState(0);
  /** Numéro tapé dans « Page », appliqué à Entrée ou en quittant la case ; vide : la page lue. */
  const [saisie, setSaisie] = useState<string | number>("");
  const [hauteur, setHauteur] = useState(600);
  const [vignettes, setVignettes] = useState(lirePreference);
  /** Affichage : une page ou deux côte à côte, la rotation (le fichier n'est pas modifié), le plein écran. */
  const [disposition, setDisposition] = useState<Disposition>(lireDisposition);
  const [rotation, setRotation] = useState<Rotation>(0);
  const [pleinEcran, setPleinEcran] = useState(false);
  /** Recherche : le texte tapé, ses occurrences dans le document (null : pas de recherche), la courante. */
  const [recherche, setRecherche] = useState("");
  const [resultats, setResultats] = useState<Occurrence[] | null>(null);
  const [actuel, setActuel] = useState(0);
  /** Éditeur : le mode annotation, l'outil et ses réglages, et les annotations de chaque document ouvert. */
  const [annoter, setAnnoter] = useState(false);
  const [outil, setOutil] = useState<OutilAnnotation>("surligner");
  const [teinteSurligneur, setTeinteSurligneur] = useState<Teinte>("jaune");
  const [teinteTexte, setTeinteTexte] = useState<Teinte>("noir");
  const [tailleTexte, setTailleTexte] = useState(12);
  const [reglages, setReglages] = useState<ReglagesCommentaires>({ teinteTrait: "rouge", forme: "rectangle", epaisseur: 2, tampon: "approuve", avecDate: true });
  const [historiques, setHistoriques] = useState<Record<string, Historique>>({});
  const [selection, setSelection] = useState<string | null>(null);
  const [imagesChargees, setImagesChargees] = useState<Record<string, ImageChargee>>({});
  const [enregistrement, setEnregistrement] = useState(false);
  /**
   * Capturer une zone (0.9.0) : le mode, le cadre tracé et sa page, le nombre de ses mots (null : pas
   * encore lus), la qualité de l'image, le travail en cours, et la dernière capture enregistrée.
   */
  const [capturer, setCapturer] = useState(false);
  const [capture, setCapture] = useState<{ page: number; zone: ZonePoints } | null>(null);
  const [texteCapture, setTexteCapture] = useState<{ texte: string; mots: number } | null>(null);
  const [qualiteCapture, setQualiteCapture] = useState<QualiteCapture>("300");
  const [travailCapture, setTravailCapture] = useState<TravailCapture>(null);
  const [captureEnregistree, setCaptureEnregistree] = useState<{ chemin: string; enImage: boolean } | null>(null);
  /** Le PDF annoté (ou le formulaire rempli) qui vient d'être enregistré, et qui s'affiche. */
  const [enregistre, setEnregistre] = useState<string | null>(null);
  const [sorteEnregistree, setSorteEnregistree] = useState<"annote" | "rempli" | "signe">("annote");
  /** Formulaire du PDF (0.7.0) : ses champs tels que reçus (null : pas encore lus) ; l'option « aplatir ». */
  const [formulaire, setFormulaire] = useState<FormulairePdf | null>(null);
  const [aplatir, setAplatir] = useState(false);
  /**
   * Signer (0.7.0) : la fenêtre de signature ouverte (et le champ de signature où poser, s'il y en
   * a un) ; la signature que l'outil Signature pose d'un clic.
   */
  const [fenetreSignature, setFenetreSignature] = useState<{ cible: { page: number; zone: Zone } | null } | null>(null);
  const [signatureChoisie, setSignatureChoisie] = useState<SignatureChoisie | null>(null);
  /**
   * Signatures numériques (0.7.0) : celles du PDF, vérifiées par le cœur (null : pas encore lues) ;
   * le panneau de leurs détails, ouvert (et la signature à y montrer d'abord) ; après un
   * enregistrement, si elles sont restées valables.
   */
  const [signatures, setSignatures] = useState<SignaturePdf[] | null>(null);
  /** Le PDF semble numérisé : aucun texte sur ses premières pages (0.8.0, OCR). */
  const [numerise, setNumerise] = useState(false);
  /** Son texte, reconnu en mémoire : ce qu'a donné la reconnaissance. */
  const reconnaissance = useReconnaissance();
  const [reconnu, setReconnu] = useState<BilanReconnaissance | null>(null);
  const [enregistrementTexte, setEnregistrementTexte] = useState(false);
  useEffect(() => setReconnu(null), [chemin]);
  const [panneauSignatures, setPanneauSignatures] = useState<{ enAvant: string | null } | null>(null);
  // Après l'enregistrement d'un PDF signé : ses signatures restent valables, ou signalent les corrections.
  const [signaturesGardees, setSignaturesGardees] = useState<"gardees" | "pagesChangees" | null>(null);
  const cadre = useRef<HTMLDivElement>(null);
  const defilement = useRef<HTMLDivElement>(null);
  const champRecherche = useRef<HTMLInputElement>(null);
  const pages = useRef<(HTMLDivElement | null)[]>([]);
  // Page et position dans la page à retrouver après un changement d'échelle ou un retour au lecteur.
  const ancre = useRef<{ page: number; fraction: number } | null>(null);
  const courante_ = useRef(0);
  const echelle_ = useRef(echelle);
  echelle_.current = echelle;
  const mode_ = useRef(mode);
  mode_.current = mode;
  const rotation_ = useRef(rotation);
  rotation_.current = rotation;
  /** La page lue et la position dans la page, tenues à jour au défilement ; et l'état de chaque document. */
  const positionLue = useRef({ page: 0, fraction: 0 });
  const etats = useRef<Record<string, EtatDocument>>({});
  const documentAffiche = useRef<string | null>(null);

  async function choisir() {
    const choisi = await open({ multiple: false, directory: false, filters: [{ name: t.filtre, extensions: ["pdf"] }] });
    if (typeof choisi === "string") onChoisir(choisi);
  }

  // Le document : la taille de chaque page, pour tout placer avant le moindre rendu.
  useEffect(() => {
    // Un autre document (un autre onglet) : on note où l'on en était dans le précédent, et l'on
    // retrouve où l'on en était dans celui-ci. Relu avec son mot de passe : il garde son état.
    const precedent = documentAffiche.current;
    if (precedent !== chemin) {
      if (precedent) etats.current[precedent] = { ...positionLue.current, mode: mode_.current, echelle: echelle_.current, rotation: rotation_.current };
      documentAffiche.current = chemin;
      // Ses retouches sont renvoyées au cœur : son onglet a pu se fermer entre-temps (le cœur les a oubliées).
      if (chemin) delete retouchesEnvoyees.current[chemin];
      const etat = chemin ? etats.current[chemin] : undefined;
      setMode(etat?.mode ?? "largeur");
      if (etat?.mode === "libre") setEchelle(etat.echelle);
      setRotation(etat?.rotation ?? 0);
      positionLue.current = etat ? { page: etat.page, fraction: etat.fraction } : { page: 0, fraction: 0 };
      ancre.current = etat ? { page: etat.page, fraction: etat.fraction } : null;
      setCourante(etat?.page ?? 0);
      courante_.current = etat?.page ?? 0;
    }
    setFormats(null);
    setErreur(null);
    setProtege(false);
    setNavigation(null);
    setTaille(null);
    setRecherche("");
    setResultats(null);
    setSelection(null);
    setEnregistre((avant) => (avant && chemin && memeFichier(avant, chemin) ? avant : null));
    if (!chemin) return;
    let actuelDoc = true;
    pagesPdf(chemin).then(
      (lus) => actuelDoc && setFormats(lus),
      (e) => {
        if (!actuelDoc) return;
        const lue = enErreur(e);
        if (lue.motif === MOT_DE_PASSE) setProtege(true);
        else setErreur(lue.message || t.illisible);
      },
    );
    tailleFichier(chemin).then(
      (octets) => actuelDoc && setTaille(formaterTaille(octets)),
      () => undefined,
    );
    // Signets et liens : un PDF sans eux (ou illisible pour lopdf) s'affiche quand même.
    navigationPdf(chemin).then(
      (lue) => actuelDoc && setNavigation(lue),
      () => undefined,
    );
    // Les commentaires déjà présents dans le PDF reçu (après son mot de passe, s'il en a un).
    setExistants(null);
    setChoisiPdf(null);
    setRepere(null);
    commentairesPdf(chemin).then(
      (lus) => actuelDoc && setExistants(lus),
      () => actuelDoc && setExistants([]),
    );
    // Les champs de son formulaire, s'il en a un.
    setFormulaire(null);
    formulairePdf(chemin).then(
      (lu) => actuelDoc && setFormulaire(lu),
      () => actuelDoc && setFormulaire(null),
    );
    // Ses signatures numériques, vérifiées (le PDF est relu tel qu'il a été signé).
    setSignatures(null);
    setPanneauSignatures(null);
    signaturesPdf(chemin).then(
      (lues) => actuelDoc && setSignatures(lues),
      () => actuelDoc && setSignatures([]),
    );
    // Un scan sans texte : le lecteur propose de reconnaître son texte.
    setNumerise(false);
    sembleNumerise(chemin).then(
      (oui) => actuelDoc && setNumerise(oui),
      () => {},
    );
    return () => {
      actuelDoc = false;
    };
  }, [chemin, ouverture]);

  // Un autre document : l'avis « ouvert avec son mot de passe » ne le concerne pas.
  useEffect(() => setDeverrouille(false), [chemin]);

  // Hauteur disponible sous la barre de l'application : le lecteur tient dans la fenêtre, seules ses pages défilent.
  useLayoutEffect(() => {
    if (!actif) return;
    const mesurer = () => {
      const haut = cadre.current?.getBoundingClientRect().top ?? 0;
      setHauteur(Math.max(320, window.innerHeight - haut - MARGE));
    };
    mesurer();
    window.addEventListener("resize", mesurer);
    return () => window.removeEventListener("resize", mesurer);
  }, [actif, formats]);

  /** Échelle qui fait tenir la plus large rangée de pages (ou la plus haute page) dans la zone qui défile. */
  const ajuster = useCallback(
    (voulu: Mode) => {
      const zone = defilement.current;
      if (!zone || !formats || formats.length === 0 || voulu === "libre") return;
      const voulue = echelleAjustee(formats, rotation, disposition, { largeur: zone.clientWidth, hauteur: zone.clientHeight }, voulu, MARGE);
      changerEchelle(Math.min(ECHELLE_MAX, Math.max(ECHELLE_MIN, voulue)));
    },
    // changerEchelle ne lit que des références : seules les pages et leur disposition comptent.
    [formats, rotation, disposition],
  );

  // Largeur ou page entière : l'échelle suit la taille de la fenêtre (et la colonne des vignettes).
  useEffect(() => {
    const zone = defilement.current;
    if (!zone || mode === "libre" || !actif) return;
    ajuster(mode);
    const observateur = new ResizeObserver(() => ajuster(mode));
    observateur.observe(zone);
    return () => observateur.disconnect();
  }, [mode, ajuster, actif, hauteur]);

  /** Change l'échelle en gardant à l'écran l'endroit que l'on lisait. */
  function changerEchelle(nouvelle: number) {
    const zone = defilement.current;
    const page = pages.current[courante_.current];
    if (zone && page) {
      const haut = page.getBoundingClientRect().top - zone.getBoundingClientRect().top;
      ancre.current = { page: courante_.current, fraction: Math.max(0, -haut) / Math.max(1, page.offsetHeight) };
    }
    setEchelle(nouvelle);
  }

  // Après un changement d'échelle, de rotation ou de disposition, au retour dans le lecteur, ou quand
  // les pages d'un onglet retrouvé arrivent : retour à l'endroit lu.
  useLayoutEffect(() => {
    const zone = defilement.current;
    const voulue = ancre.current;
    if (!zone || !voulue || !actif) return;
    const page = pages.current[voulue.page];
    if (!page) return;
    ancre.current = null;
    const haut = page.getBoundingClientRect().top - zone.getBoundingClientRect().top;
    zone.scrollTop += haut + voulue.fraction * page.offsetHeight;
  }, [echelle, actif, formats, rotation, disposition]);

  // En quittant le lecteur, on note la page lue : un écran caché perd sa position de défilement.
  useEffect(() => {
    if (!actif) ancre.current = { page: courante_.current, fraction: 0 };
  }, [actif]);

  /** La page lue : celle qui occupe le tiers haut de la zone (recherche par dichotomie), la première de sa rangée. */
  const suivre = useCallback(() => {
    const zone = defilement.current;
    if (!zone || !formats) return;
    const dessus = zone.getBoundingClientRect().top;
    const repere = dessus + zone.clientHeight / 3;
    let [bas, haut] = [0, formats.length - 1];
    while (bas < haut) {
      const milieu = Math.ceil((bas + haut) / 2);
      const page = pages.current[milieu];
      if (page && page.getBoundingClientRect().top <= repere) bas = milieu;
      else haut = milieu - 1;
    }
    const lue = premiereDeRangee(bas, disposition);
    const element = pages.current[lue];
    if (element) positionLue.current = { page: lue, fraction: Math.max(0, dessus - element.getBoundingClientRect().top) / Math.max(1, element.offsetHeight) };
    if (lue !== courante_.current) {
      courante_.current = lue;
      setCourante(lue);
    }
  }, [formats, disposition]);

  /** Fait défiler jusqu'à la page `index`, `decalage` pixels sous son haut. */
  function allerA(index: number, decalage = 0) {
    const zone = defilement.current;
    const page = pages.current[index];
    if (!zone || !page) return;
    zone.scrollTop += page.getBoundingClientRect().top - zone.getBoundingClientRect().top - MARGE + decalage;
  }

  // Avancement de l'envoi des pages à l'imprimante (Windows ; sur Mac, macOS montre le sien).
  useEffect(() => {
    const arret = listen<AvancementImpression>(AVANCEMENT_IMPRESSION, (evenement) => setImpression(evenement.payload)).catch(() => () => {});
    return () => void arret.then((arreter) => arreter());
  }, []);

  /**
   * Imprime le document tel qu'il s'affiche, annotations en cours comprises : sur Mac par la fenêtre
   * d'impression de macOS, sous Windows par celle d'Automat-PDF (aperçu, couleur ou noir et blanc…).
   */
  function imprimer() {
    if (!chemin || !formats || impression) return;
    if (MAC) void lancerImpression(null);
    else setFenetreImpression(true);
  }

  async function lancerImpression(options: OptionsImpression | null) {
    if (!chemin) return;
    setFenetreImpression(false);
    setImpression({ faites: 0, total: 0 });
    try {
      if (await imprimerPdf(chemin, pourLeCoeur(annotations), options)) notifications.show({ color: "eau", message: t.imprime, autoClose: 4000 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setImpression(null);
    }
  }

  /** Suit un signet ou un lien : vers une page (et sa hauteur), ou vers un site, après confirmation. */
  function suivreLien(destination: DestinationPdf | null, adresse: string | null) {
    const format = destination ? formats?.[destination.page] : undefined;
    if (destination) allerA(destination.page, destination.y && format ? hautAffiche(format, 0, destination.y, rotation) * echelle : 0);
    else if (adresse) setLienExterne(adresse);
  }

  function validerSaisie() {
    const numero = Number(saisie);
    if (saisie !== "" && Number.isInteger(numero) && numero >= 1 && numero <= (formats?.length ?? 0)) allerA(numero - 1);
    setSaisie("");
  }

  // Recherche : les occurrences dans tout le document, un quart de seconde après la dernière touche.
  useEffect(() => {
    if (!chemin || recherche.trim() === "") {
      setResultats(null);
      return;
    }
    let valable = true;
    const minuterie = window.setTimeout(() => {
      textesDe(chemin).then(
        (textesPages) => {
          if (!valable) return;
          const trouvees = occurrences(textesPages, recherche);
          setResultats(trouvees);
          setActuel(0);
          if (trouvees.length > 0) montrerResultat(trouvees[0]);
        },
        () => valable && setResultats([]),
      );
    }, 250);
    return () => {
      valable = false;
      window.clearTimeout(minuterie);
    };
    // montrerResultat ne lit que des références et la recherche en cours.
  }, [recherche, chemin]);

  /** Fait défiler jusqu'à une occurrence : sa page, puis le mot trouvé, au tiers de la zone. */
  function montrerResultat(occurrence: Occurrence) {
    if (!chemin) return;
    allerA(occurrence.page);
    motsDe(chemin, occurrence.page).then(
      (mots) => {
        const couverts = motsCouverts(mots, recherche)[occurrence.rang];
        const zone = defilement.current;
        const format = formats?.[occurrence.page];
        if (!couverts || !zone || !format) return;
        const mot = mots[couverts[0]];
        allerA(occurrence.page, hautAffiche(format, mot.x, mot.y, rotation_.current) * echelle_.current - zone.clientHeight / 3);
      },
      () => undefined,
    );
  }

  function changerResultat(pas: number) {
    if (!resultats || resultats.length === 0) return;
    const suivant = (actuel + pas + resultats.length) % resultats.length;
    setActuel(suivant);
    montrerResultat(resultats[suivant]);
  }

  // Raccourcis : ⌘ (ou Ctrl) + F, + / − / 0, P, L (plein écran) ; Ctrl + Tab : onglet suivant.
  useEffect(() => {
    if (!actif || !formats || pleinEcran) return;
    const touche = (evenement: KeyboardEvent) => {
      if (evenement.ctrlKey && evenement.key === "Tab" && ouverts.length > 1 && chemin) {
        const rang = ouverts.indexOf(chemin);
        onOnglet(ouverts[(rang + (evenement.shiftKey ? -1 : 1) + ouverts.length) % ouverts.length]);
        evenement.preventDefault();
        return;
      }
      if (!(evenement.metaKey || evenement.ctrlKey)) return;
      if (evenement.key === "f" || evenement.key === "F") champRecherche.current?.focus();
      else if (evenement.key === "l" || evenement.key === "L") setPleinEcran(true);
      else if (evenement.key === "+" || evenement.key === "=") agrandir();
      else if (evenement.key === "-") reduire();
      else if (evenement.key === "0") setMode("largeur");
      else if (evenement.key === "p" || evenement.key === "P") void imprimer();
      else return;
      evenement.preventDefault();
    };
    window.addEventListener("keydown", touche);
    return () => window.removeEventListener("keydown", touche);
  });

  function agrandir() {
    setMode("libre");
    changerEchelle(Math.min(ECHELLE_MAX, echelle * 1.25));
  }
  function reduire() {
    setMode("libre");
    changerEchelle(Math.max(ECHELLE_MIN, echelle / 1.25));
  }

  // Éditeur : les annotations du document ouvert, avec leur historique (annuler, rétablir).
  const historique = (chemin && historiques[chemin]) || HISTORIQUE_VIDE;
  const annotations = historique.present;

  function changerHistorique(transformer: (avant: Historique) => Historique) {
    if (!chemin) return;
    setHistoriques((avant) => ({ ...avant, [chemin]: transformer(avant[chemin] ?? HISTORIQUE_VIDE) }));
  }
  const ajouterAnnotation = (annotation: Annotation) => changerHistorique((h) => appliquer(h, [...h.present, annotation]));
  const modifierAnnotation = (annotation: Annotation) => changerHistorique((h) => appliquer(h, h.present.map((a) => (a.id === annotation.id ? annotation : a))));
  function supprimerAnnotation(id: string) {
    changerHistorique((h) => appliquer(h, h.present.filter((a) => a.id !== id)));
    setSelection(null);
  }
  function effacerAnnotations(ids: string[]) {
    changerHistorique((h) => appliquer(h, sans(h.present, ids)));
    setSelection(null);
  }
  /** Liste des commentaires : mène à celui-ci (son haut au tiers de la zone) et le choisit. */
  function allerAuCommentaire(annotation: Annotation) {
    setOutil("selection");
    setSelection(annotation.id);
    const tiers = (defilement.current?.clientHeight ?? 0) / 3;
    const zone = zoneDe(annotation);
    const format = formats?.[annotation.page];
    allerA(annotation.page, format ? Math.max(0, hautAffiche(format, zone.x, zone.y, rotation) * echelle - tiers) : 0);
  }
  /** Liste des commentaires : mène à un commentaire du PDF et le montre un instant sur sa page. */
  function allerAuCommentairePdf(commentaire: CommentairePdf) {
    setSelection(null);
    setChoisiPdf(commentaire.id);
    setRepere({ page: commentaire.page, zone: commentaire });
    const tiers = (defilement.current?.clientHeight ?? 0) / 3;
    const format = formats?.[commentaire.page];
    allerA(commentaire.page, format ? Math.max(0, hautAffiche(format, commentaire.x, commentaire.y, rotation) * echelle - tiers) : 0);
  }
  /** Supprime un commentaire du PDF, ou change son texte : une étape de l'historique (annuler, rétablir). */
  function supprimerCommentairePdf(commentaire: CommentairePdf) {
    ajouterAnnotation({ id: nouvelId(), sorte: "suppression", page: commentaire.page, cible: commentaire.id });
    setChoisiPdf(null);
    setRepere(null);
  }
  function modifierCommentairePdf(commentaire: CommentairePdf, texte: string) {
    ajouterAnnotation({ id: nouvelId(), sorte: "retouche", page: commentaire.page, cible: commentaire.id, texte });
  }
  /**
   * La signature choisie dans la fenêtre : posée dans le champ de signature d'où l'on vient, à sa
   * taille ; sinon, l'outil Signature la pose là où l'on clique.
   */
  function choisirSignature(choix: SignatureChoisie) {
    const cible = fenetreSignature?.cible ?? null;
    setFenetreSignature(null);
    if (cible) {
      ajouterAnnotation({ id: nouvelId(), sorte: "signature", page: cible.page, zone: zoneDansCadre(choix.trace, cible.zone), trace: choix.trace, teinte: choix.teinte, initiales: choix.initiales });
      return;
    }
    setSignatureChoisie(choix);
    setOutil("signature");
    setSelection(null);
  }
  /** Une annotation posée sur la page ; après une signature, on revient à la sélection pour l'ajuster. */
  function poser(annotation: Annotation) {
    ajouterAnnotation(annotation);
    if (annotation.sorte === "signature") setOutil("selection");
  }

  /** Un champ du formulaire rempli : une étape de l'historique (annuler, rétablir). */
  function changerChamp(champ: ChampFormulaire, valeur: ValeurChamp) {
    ajouterAnnotation({ id: nouvelId(), sorte: "champ", page: champ.widgets[0]?.page ?? 0, nom: champ.nom, valeur });
  }
  function remonter(sens: "annuler" | "retablir") {
    changerHistorique(sens === "annuler" ? annuler : retablir);
    setSelection(null);
  }

  /**
   * Change d'outil ; « Image » demande d'abord l'image, posée au milieu de la page lue ;
   * « Signature » ouvre d'abord la fenêtre où l'on dessine (ou retrouve) sa signature.
   */
  async function choisirOutil(nouvel: OutilAnnotation) {
    if (nouvel === "signature") {
      setFenetreSignature({ cible: null });
      return;
    }
    if (nouvel !== "image") {
      setOutil(nouvel);
      if (nouvel !== "selection") setSelection(null);
      return;
    }
    const choisi = await open({ multiple: false, directory: false, filters: [{ name: t.annotation.images, extensions: Object.keys(TYPES_IMAGES) }] });
    if (typeof choisi !== "string" || !formats) return;
    try {
      const image = imagesChargees[choisi] ?? (await chargerImage(choisi));
      setImagesChargees((avant) => ({ ...avant, [choisi]: image }));
      const page = courante_.current;
      const format = formats[page];
      const largeur = Math.min(format.largeur * 0.3, 200);
      const hauteur = largeur / image.proportion;
      const id = nouvelId();
      ajouterAnnotation({ id, sorte: "image", page, chemin: choisi, zone: { x: (format.largeur - largeur) / 2, y: (format.hauteur - hauteur) / 2, largeur, hauteur } });
      setSelection(id);
      setOutil("selection");
    } catch (e) {
      signalerErreur(e);
    }
  }

  /**
   * Reconnaît, en mémoire, le texte du PDF numérisé : la recherche et la copie marchent ensuite, et
   * les retouches en cours (commentaires, champs) se refont sur le PDF avec son texte.
   */
  async function reconnaitreTexte() {
    if (!chemin) return;
    const document = chemin;
    try {
      const bilan = await reconnaissance.lancer(document);
      oublierTextes(document);
      delete retouchesEnvoyees.current[document];
      setReconnu(bilan);
      setVersionImages((version) => version + 1);
      setOuverture((n) => n + 1);
    } catch (e) {
      signalerErreur(e);
    }
  }

  /** Enregistre, à côté de l'original, le PDF avec son texte reconnu (tel qu'il est en mémoire). */
  async function enregistrerTexte() {
    if (!chemin) return;
    const sortie = await save({ defaultPath: aCote(chemin, nomTexteReconnu(nomDe(chemin))), filters: [{ name: t.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    if (memeFichier(sortie, chemin)) {
      signalerErreur({ message: t.annotation.memeFichier });
      return;
    }
    setEnregistrementTexte(true);
    try {
      await enregistrerLecture(chemin, sortie);
      notifications.show({ color: "green", message: t.numerise.enregistre(nomDe(sortie)), autoClose: 8000 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnregistrementTexte(false);
    }
  }

  /**
   * Enregistre un nouveau PDF, à côté de l'original, puis l'affiche : les annotations, les retouches
   * des commentaires et les champs remplis ; `avecAplatir` : le formulaire rejoint ses pages.
   */
  async function enregistrer(avecAplatir = false) {
    if (!chemin || (annotations.length === 0 && !avecAplatir)) return;
    const sorte = sorteDEnregistrement(annotations);
    const nom = (sorte === "signe" ? nomSigne : sorte === "rempli" ? nomRempli : nomAnnote)(nomDe(chemin));
    const sortie = await save({ defaultPath: aCote(chemin, nom), filters: [{ name: t.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    if (memeFichier(sortie, chemin)) {
      signalerErreur({ message: t.annotation.memeFichier });
      return;
    }
    setEnregistrement(true);
    try {
      await annoterPdf(chemin, pourLeCoeur(annotations), sortie, avecAplatir, retouches);
      // Un PDF signé numériquement est enregistré à la suite de sa version signée (sauf aplati) ;
      // seuls corriger un mot et le correcteur blanc changent ses pages.
      const signe = !avecAplatir && (signatures ?? []).some((s) => s.integrite === "intacte");
      const pagesChangees = annotations.some((a) => a.sorte === "correction" || a.sorte === "blanc");
      setSignaturesGardees(signe ? (pagesChangees ? "pagesChangees" : "gardees") : null);
      const source = chemin;
      setHistoriques(({ [source]: _enregistrees, ...autres }) => autres);
      // Le PDF enregistré garde les retouches ; l'original (toujours ouvert dans son onglet) retrouve les siennes.
      if (retouches.supprimes.length > 0 || retouches.textes.length > 0 || retouches.champs.length > 0) {
        const aucune = { supprimes: [], textes: [], champs: [] };
        retouchesEnvoyees.current[source] = JSON.stringify(aucune);
        void retoucherLecture(source, aucune).then(() => relire(source), signalerErreur);
      }
      setSelection(null);
      setAnnoter(false);
      setAplatir(false);
      setSorteEnregistree(sorte);
      setEnregistre(sortie);
      onChoisir(sortie);
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnregistrement(false);
    }
  }

  // Formulaire : les valeurs reçues avec le PDF, les valeurs actuelles (après les champs remplis ici).
  const initiales = useMemo(() => initialesDe(formulaire), [formulaire]);
  const valeurs = useMemo(() => ({ ...initiales, ...valeursChamps(annotations) }), [initiales, annotations]);
  const champsRemplissables = useMemo(() => formulaire?.champs.filter(remplissable) ?? [], [formulaire]);
  /** Les champs posés sur les pages : ceux à remplir, et ceux où signer. */
  const champsSurLesPages = useMemo(() => formulaire?.champs.filter((champ) => remplissable(champ) || signable(champ)) ?? [], [formulaire]);
  const aFormulaire = champsSurLesPages.length > 0 || Boolean(formulaire?.xfaSeul);

  // Les retouches du PDF (ses commentaires, ses champs) suivent l'historique : envoyées au cœur (qui
  // les applique à sa copie en mémoire, depuis l'origine), puis les pages et les vignettes relues.
  const retouches = useMemo(() => retouchesDe(annotations, initiales), [annotations, initiales]);
  useEffect(() => {
    if (!chemin || !formats) return;
    const cle = JSON.stringify(retouches);
    const vide = retouches.supprimes.length === 0 && retouches.textes.length === 0 && retouches.champs.length === 0;
    const avant = retouchesEnvoyees.current[chemin];
    if (cle === avant || (avant === undefined && vide)) return;
    retouchesEnvoyees.current[chemin] = cle;
    const document = chemin;
    retoucherLecture(document, retouches).then(() => {
      relire(document);
      setVersionImages((version) => version + 1);
    }, signalerErreur);
  }, [chemin, formats, retouches]);
  // Le cadre qui montre un commentaire du PDF s'efface après deux clignotements.
  useEffect(() => {
    if (!repere) return;
    const minuterie = window.setTimeout(() => setRepere(null), 2200);
    return () => window.clearTimeout(minuterie);
  }, [repere]);

  // Capturer : l'éditeur et la capture ne vont pas ensemble ; un autre document (la capture
  // enregistrée qu'on ouvre, un autre onglet) se lit normalement.
  useEffect(() => {
    if (annoter) setCapturer(false);
  }, [annoter]);
  useEffect(() => {
    setCapturer(false);
    setCapture(null);
  }, [chemin]);
  useEffect(() => {
    if (!capturer) setCapture(null);
  }, [capturer]);
  // Le texte du cadre, lu dans les mots de sa page (aussi ceux d'un scan dont le texte est reconnu).
  useEffect(() => {
    setTexteCapture(null);
    if (!chemin || !capture) return;
    let actuel = true;
    motsDe(chemin, capture.page).then(
      (mots) => actuel && setTexteCapture(texteDansZone(mots, capture.zone)),
      () => actuel && setTexteCapture({ texte: "", mots: 0 }),
    );
    return () => {
      actuel = false;
    };
  }, [chemin, capture, versionImages]);
  // Échap efface le cadre tracé, puis quitte la capture (pas pendant une saisie).
  useEffect(() => {
    if (!actif || pleinEcran || !capturer) return;
    const touche = (evenement: KeyboardEvent) => {
      const cible = evenement.target as HTMLElement | null;
      if (evenement.key !== "Escape" || (cible && (cible.tagName === "INPUT" || cible.tagName === "TEXTAREA"))) return;
      if (capture) setCapture(null);
      else setCapturer(false);
    };
    window.addEventListener("keydown", touche);
    return () => window.removeEventListener("keydown", touche);
  }, [actif, pleinEcran, capturer, capture]);

  /** La zone capturée, comme le cœur la prend : sa page et ses fractions de page. */
  function zoneCapturee() {
    if (!chemin || !capture || !formats?.[capture.page]) return null;
    return { chemin, page: capture.page, zone: enFractions(capture.zone, formats[capture.page]) };
  }

  async function copierImageCapture() {
    const cible = zoneCapturee();
    if (!cible) return;
    setTravailCapture("image");
    try {
      await copierZone(cible.chemin, cible.page, cible.zone, Number(qualiteCapture));
      notifications.show({ color: "teal", title: t.capture.imageCopiee, message: t.capture.imageCopieeDetail, autoClose: 6000 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setTravailCapture(null);
    }
  }

  async function copierTexteCapture() {
    if (!texteCapture?.mots) return;
    try {
      await copierTexte(texteCapture.texte);
      notifications.show({ color: "teal", title: t.capture.texteCopie, message: t.capture.texteCopieDetail(texteCapture.mots), autoClose: 6000 });
    } catch (e) {
      signalerErreur(e);
    }
  }

  /** Enregistre la capture en image (PNG ou JPG, d'après le nom choisi) ou en PDF. */
  async function enregistrerCapture(enPdf: boolean) {
    const cible = zoneCapturee();
    if (!cible) return;
    const base = nomDe(cible.chemin).replace(/\.pdf$/i, "");
    const sortie = await save({
      defaultPath: aCote(cible.chemin, t.capture.nom(base, cible.page + 1, enPdf ? "pdf" : "png")),
      filters: enPdf
        ? [{ name: t.filtre, extensions: ["pdf"] }]
        : [
            { name: t.capture.filtrePng, extensions: ["png"] },
            { name: t.capture.filtreJpg, extensions: ["jpg", "jpeg"] },
          ],
    });
    if (!sortie) return;
    setTravailCapture(enPdf ? "pdf" : "fichier");
    setCaptureEnregistree(null);
    try {
      let enImage = false;
      if (enPdf) enImage = await enregistrerZonePdf(cible.chemin, cible.page, cible.zone, sortie);
      else await enregistrerZoneImage(cible.chemin, cible.page, cible.zone, Number(qualiteCapture), sortie);
      setCaptureEnregistree({ chemin: sortie, enImage });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setTravailCapture(null);
    }
  }

  // Raccourcis de l'éditeur (et d'un formulaire) : annuler, rétablir ; dans l'éditeur, supprimer
  // l'annotation choisie. Pas pendant une saisie.
  useEffect(() => {
    if (!actif || pleinEcran || !(annoter || aFormulaire)) return;
    const touche = (evenement: KeyboardEvent) => {
      const cible = evenement.target as HTMLElement | null;
      if (cible && (cible.tagName === "INPUT" || cible.tagName === "TEXTAREA" || cible.tagName === "SELECT" || cible.isContentEditable)) return;
      const commande = evenement.metaKey || evenement.ctrlKey;
      const lettre = evenement.key.toLowerCase();
      if (commande && lettre === "z") remonter(evenement.shiftKey ? "retablir" : "annuler");
      else if (commande && lettre === "y") remonter("retablir");
      else if (annoter && (evenement.key === "Delete" || evenement.key === "Backspace") && selection) supprimerAnnotation(selection);
      else if (annoter && evenement.key === "Escape") setSelection(null);
      else return;
      evenement.preventDefault();
    };
    window.addEventListener("keydown", touche);
    return () => window.removeEventListener("keydown", touche);
  });

  /** Ferme un onglet ; ses annotations non enregistrées restent jusqu'à la fermeture d'Automat-PDF. */
  function fermerOnglet(document: string) {
    const enCours = nombreDeModifications(historiques[document]?.present ?? []);
    if (enCours > 0) notifications.show({ color: "orange", message: t.onglets.gardees(enCours, nomDe(document)), autoClose: 9000 });
    onFermer(document);
  }

  /** Un quart de tour de plus pour l'affichage ; on reste sur la page lue. */
  function pivoter() {
    ancre.current = { page: courante_.current, fraction: 0 };
    setRotation((avant) => tourner(avant));
  }

  /**
   * Une page ou deux côte à côte (préférence gardée) ; on reste sur la page lue. Agrandies à la main,
   * deux pages ne tiendraient pas dans la zone (la seconde, hors de l'écran, passerait inaperçue) :
   * elles reprennent la largeur de la zone.
   */
  function basculerDisposition() {
    ancre.current = { page: courante_.current, fraction: 0 };
    if (mode === "libre") setMode("largeur");
    setDisposition((avant) => {
      const suivante = avant === "deux" ? "une" : "deux";
      try {
        localStorage.setItem(CLE_DISPOSITION, suivante);
      } catch {
        // la préférence ne vaut que pour cette fois
      }
      return suivante;
    });
  }

  function basculerVignettes() {
    setVignettes((avant) => {
      try {
        localStorage.setItem(CLE_VIGNETTES, avant ? "non" : "oui");
      } catch {
        // la préférence ne vaut que pour cette fois
      }
      return !avant;
    });
  }

  if (!chemin) {
    return (
      <Stack gap="lg" maw={760}>
        <div>
          <Title order={2}>{t.titre}</Title>
          <Text c="dimmed">{t.videDetail}</Text>
        </div>
        <ZoneDepot invite={t.deposer} detail={t.vide} bouton={t.ouvrir} icone={IconFileText} onChoisir={() => void choisir()} />
      </Stack>
    );
  }

  const nombre = formats?.length ?? 0;
  const details = [formats ? t.pages(nombre) : null, taille].filter(Boolean).join(" · ");
  /** Annotations et retouches de commentaires en cours (les champs remplis se comptent à part). */
  const nombreAnnotations = nombreDeModifications(annotations.filter((a) => a.sorte !== "champ"));
  const occurrenceActuelle = resultats && resultats.length > 0 ? resultats[actuel] : null;
  const aSignets = (navigation?.signets.length ?? 0) > 0;
  return (
    <Stack ref={cadre} gap="md" h={hauteur} data-lecteur>
      {/* Onglets : les PDF ouverts, dès qu'il y en a deux. Les annotations non enregistrées y sont comptées. */}
      {ouverts.length > 1 && (
        <div className="onglets-lecteur" role="tablist" aria-label={t.onglets.titre} data-onglets>
          {ouverts.map((document) => {
            const enCours = nombreDeModifications(historiques[document]?.present ?? []);
            return (
              <div key={document} className="onglet-lecteur" data-actif={document === chemin || undefined}>
                <UnstyledButton className="onglet-lecteur__nom" role="tab" aria-selected={document === chemin} title={document} onClick={() => onOnglet(document)} data-onglet={nomDe(document)}>
                  <IconFileText size={15} stroke={1.7} />
                  <span>{nomDe(document)}</span>
                  {enCours > 0 && (
                    <Badge size="xs" circle color="orange" title={t.onglets.nonEnregistrees(enCours)}>
                      {enCours}
                    </Badge>
                  )}
                </UnstyledButton>
                <CloseButton size="sm" radius="xl" aria-label={t.onglets.fermer(nomDe(document))} onClick={() => fermerOnglet(document)} />
              </div>
            );
          })}
        </div>
      )}
      {/* En-tête : la première page, le nom du document, et de quoi l'ouvrir ailleurs ou en ouvrir un autre. */}
      <Group justify="space-between" wrap="nowrap" gap="md" align="center">
        <Group gap="md" wrap="nowrap" miw={0} style={{ flex: 1 }}>
          <Vignette chemin={chemin} page={0} taille={64} largeur={160} alt="" />
          <Box miw={0}>
            <Title order={2} lineClamp={1} title={chemin}>
              {nomDe(chemin)}
            </Title>
            <Text size="sm" c="dimmed">
              {details || " "}
            </Text>
          </Box>
        </Group>
        <Group gap={6} wrap="nowrap">
          <Tooltip label={t.annotation.annoterAide}>
            <Button
              radius="xl"
              color="eau"
              variant={annoter ? "filled" : "light"}
              leftSection={<IconPencil size={17} stroke={1.8} />}
              rightSection={!annoter && nombreAnnotations > 0 ? <Badge size="sm" circle color="orange">{nombreAnnotations}</Badge> : undefined}
              onClick={() => {
                setAnnoter((avant) => !avant);
                setSelection(null);
              }}
              disabled={!formats}
              aria-pressed={annoter}
              data-annoter
            >
              {t.annotation.annoter}
            </Button>
          </Tooltip>
          <Tooltip label={t.capture.aide} multiline w={300}>
            <Button
              radius="xl"
              color="eau"
              variant={capturer ? "filled" : "light"}
              leftSection={<IconScreenshot size={17} stroke={1.8} />}
              onClick={() => {
                if (!capturer) {
                  setAnnoter(false);
                  setSelection(null);
                }
                setCapturer((avant) => !avant);
              }}
              disabled={!formats}
              aria-pressed={capturer}
              data-capturer
            >
              {t.capture.bouton}
            </Button>
          </Tooltip>
          <Tooltip label={t.vignettes}>
            <ActionIcon className="bouton-verre" variant="subtle" color="gray" size={36} radius="md" onClick={basculerVignettes} aria-label={t.vignettes} aria-pressed={vignettes}>
              <IconLayoutSidebar size={18} stroke={1.8} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label={t.autre}>
            <ActionIcon className="bouton-verre" variant="subtle" color="gray" size={36} radius="md" onClick={() => void choisir()} aria-label={t.autre}>
              <IconPlus size={18} stroke={1.8} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label={t.montrer}>
            <ActionIcon className="bouton-verre" variant="subtle" color="gray" size={36} radius="md" onClick={() => void montrerDansDossier(chemin)} aria-label={t.montrer}>
              <IconFolder size={18} stroke={1.8} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label={impression ? t.impressionEnCours(impression.faites, impression.total) : t.imprimer}>
            <ActionIcon className="bouton-verre" variant="subtle" color="gray" size={36} radius="md" onClick={() => void imprimer()} loading={impression !== null} disabled={!formats} aria-label={t.imprimer} data-imprimer>
              <IconPrinter size={18} stroke={1.8} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label={t.pleinEcran.ouvrir}>
            <ActionIcon className="bouton-verre" variant="subtle" color="gray" size={36} radius="md" onClick={() => setPleinEcran(true)} disabled={!formats} aria-label={t.pleinEcran.ouvrir} data-plein-ecran>
              <IconMaximize size={18} stroke={1.8} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label={t.applicationHabituelle}>
            <ActionIcon className="bouton-verre" variant="subtle" color="gray" size={36} radius="md" onClick={() => void ouvrirFichier(chemin)} aria-label={t.applicationHabituelle}>
              <IconExternalLink size={18} stroke={1.8} />
            </ActionIcon>
          </Tooltip>
        </Group>
      </Group>

      {/* Recherche, page et zoom, sur une ligne : une pastille de verre au-dessus des pages. */}
      <Group justify="space-between" wrap="wrap" gap="xs" className="commandes-lecteur">
        <TextInput
          ref={champRecherche}
          size="xs"
          radius="xl"
          w={320}
          maw="100%"
          placeholder={t.rechercher}
          aria-label={t.rechercherAide}
          title={t.rechercherAide}
          leftSection={<IconSearch size={14} />}
          rightSectionWidth={resultats ? 128 : undefined}
          rightSection={
            resultats ? (
              <Group gap={2} wrap="nowrap" pr={4}>
                <Text size="xs" c="dimmed" style={{ whiteSpace: "nowrap" }} data-resultats>
                  {resultats.length === 0 ? t.aucunResultat : t.resultat(actuel + 1, resultats.length)}
                </Text>
                <ActionIcon size="sm" variant="subtle" color="gray" onClick={() => changerResultat(-1)} disabled={resultats.length === 0} aria-label={t.precedent}>
                  <IconChevronUp size={14} />
                </ActionIcon>
                <ActionIcon size="sm" variant="subtle" color="gray" onClick={() => changerResultat(1)} disabled={resultats.length === 0} aria-label={t.suivant}>
                  <IconChevronDown size={14} />
                </ActionIcon>
                <CloseButton size="sm" onClick={() => setRecherche("")} aria-label={t.effacer} />
              </Group>
            ) : null
          }
          value={recherche}
          disabled={!formats}
          onChange={(evenement) => setRecherche(evenement.currentTarget.value)}
          onKeyDown={(evenement) => {
            if (evenement.key === "Enter") changerResultat(evenement.shiftKey ? -1 : 1);
            else if (evenement.key === "Escape") setRecherche("");
          }}
          data-recherche-lecteur
        />
        <Group gap={6} wrap="nowrap">
          <Group gap={6} wrap="nowrap">
            <Text size="sm">{t.page}</Text>
            <NumberInput
              size="xs"
              w={64}
              min={1}
              max={Math.max(1, nombre)}
              hideControls
              value={saisie === "" ? courante + 1 : saisie}
              disabled={!formats}
              onChange={setSaisie}
              onKeyDown={(evenement) => evenement.key === "Enter" && validerSaisie()}
              onBlur={validerSaisie}
              aria-label={t.page}
              data-page-courante
            />
            <Text size="sm" c="dimmed" style={{ whiteSpace: "nowrap" }}>
              {t.sur(nombre)}
            </Text>
          </Group>
          <Tooltip label={t.reduire}>
            <ActionIcon variant="subtle" color="gray" onClick={reduire} disabled={!formats} aria-label={t.reduire}>
              <IconZoomOut size={20} stroke={1.6} />
            </ActionIcon>
          </Tooltip>
          <Text size="sm" w={44} ta="center" data-zoom>
            {Math.round((echelle / TAILLE_REELLE) * 100)} %
          </Text>
          <Tooltip label={t.agrandir}>
            <ActionIcon variant="subtle" color="gray" onClick={agrandir} disabled={!formats} aria-label={t.agrandir}>
              <IconZoomIn size={20} stroke={1.6} />
            </ActionIcon>
          </Tooltip>
          {/* Une seule page : rien à mettre côte à côte (l'infobulle le dit, sur le bouton grisé). */}
          <Tooltip label={nombre === 1 ? t.deuxPagesUneSeule : t.deuxPages}>
            <span style={{ display: "inline-flex" }}>
              <ActionIcon
                variant={disposition === "deux" && nombre > 1 ? "light" : "subtle"}
                color={disposition === "deux" && nombre > 1 ? "eau" : "gray"}
                onClick={basculerDisposition}
                disabled={!formats || nombre < 2}
                aria-label={t.deuxPages}
                aria-pressed={disposition === "deux" && nombre > 1}
                data-deux-pages
              >
                <IconBook2 size={19} stroke={1.6} />
              </ActionIcon>
            </span>
          </Tooltip>
          <Tooltip label={rotation ? t.pivoterDe(rotation) : t.pivoter} multiline w={240}>
            <ActionIcon variant={rotation ? "light" : "subtle"} color={rotation ? "eau" : "gray"} onClick={pivoter} disabled={!formats} aria-label={t.pivoter} data-pivoter={rotation}>
              <IconRotateClockwise size={19} stroke={1.6} />
            </ActionIcon>
          </Tooltip>
          <SegmentedControl
            size="xs"
            radius="xl"
            value={mode === "libre" ? "" : mode}
            onChange={(valeur) => setMode(valeur as Mode)}
            disabled={!formats}
            data={[
              { value: "largeur", label: t.largeur },
              { value: "page", label: t.entiere },
            ]}
            aria-label={`${t.largeurAide} / ${t.entiereAide}`}
          />
        </Group>
      </Group>

      {annoter && (
        <BarreAnnotations
          outil={outil}
          onOutil={(nouvel) => void choisirOutil(nouvel)}
          teinteSurligneur={teinteSurligneur}
          onTeinteSurligneur={setTeinteSurligneur}
          teinteTexte={teinteTexte}
          onTeinteTexte={setTeinteTexte}
          taille={tailleTexte}
          onTaille={setTailleTexte}
          reglages={reglages}
          onReglages={setReglages}
          nombre={nombreAnnotations}
          peutAnnuler={historique.passe.length > 0}
          peutRetablir={historique.futur.length > 0}
          onAnnuler={() => remonter("annuler")}
          onRetablir={() => remonter("retablir")}
          selection={selection !== null}
          onSupprimer={() => selection && supprimerAnnotation(selection)}
          enregistrement={enregistrement}
          onEnregistrer={() => void enregistrer()}
          onTerminer={() => {
            setAnnoter(false);
            setSelection(null);
          }}
          signe={(signatures ?? []).some((s) => s.integrite === "intacte")}
        />
      )}
      {capturer && formats && (
        <BarreCapture
          capture={capture}
          mots={texteCapture?.mots ?? null}
          qualite={qualiteCapture}
          onQualite={setQualiteCapture}
          enCours={travailCapture}
          onCopierImage={() => void copierImageCapture()}
          onCopierTexte={() => void copierTexteCapture()}
          onEnregistrerImage={() => void enregistrerCapture(false)}
          onEnregistrerPdf={() => void enregistrerCapture(true)}
          onTerminer={() => setCapturer(false)}
        />
      )}
      {captureEnregistree && (
        <Alert color="teal" icon={<IconCircleCheck size={20} />} title={t.capture.enregistree} withCloseButton onClose={() => setCaptureEnregistree(null)} py={8} data-capture-enregistree>
          <Group justify="space-between" gap="sm" wrap="wrap">
            <Stack gap={2}>
              <Text size="sm">{t.capture.enregistreeDetail(nomDe(captureEnregistree.chemin))}</Text>
              {captureEnregistree.enImage && <Text size="sm">{t.capture.enImage}</Text>}
            </Stack>
            <Group gap="xs">
              <Button
                size="xs"
                variant="default"
                leftSection={<IconExternalLink size={15} />}
                onClick={() =>
                  /\.pdf$/i.test(captureEnregistree.chemin) ? onChoisir(captureEnregistree.chemin) : void ouvrirFichier(captureEnregistree.chemin).catch(signalerErreur)
                }
                data-ouvrir-capture
              >
                {t.capture.ouvrir}
              </Button>
              <Button size="xs" variant="default" leftSection={<IconFolder size={15} />} onClick={() => void montrerDansDossier(captureEnregistree.chemin).catch(signalerErreur)}>
                {textes.resultat.montrer}
              </Button>
            </Group>
          </Group>
        </Alert>
      )}
      {enregistre && chemin && memeFichier(enregistre, chemin) && (
        <Alert
          color="teal"
          icon={<IconCircleCheck size={20} />}
          title={sorteEnregistree === "rempli" ? t.formulaire.enregistre : sorteEnregistree === "signe" ? t.signature.enregistre : t.annotation.enregistre}
          withCloseButton
          onClose={() => setEnregistre(null)}
          py={8}
          data-annote-enregistre
        >
          <Group justify="space-between" gap="sm" wrap="wrap">
            <Stack gap={2}>
              <Text size="sm">{t.annotation.enregistreDetail(nomDe(enregistre))}</Text>
              {signaturesGardees && (
                <Text size="sm" data-signatures-gardees={signaturesGardees}>
                  {t.signaturesNumeriques[signaturesGardees]}
                </Text>
              )}
            </Stack>
            <Button size="xs" variant="default" leftSection={<IconFolder size={15} />} onClick={() => void montrerDansDossier(enregistre).catch(signalerErreur)}>
              {textes.resultat.montrer}
            </Button>
          </Group>
        </Alert>
      )}

      {signatures && signatures.length > 0 && formats && !annoter && (
        <BandeauSignatures signatures={signatures} onDetails={() => setPanneauSignatures({ enAvant: null })} />
      )}

      {(numerise || reconnu) && formats && !annoter && (
        <BandeauNumerise
          enCours={reconnaissance.enCours}
          avancement={reconnaissance.avancement}
          bilan={reconnu}
          enregistrement={enregistrementTexte}
          onReconnaitre={() => void reconnaitreTexte()}
          onEnregistrer={() => void enregistrerTexte()}
          onFermer={() => setReconnu(null)}
        />
      )}

      {chemin && signatures && signatures.length > 0 && (
        <PanneauSignatures
          ouvert={panneauSignatures !== null}
          onFermer={() => setPanneauSignatures(null)}
          chemin={chemin}
          signatures={signatures}
          enAvant={panneauSignatures?.enAvant ?? null}
          onVoir={(signature) => allerA(signature.page!, Math.max(0, (signature.zone?.[1] ?? 0) * echelle - 120))}
          onOuvrir={onChoisir}
        />
      )}

      {formulaire && formats && !annoter && aFormulaire && (
        <BandeauFormulaire
          formulaire={formulaire}
          aRemplir={champsRemplissables.length > 0}
          remplis={retouches.champs.length}
          signe={annotations.some((a) => a.sorte === "signature")}
          aplatir={aplatir}
          onAplatir={setAplatir}
          enregistrement={enregistrement}
          onEnregistrer={() => void enregistrer(aplatir)}
        />
      )}

      {deverrouille && formats && (
        <Alert color="blue" variant="light" withCloseButton onClose={() => setDeverrouille(false)} mb="xs" data-deverrouille>
          {t.motDePasse.ouvert}
        </Alert>
      )}

      {/* Faire d'Automat-PDF le lecteur des PDF : proposé une fois par session, tant que ni choisi ni refusé. */}
      {formats && !annoter && <PropositionLecteurParDefaut />}

      {chemin && formats && (
        <FenetreImpression
          ouvert={fenetreImpression}
          chemin={chemin}
          formats={formats}
          courante={courante}
          avecAnnotations={pourLeCoeur(annotations).length > 0}
          onFermer={() => setFenetreImpression(false)}
          onImprimer={(options) => void lancerImpression(options)}
        />
      )}

      <FenetreSignature
        ouvert={fenetreSignature !== null}
        onFermer={() => setFenetreSignature(null)}
        onChoisir={choisirSignature}
        onImage={() => {
          setFenetreSignature(null);
          if (!annoter) setAnnoter(true);
          void choisirOutil("image");
        }}
      />

      {pleinEcran && chemin && formats && (
        <PleinEcran
          chemin={chemin}
          formats={formats}
          depart={courante}
          rotation={rotation}
          annotations={annotations}
          images={imagesChargees}
          onFermer={(page) => {
            setPleinEcran(false);
            // Retour à la page vue en dernier, une fois la fenêtre revenue à sa taille.
            requestAnimationFrame(() => allerA(page));
          }}
        />
      )}

      <Modal opened={lienExterne !== null} onClose={() => setLienExterne(null)} title={t.lienExterne.titre} centered>
        <Stack gap="sm">
          <Text size="sm">{t.lienExterne.texte}</Text>
          <Text size="sm" fw={600} style={{ overflowWrap: "anywhere" }} data-adresse-lien>
            {lienExterne}
          </Text>
          <Text size="xs" c="dimmed">
            {t.lienExterne.prudence}
          </Text>
          <Group justify="flex-end" gap="sm">
            <Button variant="default" onClick={() => setLienExterne(null)}>
              {t.lienExterne.annuler}
            </Button>
            <Button
              leftSection={<IconExternalLink size={15} />}
              onClick={() => {
                const adresse = lienExterne;
                setLienExterne(null);
                if (adresse) void ouvrirLien(adresse).catch(signalerErreur);
              }}
            >
              {t.lienExterne.ouvrir}
            </Button>
          </Group>
        </Stack>
      </Modal>

      <Group align="stretch" gap="md" wrap="nowrap" style={{ flex: 1, minHeight: 0 }}>
        {vignettes && formats && (formats.length > 1 || aSignets) && (
          <div className="panneau-lateral" data-signets={aSignets || undefined}>
            {aSignets && (
              <SegmentedControl
                size="xs"
                fullWidth
                value={panneau}
                onChange={(valeur) => setPanneau(valeur === "signets" ? "signets" : "pages")}
                data={[
                  { value: "pages", label: t.pagesVignettes },
                  { value: "signets", label: t.signets },
                ]}
              />
            )}
            {aSignets && panneau === "signets" ? (
              <ColonneSignets signets={navigation!.signets} courante={courante} onSuivre={suivreLien} />
            ) : (
              <ColonneVignettes chemin={chemin} nombre={formats.length} courante={courante} onPage={(index) => allerA(index)} />
            )}
          </div>
        )}

        <Box ref={defilement} onScroll={suivre} className="zone-pages" data-pages>
          {protege && chemin ? (
            <MotDePasseLecture
              chemin={chemin}
              onOuvert={() => {
                setDeverrouille(true);
                setOuverture((n) => n + 1);
                relire(chemin);
              }}
            />
          ) : erreur ? (
            <Alert color="red" m="md" title={t.illisible}>
              {erreur}
            </Alert>
          ) : !formats ? (
            <Center h="100%">
              <Loader color="gray" />
            </Center>
          ) : (
            <Stack align="center" gap={MARGE} p={MARGE} style={{ width: "max-content", minWidth: "100%" }}>
              {rangees(formats.length, disposition).map((rangee) => (
                <Group key={rangee[0]} gap={MARGE} wrap="nowrap" align="flex-start" justify="center">
                  {rangee.map((index) => (
                    <PageLecteur
                      key={index}
                      chemin={chemin}
                      index={index}
                      format={formats[index]}
                      echelle={echelle}
                      rotation={rotation}
                      version={versionImages}
                      racine={defilement.current}
                      requete={resultats && resultats.length > 0 ? recherche : ""}
                      rangActuel={occurrenceActuelle?.page === index ? occurrenceActuelle.rang : -1}
                      liens={annoter || capturer ? undefined : navigation?.liens[index]}
                      onLien={(lien) => suivreLien(lien.destination, lien.adresse)}
                      formulaire={
                        !annoter && champsSurLesPages.some((champ) => champ.widgets.some((w) => w.page === index))
                          ? (versionImage) => (
                              <CoucheFormulaire
                                page={index}
                                echelle={echelle}
                                champs={champsSurLesPages}
                                valeurs={valeurs}
                                versionImage={versionImage}
                                onChanger={changerChamp}
                                signees={annotations.flatMap((a) => (a.sorte === "signature" && a.page === index ? [a.zone] : []))}
                                onSigner={(_champ, widget) => setFenetreSignature({ cible: { page: widget.page, zone: { x: widget.x, y: widget.y, largeur: widget.largeur, hauteur: widget.hauteur } } })}
                              />
                            )
                          : undefined
                      }
                      coucheSignatures={
                        !annoter && signatures && signaturesDeLaPage(signatures, index).length > 0 ? (
                          <CoucheSignatures page={index} echelle={echelle} signatures={signatures} onChoisir={(signature) => setPanneauSignatures({ enAvant: signature.champ })} />
                        ) : undefined
                      }
                      refPage={(element) => {
                        pages.current[index] = element;
                      }}
                      coucheCapture={
                        capturer ? (
                          <CoucheCapture
                            page={index}
                            format={formats[index]}
                            echelle={echelle}
                            rotation={rotation}
                            zone={capture?.page === index ? capture.zone : null}
                            onZone={(page, zone) => {
                              setCaptureEnregistree(null);
                              setCapture(zone ? { page, zone } : null);
                            }}
                          />
                        ) : undefined
                      }
                      couche={
                        annoter || annotations.some((a) => a.page === index)
                          ? (mots) => (
                              <CoucheAnnotations
                                page={index}
                                echelle={echelle}
                                rotation={rotation}
                                annotations={annotations.filter((a) => a.page === index)}
                                mots={mots}
                                actif={annoter}
                                outil={outil}
                                teinteSurligneur={teinteSurligneur}
                                teinteTexte={teinteTexte}
                                taille={tailleTexte}
                                reglages={reglages}
                                images={imagesChargees}
                                selection={selection}
                                onSelection={setSelection}
                                onAjouter={poser}
                                onModifier={modifierAnnotation}
                                onSupprimer={supprimerAnnotation}
                                onEffacer={effacerAnnotations}
                                repere={repere?.page === index ? repere.zone : null}
                                signature={outil === "signature" ? signatureChoisie : null}
                              />
                            )
                          : undefined
                      }
                    />
                  ))}
                </Group>
              ))}
            </Stack>
          )}
        </Box>

        {/* En annotation, la liste des commentaires ; sinon les outils qui s'appliquent à ce PDF : des
            cartes de verre, avec la pastille de leur groupe. */}
        {annoter ? (
          <ListeCommentaires
            annotations={annotations}
            existants={existants ?? []}
            supprimes={retouches.supprimes}
            textesChanges={retouches.textes}
            selection={selection}
            choisiPdf={choisiPdf}
            onChoisir={(annotation) => {
              setChoisiPdf(null);
              allerAuCommentaire(annotation);
            }}
            onSupprimer={supprimerAnnotation}
            onAllerPdf={allerAuCommentairePdf}
            onSupprimerPdf={supprimerCommentairePdf}
            onModifierPdf={modifierCommentairePdf}
          />
        ) : (
          <Stack gap={8} w={272} style={{ overflow: "auto", flexShrink: 0 }} data-outils-lecteur>
            <div>
              <Text fw={750} size="lg">
                {t.outils}
              </Text>
              <Text size="xs" c="dimmed">
                {t.outilsAide}
              </Text>
            </div>
            {outils.map((definition) => {
              const Icone = definition.icone;
              const { titre, description } = textes.outils[definition.cle];
              return (
                <UnstyledButton
                  key={definition.cle}
                  className="carte-action"
                  onClick={() => {
                    // La page lue part avec le PDF : Extraire des pages la propose déjà choisie.
                    if (chemin) confierPage(chemin, courante_.current);
                    onOutil(definition);
                  }}
                  disabled={!formats}
                  data-action={definition.cle}
                >
                  <span className="pastille" style={varsTeinte(GROUPE_DE[definition.cle].teinte)}>
                    <Icone size={17} stroke={1.8} />
                  </span>
                  <div style={{ minWidth: 0 }}>
                    <Text fw={650} size="sm" lh={1.25}>
                      {titre}
                    </Text>
                    <Text size="xs" c="dimmed" lineClamp={1}>
                      {description}
                    </Text>
                  </div>
                </UnstyledButton>
              );
            })}
          </Stack>
        )}
      </Group>
    </Stack>
  );
}

/** Colonne des vignettes : une par page, la page lue mise en avant et gardée à l'écran. */
function ColonneVignettes({ chemin, nombre, courante, onPage }: { chemin: string; nombre: number; courante: number; onPage: (index: number) => void }) {
  const elements = useRef<(HTMLButtonElement | null)[]>([]);
  useEffect(() => {
    elements.current[courante]?.scrollIntoView({ block: "nearest" });
  }, [courante]);
  return (
    <Stack gap={10} className="colonne-vignettes" data-vignettes-lecteur>
      {Array.from({ length: nombre }, (_, index) => (
        <UnstyledButton
          key={index}
          ref={(element: HTMLButtonElement | null) => {
            elements.current[index] = element;
          }}
          className="vignette-lecteur"
          data-courante={index === courante || undefined}
          onClick={() => onPage(index)}
          aria-label={t.alt(index + 1)}
          aria-current={index === courante ? "page" : undefined}
        >
          <Vignette chemin={chemin} page={index} taille={92} largeur={184} alt="" />
          <Text size="xs" c="dimmed" ta="center">
            {index + 1}
          </Text>
        </UnstyledButton>
      ))}
    </Stack>
  );
}

/** Mesure d'un texte dans la police de la couche de texte, pour la caler sur la largeur du mot. */
const toile = typeof document === "undefined" ? null : document.createElement("canvas").getContext("2d");
function largeurMesuree(texte: string, taillePx: number): number {
  if (!toile) return texte.length * taillePx * 0.5;
  toile.font = `${taillePx}px sans-serif`;
  return toile.measureText(texte).width || 1;
}

/** Place d'un mot sur la page affichée, en pixels CSS : position, longueur et taille, sens d'écriture. */
function placeDe(mot: Mot, echelle: number) {
  return {
    left: mot.x * echelle,
    top: mot.y * echelle,
    width: mot.longueur * echelle,
    height: mot.taille * echelle,
    transform: mot.sens ? `rotate(${mot.sens * 90}deg)` : undefined,
  };
}

interface PropsPage {
  chemin: string;
  index: number;
  format: FormatPage;
  echelle: number;
  /** Rotation de l'affichage : la page est dessinée droite, puis tournée dans sa boîte. */
  rotation: Rotation;
  /** Change quand le cœur a retouché le PDF (commentaires) : l'image de la page est redemandée. */
  version: number;
  racine: HTMLDivElement | null;
  /** Le texte cherché (vide : rien à surligner). */
  requete: string;
  /** Rang, dans cette page, de l'occurrence sur laquelle on est (−1 : aucune). */
  rangActuel: number;
  refPage: (element: HTMLDivElement | null) => void;
  /** Couche de l'éditeur (annotations), posée au-dessus du texte ; reçoit les mots de la page. */
  couche?: (mots: Mot[] | null) => ReactNode;
  /** Liens de la page (vers une page, un site, un e-mail) ; aucun pendant l'annotation. */
  liens?: LienPdf[];
  onLien?: (lien: LienPdf) => void;
  /** Champs du formulaire de la page ; reçoit la version de l'image affichée. */
  formulaire?: (versionImage: number) => ReactNode;
  /** Signatures numériques visibles sur la page : un clic les vérifie. */
  coucheSignatures?: ReactNode;
  /** Capturer une zone : la couche où se trace le cadre, au-dessus de tout le reste. */
  coucheCapture?: ReactNode;
}

/**
 * Une page : sa place est réservée tout de suite ; son image et ses mots viennent quand elle approche
 * de l'écran. Les mots, invisibles, se sélectionnent et se copient ; ceux que la recherche trouve sont
 * surlignés.
 */
function PageLecteur({ chemin, index, format, echelle, rotation, version, racine, requete, rangActuel, refPage, couche, liens, onLien, formulaire, coucheSignatures, coucheCapture }: PropsPage) {
  const boite = useRef<HTMLDivElement | null>(null);
  const [proche, setProche] = useState(false);
  /** L'image affichée, et la version du PDF qu'elle montre. */
  const [image, setImage] = useState<{ adresse: string; version: number } | null>(null);
  const [mots, setMots] = useState<Mot[] | null>(null);
  const largeur = Math.round(format.largeur * echelle);
  const hauteur = Math.round(format.hauteur * echelle);
  // Image un peu plus large que la page affichée (écran haute définition), par paliers de 200 pixels
  // pour resservir celles déjà dessinées ; le cœur n'en dessine pas de plus de 2 400 pixels.
  const demandee = Math.min(2400, Math.max(200, Math.ceil((largeur * (window.devicePixelRatio || 1)) / 200) * 200));

  useEffect(() => {
    const element = boite.current;
    if (!element || !racine) return;
    const observateur = new IntersectionObserver(([entree]) => setProche(entree.isIntersecting), { root: racine, rootMargin: "1200px 0px" });
    observateur.observe(element);
    return () => observateur.disconnect();
  }, [racine]);

  useEffect(() => {
    if (!proche) return;
    let actuelle = true;
    imagePage(chemin, index, demandee).then(
      (adresse) => actuelle && setImage({ adresse, version }),
      () => undefined,
    );
    return () => {
      actuelle = false;
    };
  }, [proche, chemin, index, demandee, version]);

  // Le PDF relu (texte reconnu, retouches) : ses mots sont redemandés.
  useEffect(() => setMots(null), [version]);
  useEffect(() => {
    if (!proche || mots) return;
    let actuelle = true;
    motsDe(chemin, index).then(
      (lus) => actuelle && setMots(lus),
      () => undefined,
    );
    return () => {
      actuelle = false;
    };
  }, [proche, chemin, index, mots]);

  const surlignes = useMemo(() => (mots && requete ? motsCouverts(mots, requete) : []), [mots, requete]);

  const couchee = rotation % 180 !== 0;
  return (
    <Box
      ref={(element: HTMLDivElement | null) => {
        boite.current = element;
        refPage(element);
      }}
      w={couchee ? hauteur : largeur}
      h={couchee ? largeur : hauteur}
      pos="relative"
      style={{ flexShrink: 0, background: "white", boxShadow: "var(--automat-ombre-page)" }}
      data-page={index + 1}
      data-rotation={rotation || undefined}
    >
      <div className="page-droite" style={{ width: largeur, height: hauteur, transform: transformation(rotation, largeur, hauteur) }}>
        {image ? (
          <img src={image.adresse} alt={t.alt(index + 1)} draggable={false} style={{ width: "100%", height: "100%", display: "block", userSelect: "none" }} />
        ) : (
          <Center h="100%">
            <Loader size="sm" color="gray" />
          </Center>
        )}
        {/* Surlignages et couche de texte : seulement près de l'écran, pour ne pas charger la fenêtre de
            milliers de mots invisibles. */}
        {proche &&
          surlignes.map((couverts, rang) =>
            couverts.map((i) => <div key={`${rang}-${i}`} className="surlignage" data-actuel={rang === rangActuel || undefined} style={placeDe(mots![i], echelle)} />),
          )}
        {proche && mots && mots.length > 0 && (
          <div className="couche-texte">
            {mots.map((mot, i) => {
              const place = placeDe(mot, echelle);
              const etirement = place.width / largeurMesuree(mot.texte, place.height);
              return (
                <Fragment key={i}>
                  {i > 0 && (mot.debutLigne ? <br /> : " ")}
                  <span style={{ ...place, width: undefined, fontSize: place.height, transform: `${place.transform ?? ""} scaleX(${etirement})` }}>{mot.texte}</span>
                </Fragment>
              );
            })}
          </div>
        )}
        {onLien &&
          liens?.map((lien, rang) => {
            const libelle = lien.adresse ?? (lien.destination ? t.lienPage(lien.destination.page + 1) : "");
            return (
              <button
                key={`lien-${rang}`}
                type="button"
                className="lien-pdf"
                style={{ left: lien.x * echelle, top: lien.y * echelle, width: lien.largeur * echelle, height: lien.hauteur * echelle }}
                title={libelle}
                aria-label={libelle}
                onClick={() => onLien(lien)}
                data-lien
              />
            );
          })}
        {formulaire?.(image?.version ?? -1)}
        {coucheSignatures}
        {couche?.(mots)}
        {coucheCapture}
      </div>
    </Box>
  );
}
