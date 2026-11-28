// Fichiers reçus du système (clic droit dans l'Explorateur ou le Finder, glisser-déposer) : chaque outil dit ce qu'il en fait.
import { useEffect, useRef } from "react";

export type SorteFichier = "pdf" | "image" | "office" | "liste";

const EXTENSIONS: Record<SorteFichier, string[]> = {
  pdf: ["pdf"],
  image: ["jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "gif"],
  office: ["doc", "docx", "docm", "dot", "dotx", "rtf", "odt", "xls", "xlsx", "xlsm", "xlsb", "ods", "ppt", "pptx", "pptm", "pps", "ppsx", "odp"],
  liste: ["xlsx", "xlsm", "xls", "ods", "csv", "tsv", "txt"],
};

/** Toutes les extensions qu'un outil au moins accepte. */
export const TOUTES_EXTENSIONS = [...new Set(Object.values(EXTENSIONS).flat())];

/** Ce qu'est un fichier, d'après son extension ; un classeur Excel est à la fois document Office et liste. */
export function sortesDe(chemin: string): SorteFichier[] {
  const extension = chemin.split(".").pop()?.toLowerCase() ?? "";
  return (Object.keys(EXTENSIONS) as SorteFichier[]).filter((sorte) => EXTENSIONS[sorte].includes(extension));
}

/** Les fichiers d'une sorte, parmi ceux reçus. */
export const dela = (fichiers: string[], sorte: SorteFichier) => fichiers.filter((fichier) => sortesDe(fichier).includes(sorte));

const gestionnaires = new Map<string, (fichiers: string[]) => void>();
/** Pages confiées par le lecteur avec leur PDF (voir `confierPage`). */
const pagesConfiees = new Map<string, number>();

/** L'outil `outil` reçoit les fichiers qui lui sont confiés (déjà triés : il les accepte). */
export function useFichiersRecus(outil: string, gestionnaire: (fichiers: string[]) => void) {
  const actuel = useRef(gestionnaire);
  actuel.current = gestionnaire;
  useEffect(() => {
    const appel = (fichiers: string[]) => actuel.current(fichiers);
    gestionnaires.set(outil, appel);
    return () => {
      if (gestionnaires.get(outil) === appel) gestionnaires.delete(outil);
    };
  }, [outil]);
}

/** Confie des fichiers à un outil ; `false` s'il ne sait pas les recevoir. */
export function transmettre(outil: string, fichiers: string[]): boolean {
  const gestionnaire = gestionnaires.get(outil);
  try {
    gestionnaire?.(fichiers);
  } finally {
    // Une page confiée ne vaut que pour cette remise.
    pagesConfiees.clear();
  }
  return gestionnaire !== undefined;
}

/**
 * La page (à partir de 0) que le lecteur montrait de ce PDF quand il l'a confié à un outil : un
 * outil qui choisit des pages peut partir de celle-là. Elle ne vaut que pour la remise qui suit.
 */
export function confierPage(chemin: string, page: number) {
  pagesConfiees.set(chemin, page);
}

/** La page confiée avec ce PDF (à lire dès sa réception, avant tout `await`). */
export const pageConfiee = (chemin: string): number | undefined => pagesConfiees.get(chemin);

/** Événement : un outil en fait ouvrir un autre, avec des fichiers (l'application le reçoit). */
export const OUTIL_DEMANDE = "automat-pdf:outil-demande";

export interface DemandeOutil {
  outil: string;
  fichiers: string[];
}

/** Ouvre l'outil `outil` avec ces fichiers, comme le fait le lecteur. */
export function demanderOutil(outil: string, fichiers: string[]) {
  window.dispatchEvent(new CustomEvent<DemandeOutil>(OUTIL_DEMANDE, { detail: { outil, fichiers } }));
}
