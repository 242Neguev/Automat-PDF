// Les outils de l'application, rangés par groupes : menu de gauche, accueil, fichiers reçus.
import type { ComponentType } from "react";
import {
  IconArrowsExchange,
  IconArrowsMinimize,
  IconEyeOff,
  IconFeather,
  IconFileExport,
  IconFiles,
  IconFileTypeDocx,
  IconFileTypePdf,
  IconFileTypeTxt,
  IconLayersIntersect,
  IconLayoutColumns,
  IconLayoutGrid,
  IconLock,
  IconNumbers,
  IconPencil,
  IconPhoto,
  IconPhotoPlus,
  IconScan,
  IconScissors,
  IconShieldLock,
  IconSignature,
  IconStack2,
  IconTextScan2,
  IconUsers,
} from "@tabler/icons-react";
import type { Teinte } from "./theme";
import Caviarder from "./pages/annoter/Caviarder";
import NumerosTampon from "./pages/annoter/NumerosTampon";
import SignatureCachet from "./pages/annoter/SignatureCachet";
import type { SorteFichier } from "./pages/commun/reception";
import Numeriser from "./pages/convertir/Numeriser";
import OfficeEnPdf from "./pages/convertir/OfficeEnPdf";
import PdfEnTexte from "./pages/convertir/PdfEnTexte";
import PdfEnWord from "./pages/convertir/PdfEnWord";
import ReconnaitreTexte from "./pages/convertir/ReconnaitreTexte";
import FusionPdf from "./pages/fusion/FusionPdf";
import ImagesEnPdf from "./pages/images/ImagesEnPdf";
import PdfEnImages from "./pages/images/PdfEnImages";
import IntercalerPdf from "./pages/intercaler/IntercalerPdf";
import MiseEnPage from "./pages/mise-en-page/MiseEnPage";
import Compresser from "./pages/optimiser/Compresser";
import OrganiserPdf from "./pages/organiser/OrganiserPdf";
import MotDePasse from "./pages/securite/MotDePasse";
import ExtrairePages from "./pages/separation/ExtrairePages";
import SeparationParPersonne from "./pages/separation/SeparationParPersonne";
import SeparationPdf from "./pages/separation/SeparationPdf";
import { textes } from "./textes";

export type Outil = keyof typeof textes.outils;

export interface DefinitionOutil {
  cle: Outil;
  icone: typeof IconFiles;
  page: ComponentType;
  /** Fichiers qu'il prend de l'Explorateur ou du Finder (clic droit, glisser-déposer). */
  accepte: SorteFichier[];
}

export interface Groupe {
  cle: keyof typeof textes.groupes;
  /** Couleur du groupe : sa carte sur l'accueil, la pastille de ses outils (theme.ts, TEINTES). */
  teinte: Teinte;
  icone: typeof IconFiles;
  outils: DefinitionOutil[];
}

export const GROUPES: Groupe[] = [
  {
    cle: "pages",
    teinte: "jaune",
    icone: IconLayoutGrid,
    outils: [
      { cle: "organiser", icone: IconLayoutGrid, page: OrganiserPdf, accepte: ["pdf"] },
      { cle: "miseEnPage", icone: IconLayoutColumns, page: MiseEnPage, accepte: ["pdf"] },
    ],
  },
  {
    cle: "assembler",
    teinte: "orange",
    icone: IconStack2,
    outils: [
      { cle: "fusionner", icone: IconFiles, page: FusionPdf, accepte: ["pdf"] },
      { cle: "intercaler", icone: IconLayersIntersect, page: IntercalerPdf, accepte: ["pdf"] },
      { cle: "images", icone: IconPhotoPlus, page: ImagesEnPdf, accepte: ["image"] },
    ],
  },
  {
    cle: "decouper",
    teinte: "lavande",
    icone: IconScissors,
    outils: [
      { cle: "extraire", icone: IconFileExport, page: ExtrairePages, accepte: ["pdf"] },
      { cle: "separer", icone: IconScissors, page: SeparationPdf, accepte: ["pdf", "liste"] },
      { cle: "parPersonne", icone: IconUsers, page: SeparationParPersonne, accepte: ["pdf", "liste"] },
    ],
  },
  {
    cle: "annoter",
    teinte: "citron",
    icone: IconPencil,
    outils: [
      { cle: "marques", icone: IconNumbers, page: NumerosTampon, accepte: ["pdf"] },
      { cle: "signature", icone: IconSignature, page: SignatureCachet, accepte: ["pdf", "image"] },
    ],
  },
  {
    cle: "securite",
    teinte: "ciel",
    icone: IconShieldLock,
    outils: [
      { cle: "motDePasse", icone: IconLock, page: MotDePasse, accepte: ["pdf"] },
      { cle: "caviarder", icone: IconEyeOff, page: Caviarder, accepte: ["pdf"] },
    ],
  },
  { cle: "optimiser", teinte: "rose", icone: IconFeather, outils: [{ cle: "compresser", icone: IconArrowsMinimize, page: Compresser, accepte: ["pdf"] }] },
  {
    cle: "convertir",
    teinte: "menthe",
    icone: IconArrowsExchange,
    outils: [
      { cle: "enImages", icone: IconPhoto, page: PdfEnImages, accepte: ["pdf"] },
      { cle: "officeEnPdf", icone: IconFileTypePdf, page: OfficeEnPdf, accepte: ["office"] },
      { cle: "pdfEnWord", icone: IconFileTypeDocx, page: PdfEnWord, accepte: ["pdf"] },
      { cle: "pdfEnTexte", icone: IconFileTypeTxt, page: PdfEnTexte, accepte: ["pdf"] },
      { cle: "numeriser", icone: IconScan, page: Numeriser, accepte: [] },
      { cle: "ocr", icone: IconTextScan2, page: ReconnaitreTexte, accepte: ["pdf"] },
    ],
  },
];

export const OUTILS = GROUPES.flatMap((groupe) => groupe.outils);

/** Le groupe de chaque outil (pour sa teinte). */
export const GROUPE_DE = Object.fromEntries(GROUPES.flatMap((groupe) => groupe.outils.map((outil) => [outil.cle, groupe]))) as Record<Outil, Groupe>;

/** Ce qui décrit un outil pour la recherche : son titre, sa description et ses mots-clés. */
export function texteDe(definition: DefinitionOutil): string {
  const { titre, description, motsCles } = textes.outils[definition.cle];
  return `${titre} ${description} ${motsCles}`;
}
