// Textes des PDF ouverts dans le lecteur, demandés au cœur une seule fois : le texte de chaque page (la
// recherche) et les mots placés d'une page (la sélection, la copie et les surlignages).
import { motsPage, textesPdf, type Mot } from "../../coeur";

const textes = new Map<string, Promise<string[]>>();
const mots = new Map<string, Promise<Mot[]>>();

/** Le texte de chaque page du PDF. */
export function textesDe(chemin: string): Promise<string[]> {
  let demande = textes.get(chemin);
  if (!demande) {
    demande = textesPdf(chemin);
    textes.set(chemin, demande);
    demande.catch(() => textes.delete(chemin));
  }
  return demande;
}

/** Oublie le texte et les mots d'un PDF qui a changé en mémoire (texte reconnu) : ils seront redemandés. */
export function oublierTextes(chemin: string) {
  textes.delete(chemin);
  for (const cle of [...mots.keys()]) if (cle.startsWith(`${chemin}\u0000`)) mots.delete(cle);
}

/** Les mots de la page `page` (à partir de 0), avec leur place. */
export function motsDe(chemin: string, page: number): Promise<Mot[]> {
  const cle = `${chemin}\u0000${page}`;
  let demande = mots.get(cle);
  if (!demande) {
    demande = motsPage(chemin, page);
    mots.set(cle, demande);
    demande.catch(() => mots.delete(cle));
  }
  return demande;
}
