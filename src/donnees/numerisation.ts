// Numériser depuis le scanner : le nom proposé pour le PDF, et les réglages gardés d'une fois sur
// l'autre (stockage de l'application, sur l'ordinateur seulement).
import type { CouleurNumerisation, SourceNumerisation } from "../coeur";

export interface ReglagesGardes {
  scanner: string | null;
  source: SourceNumerisation;
  rectoVerso: boolean;
  couleur: CouleurNumerisation;
  resolution: number;
  reconnaitre: boolean;
}

export const REGLAGES_PAR_DEFAUT: ReglagesGardes = {
  scanner: null,
  source: "vitre",
  rectoVerso: false,
  couleur: "couleur",
  resolution: 300,
  reconnaitre: true,
};

const CLE = "automat-pdf.numerisation";
const MOIS = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];

/** « Numérisation du 26 novembre 2026 à 17 h 05.pdf » */
export function nomNumerisation(date: Date): string {
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `Numérisation du ${date.getDate()} ${MOIS[date.getMonth()]} ${date.getFullYear()} à ${date.getHours()} h ${minutes}.pdf`;
}

function stockageParDefaut(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

/** Les réglages gardés, complétés par ceux par défaut ; une valeur abîmée est ignorée. */
export function lireReglages(stockage: Pick<Storage, "getItem"> | null = stockageParDefaut()): ReglagesGardes {
  try {
    const lus: unknown = JSON.parse(stockage?.getItem(CLE) ?? "{}");
    if (typeof lus !== "object" || lus === null) return REGLAGES_PAR_DEFAUT;
    const r = lus as Partial<Record<keyof ReglagesGardes, unknown>>;
    return {
      scanner: typeof r.scanner === "string" ? r.scanner : REGLAGES_PAR_DEFAUT.scanner,
      source: r.source === "vitre" || r.source === "chargeur" ? r.source : REGLAGES_PAR_DEFAUT.source,
      rectoVerso: typeof r.rectoVerso === "boolean" ? r.rectoVerso : REGLAGES_PAR_DEFAUT.rectoVerso,
      couleur: r.couleur === "couleur" || r.couleur === "gris" || r.couleur === "noir-et-blanc" ? r.couleur : REGLAGES_PAR_DEFAUT.couleur,
      resolution: r.resolution === 150 || r.resolution === 300 || r.resolution === 600 ? r.resolution : REGLAGES_PAR_DEFAUT.resolution,
      reconnaitre: typeof r.reconnaitre === "boolean" ? r.reconnaitre : REGLAGES_PAR_DEFAUT.reconnaitre,
    };
  } catch {
    return REGLAGES_PAR_DEFAUT;
  }
}

/** Garde les réglages ; si le stockage est plein ou refusé, ils ne valent que jusqu'à la fermeture. */
export function ecrireReglages(reglages: ReglagesGardes, stockage: Pick<Storage, "setItem"> | null = stockageParDefaut()) {
  try {
    stockage?.setItem(CLE, JSON.stringify(reglages));
  } catch {
    // rien de plus à faire
  }
}
