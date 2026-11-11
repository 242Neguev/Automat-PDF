// Pages du document en cours d'organisation : déplacements, rotations et sélection.

/** Une page du document : la page `page` (à partir de 0) de la source `source`, tournée de `rotation` degrés. */
export interface PageTravail {
  id: string;
  source: number;
  page: number;
  rotation: number;
}

/**
 * Déplace un groupe de pages (dans leur ordre actuel) à la place de `cible` : `actif` est la page
 * tirée à la souris ; les autres pages du groupe la suivent.
 */
export function deplacer<T extends { id: string }>(pages: T[], groupe: string[], actif: string, cible: string): T[] {
  if (actif === cible) return pages;
  const dansLeGroupe = new Set(groupe);
  const restantes = pages.filter((page) => !dansLeGroupe.has(page.id) || page.id === actif);
  const de = restantes.findIndex((page) => page.id === actif);
  const vers = restantes.findIndex((page) => page.id === cible);
  if (de < 0 || vers < 0) return pages;
  const deplacees = [...restantes];
  deplacees.splice(vers, 0, ...deplacees.splice(de, 1));
  const bloc = pages.filter((page) => dansLeGroupe.has(page.id));
  return deplacees.flatMap((page) => (page.id === actif ? bloc : [page]));
}

/** Tourne les pages choisies d'un quart de tour (1 : sens horaire, -1 : sens inverse). */
export function pivoter<T extends { id: string; rotation: number }>(pages: T[], ids: Set<string>, sens: 1 | -1): T[] {
  return pages.map((page) => (ids.has(page.id) ? { ...page, rotation: (page.rotation + 90 * sens + 360) % 360 } : page));
}

/** Côté le plus grand de l'aperçu en grand, en pixels. */
export const COTE_APERCU = 720;

/**
 * Côté du carré où s'inscrit l'aperçu en grand d'une fenêtre de `largeur` × `hauteur` pixels :
 * 720 au plus, moins sur un petit écran, pour que la fenêtre d'aperçu (titre, flèches, légende)
 * tienne entière, marges comprises (5 % de chaque côté).
 */
export function coteApercu(largeur: number, hauteur: number): number {
  return Math.max(200, Math.floor(Math.min(COTE_APERCU, hauteur * 0.9 - 120, largeur * 0.9 - 140)));
}

/** Pages comprises entre deux pages (incluses), dans l'ordre du document. */
export function plage(pages: PageTravail[], de: string, a: string): string[] {
  const debut = pages.findIndex((page) => page.id === de);
  const fin = pages.findIndex((page) => page.id === a);
  if (debut < 0 || fin < 0) return [a];
  return pages.slice(Math.min(debut, fin), Math.max(debut, fin) + 1).map((page) => page.id);
}
