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
