// Les outils de l'application, rangés par groupes : menu de gauche, accueil, fichiers reçus.
import type { ComponentType } from "react";
import {
  IconArrowsMinimize,
  IconEyeOff,
  IconFiles,
  IconFileTypeDocx,
  IconFileTypePdf,
  IconLayersIntersect,
  IconLayoutColumns,
  IconLayoutGrid,
  IconLock,
  IconNumbers,
  IconPhoto,
  IconPhotoPlus,
  IconScissors,
  IconSignature,
  IconUsers,
} from "@tabler/icons-react";
import Caviarder from "./pages/annoter/Caviarder";
import NumerosTampon from "./pages/annoter/NumerosTampon";
import SignatureCachet from "./pages/annoter/SignatureCachet";
import type { SorteFichier } from "./pages/commun/reception";
import OfficeEnPdf from "./pages/convertir/OfficeEnPdf";
import PdfEnWord from "./pages/convertir/PdfEnWord";
import FusionPdf from "./pages/fusion/FusionPdf";
import ImagesEnPdf from "./pages/images/ImagesEnPdf";
import PdfEnImages from "./pages/images/PdfEnImages";
import IntercalerPdf from "./pages/intercaler/IntercalerPdf";
import MiseEnPage from "./pages/mise-en-page/MiseEnPage";
import Compresser from "./pages/optimiser/Compresser";
import OrganiserPdf from "./pages/organiser/OrganiserPdf";
import MotDePasse from "./pages/securite/MotDePasse";
import SeparationParPersonne from "./pages/separation/SeparationParPersonne";
import SeparationPdf from "./pages/separation/SeparationPdf";
import { textes } from "./textes";

export type Outil = keyof typeof textes.outils;

export interface DefinitionOutil {
  cle: Outil;
  icone: typeof IconFiles;
  page: ComponentType;
  /** Fichiers qu'il prend de l'Explorateur (clic droit, glisser-déposer). */
  accepte: SorteFichier[];
}

export interface Groupe {
  cle: keyof typeof textes.groupes;
  outils: DefinitionOutil[];
}

export const GROUPES: Groupe[] = [
  {
    cle: "pages",
    outils: [
      { cle: "organiser", icone: IconLayoutGrid, page: OrganiserPdf, accepte: ["pdf"] },
      { cle: "miseEnPage", icone: IconLayoutColumns, page: MiseEnPage, accepte: ["pdf"] },
    ],
  },
  {
    cle: "assembler",
    outils: [
      { cle: "fusionner", icone: IconFiles, page: FusionPdf, accepte: ["pdf"] },
      { cle: "intercaler", icone: IconLayersIntersect, page: IntercalerPdf, accepte: ["pdf"] },
      { cle: "images", icone: IconPhotoPlus, page: ImagesEnPdf, accepte: ["image"] },
    ],
  },
  {
    cle: "decouper",
    outils: [
      { cle: "separer", icone: IconScissors, page: SeparationPdf, accepte: ["pdf", "liste"] },
      { cle: "parPersonne", icone: IconUsers, page: SeparationParPersonne, accepte: ["pdf", "liste"] },
    ],
  },
  {
    cle: "annoter",
    outils: [
      { cle: "marques", icone: IconNumbers, page: NumerosTampon, accepte: ["pdf"] },
      { cle: "signature", icone: IconSignature, page: SignatureCachet, accepte: ["pdf", "image"] },
    ],
  },
  {
    cle: "securite",
    outils: [
      { cle: "motDePasse", icone: IconLock, page: MotDePasse, accepte: ["pdf"] },
      { cle: "caviarder", icone: IconEyeOff, page: Caviarder, accepte: ["pdf"] },
    ],
  },
  { cle: "optimiser", outils: [{ cle: "compresser", icone: IconArrowsMinimize, page: Compresser, accepte: ["pdf"] }] },
  {
    cle: "convertir",
    outils: [
      { cle: "enImages", icone: IconPhoto, page: PdfEnImages, accepte: ["pdf"] },
      { cle: "officeEnPdf", icone: IconFileTypePdf, page: OfficeEnPdf, accepte: ["office"] },
      { cle: "pdfEnWord", icone: IconFileTypeDocx, page: PdfEnWord, accepte: ["pdf"] },
    ],
  },
];

export const OUTILS = GROUPES.flatMap((groupe) => groupe.outils);

/** Ce qui décrit un outil pour la recherche : son titre, sa description et ses mots-clés. */
export function texteDe(definition: DefinitionOutil): string {
  const { titre, description, motsCles } = textes.outils[definition.cle];
  return `${titre} ${description} ${motsCles}`;
}
