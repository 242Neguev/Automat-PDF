// Recherche d'une tâche avec les mots de tous les jours : « signer », « alléger », « réunir »…

/** Mots d'un texte, sans accents ni majuscules, coupés aux espaces et à la ponctuation. */
export function mots(texte: string): string[] {
  return texte
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);
}

/** Petits mots qui ne disent rien de la tâche cherchée (« je veux mettre un mot de passe »). */
const VIDES = new Set(["a", "au", "aux", "avec", "ce", "ces", "comment", "d", "dans", "de", "des", "du", "en", "et", "faire", "je", "j", "l", "la", "le", "les", "ma", "mes", "mon", "par", "pour", "sur", "un", "une", "veux", "voudrais"]);

/**
 * Ce que vaut un mot cherché face à un mot de l'outil : 2 s'il est le même, 1 s'il en est le début
 * (« sign » : « signer ») ou s'il en est un pluriel, 0 sinon.
 */
function valeur(cherche: string, mot: string): number {
  if (mot === cherche) return 2;
  return mot.startsWith(cherche) || (mot.length >= 4 && cherche.startsWith(mot)) ? 1 : 0;
}

/**
 * Éléments qui correspondent le mieux à la recherche, dans leur ordre ; `null` quand il n'y a rien
 * à chercher. Chaque élément est décrit par son texte (titre, description, mots-clés).
 */
export function chercher<T>(recherche: string, elements: T[], texte: (element: T) => string): T[] | null {
  const cherches = mots(recherche).filter((mot) => !VIDES.has(mot));
  if (cherches.length === 0) return null;
  const scores = elements.map((element) => {
    const siens = mots(texte(element));
    return cherches.reduce((score, cherche) => score + Math.max(0, ...siens.map((mot) => valeur(cherche, mot))), 0);
  });
  const meilleur = Math.max(0, ...scores);
  return meilleur === 0 ? [] : elements.filter((_, i) => scores[i] === meilleur);
}
