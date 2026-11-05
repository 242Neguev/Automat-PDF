// Valeurs de la liste placées dans un texte à balises ({{Prénom}} {{NOM}}) : noms de fichiers…
import type { Correspondances } from "./correspondances";
import type { LigneListe } from "./liste";
import { nettoyer, type Transformation } from "./nettoyage";

const BALISE = /\{\{([^}]+)\}\}/g;

/** Balises d'un texte, dans l'ordre d'apparition et sans doublon. */
export const balisesDe = (texte: string) => [...new Set([...texte.matchAll(BALISE)].map((m) => m[1].trim()))];

/**
 * Remplace les balises d'un texte. Si toutes ses balises sont vides, le texte entier est vide :
 * le fichier est alors numéroté.
 */
export function remplir(texte: string, valeurs: Record<string, string>): string {
  const balises = balisesDe(texte);
  if (balises.length > 0 && balises.every((balise) => !valeurs[balise])) return "";
  return texte
    .replace(BALISE, (_, balise: string) => valeurs[balise.trim()] ?? "")
    .replace(/\s+/g, " ")
    .trim();
}

/** Remplace les balises d'un texte de plusieurs lignes (message), sans toucher à sa mise en page. */
export const remplirTexte = (texte: string, valeurs: Record<string, string>) => texte.replace(BALISE, (_, balise: string) => valeurs[balise.trim()] ?? "");

/** Valeur nettoyée de chaque balise pour une ligne de la liste. */
export function valeursDeLigne(modele: { transformations?: Record<string, Transformation> }, ligne: LigneListe, correspondances: Correspondances) {
  return Object.fromEntries(
    Object.entries(correspondances).map(([balise, colonne]) => [balise, nettoyer(colonne ? ligne.valeurs[colonne] : "", modele.transformations?.[balise])]),
  );
}

/**
 * Mises en forme courantes, déduites du nom des balises : une balise écrite en capitales (« NOM »)
 * passe en MAJUSCULES, un prénom en Nom Propre, un e-mail en minuscules.
 */
export function transformationsParDefaut(balises: string[]): Record<string, Transformation> {
  const resultat: Record<string, Transformation> = {};
  for (const balise of balises) {
    const simple = balise.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
    if (/^(e-?mail|mail|courriel)/.test(simple)) resultat[balise] = "minuscules";
    else if (/^prenom/.test(simple)) resultat[balise] = "nom-propre";
    else if (balise.length > 1 && balise === balise.toLocaleUpperCase("fr") && /\p{L}/u.test(balise)) resultat[balise] = "majuscules";
  }
  return resultat;
}
