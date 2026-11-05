// Fichiers reçus de l'Explorateur (clic droit, glisser-déposer) : chaque outil dit ce qu'il en fait.
import { useEffect, useRef } from "react";

export type SorteFichier = "pdf" | "image" | "office" | "liste";

const EXTENSIONS: Record<SorteFichier, string[]> = {
  pdf: ["pdf"],
  image: ["jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "gif"],
  office: ["doc", "docx", "docm", "dot", "dotx", "rtf", "odt", "xls", "xlsx", "xlsm", "xlsb", "ods", "ppt", "pptx", "pptm", "pps", "ppsx", "odp"],
  liste: ["xlsx", "xlsm", "xls", "ods", "csv", "tsv", "txt"],
};

/** Ce qu'est un fichier, d'après son extension ; un classeur Excel est à la fois document Office et liste. */
export function sortesDe(chemin: string): SorteFichier[] {
  const extension = chemin.split(".").pop()?.toLowerCase() ?? "";
  return (Object.keys(EXTENSIONS) as SorteFichier[]).filter((sorte) => EXTENSIONS[sorte].includes(extension));
}

/** Les fichiers d'une sorte, parmi ceux reçus. */
export const dela = (fichiers: string[], sorte: SorteFichier) => fichiers.filter((fichier) => sortesDe(fichier).includes(sorte));

const gestionnaires = new Map<string, (fichiers: string[]) => void>();

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
  gestionnaire?.(fichiers);
  return gestionnaire !== undefined;
}
