// Avancement d'un travail sur une liste de fichiers, annoncé par le cœur (événement
// « progression-conversion »).

/**
 * Fichiers faits et fichiers, puis, pour un travail page par page (reconnaissance du texte), les
 * pages lues du fichier en cours et ses pages à lire.
 */
export type Avancement = [number, number] | [number, number, number, number];

/** La part faite, de 0 à 1 : les fichiers finis, plus la part du fichier en cours. */
export function partFaite([fait, total, page = 0, pages = 0]: Avancement): number {
  const enCours = pages > 0 ? Math.min(1, page / pages) : 0;
  return Math.min(1, (fait + enCours) / Math.max(1, total));
}
