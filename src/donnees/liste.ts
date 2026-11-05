// Lecture des listes (Excel, CSV, ODS). Les valeurs sont gardées telles qu'Excel les affiche
// (zéros en tête, indicatifs, dates mises en forme) : elles ne sont jamais recalculées.
import * as XLSX from "xlsx";

export interface LigneListe {
  /** Numéro de la ligne dans le fichier d'origine (1 = première ligne), repris dans les messages. */
  numero: number;
  valeurs: Record<string, string>;
}

export interface Liste {
  nomFichier: string;
  feuille: string;
  colonnes: string[];
  lignes: LigneListe[];
}

const FORMATS_TEXTE = /\.(csv|tsv|txt)$/i;

/** Décode un fichier texte : UTF-8 si possible, sinon Windows-1252 (CSV enregistrés par Excel en français). */
export function decoderTexte(octets: Uint8Array): string {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(octets).replace(/^﻿/, "");
  } catch {
    return new TextDecoder("windows-1252").decode(octets);
  }
}

export function ouvrirClasseur(octets: ArrayBuffer, nomFichier: string): XLSX.WorkBook {
  if (FORMATS_TEXTE.test(nomFichier)) {
    // raw : le texte d'un CSV est gardé tel quel (« 0612… » ne devient pas un nombre).
    return XLSX.read(decoderTexte(new Uint8Array(octets)), { type: "string", raw: true });
  }
  return XLSX.read(octets, { type: "array" });
}

const estRemplie = (cellule: unknown) => String(cellule ?? "").trim() !== "";

/** Noms de colonnes uniques ; une en-tête vide devient « Colonne N ». */
function nommerColonnes(entetes: unknown[]): string[] {
  const vus = new Map<string, number>();
  return entetes.map((brut, index) => {
    const base = String(brut ?? "").replace(/\s+/g, " ").trim() || `Colonne ${index + 1}`;
    const rang = (vus.get(base) ?? 0) + 1;
    vus.set(base, rang);
    return rang === 1 ? base : `${base} (${rang})`;
  });
}

/**
 * Lit une feuille. L'en-tête est la première ligne qui a au moins deux cellules remplies,
 * ce qui saute un éventuel titre (« Liste des agents 2026 ») placé au-dessus du tableau.
 */
export function lireFeuille(classeur: XLSX.WorkBook, nomFichier: string, feuille: string): Liste {
  const onglet = classeur.Sheets[feuille];
  const vide: Liste = { nomFichier, feuille, colonnes: [], lignes: [] };
  if (!onglet?.["!ref"]) return vide;
  const premiereLigne = XLSX.utils.decode_range(onglet["!ref"]).s.r;
  const tableau = XLSX.utils.sheet_to_json<unknown[]>(onglet, { header: 1, raw: false, defval: "", blankrows: true });

  let indexEntete = tableau.findIndex((ligne) => ligne.filter(estRemplie).length >= 2);
  if (indexEntete < 0) indexEntete = tableau.findIndex((ligne) => ligne.some(estRemplie));
  if (indexEntete < 0) return vide;

  const donnees = tableau.slice(indexEntete + 1).map((ligne, i) => ({ ligne, numero: premiereLigne + indexEntete + i + 2 }));
  const nombreColonnes = Math.max(tableau[indexEntete].length, ...donnees.map(({ ligne }) => ligne.length));
  const entetes = Array.from({ length: nombreColonnes }, (_, i) => tableau[indexEntete][i] ?? "");
  // Une colonne sans en-tête ni valeur (mise en forme résiduelle d'Excel) est ignorée.
  const utiles = entetes.map((entete, i) => estRemplie(entete) || donnees.some(({ ligne }) => estRemplie(ligne[i])));
  const noms = nommerColonnes(entetes);
  const colonnes = noms.filter((_, i) => utiles[i]);

  const lignes = donnees
    .filter(({ ligne }) => ligne.some(estRemplie))
    .map(({ ligne, numero }) => ({
      numero,
      valeurs: Object.fromEntries(noms.flatMap((nom, i) => (utiles[i] ? [[nom, String(ligne[i] ?? "")]] : []))),
    }));
  return { nomFichier, feuille, colonnes, lignes };
}
