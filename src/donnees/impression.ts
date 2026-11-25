// Fenêtre d'impression d'Automat-PDF (Windows) : où la page tombera sur la feuille, et combien de
// feuilles il faudra. Les mêmes règles que le cœur (impression.rs : a_tourner, place_sur_la_feuille).

import type { Ajustement, Feuille, Orientation } from "../coeur";

/** Millimètres par point (1/72 de pouce). */
const MM_PAR_POINT = 25.4 / 72;

/** Place de la page sur la feuille, en millimètres depuis son coin en haut à gauche. */
export interface PlacementApercu {
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
  /** Tournée d'un quart de tour (orientation automatique, page et feuille de sens différents). */
  tournee: boolean;
}

/**
 * Où une page (taille en points) tombera sur la feuille : dans la zone imprimable, tournée en
 * orientation automatique si elle n'a pas le sens de la feuille, à sa taille réelle réduite si
 * besoin (ou toujours réelle, ou ajustée), centrée.
 */
export function placementApercu(page: { largeur: number; hauteur: number }, feuille: Feuille, ajustement: Ajustement, orientation: Orientation): PlacementApercu {
  const [gauche, haut, droite, bas] = feuille.marges;
  const zone = { largeur: Math.max(1, feuille.largeur - gauche - droite), hauteur: Math.max(1, feuille.hauteur - haut - bas) };
  let largeur = page.largeur * MM_PAR_POINT;
  let hauteur = page.hauteur * MM_PAR_POINT;
  const tournee = orientation === "auto" && largeur !== hauteur && largeur > hauteur !== zone.largeur > zone.hauteur;
  if (tournee) [largeur, hauteur] = [hauteur, largeur];
  const remplir = Math.min(zone.largeur / largeur, zone.hauteur / hauteur);
  const echelle = ajustement === "reduire" ? Math.min(remplir, 1) : ajustement === "reelle" ? 1 : remplir;
  const l = largeur * echelle;
  const h = hauteur * echelle;
  return { x: gauche + (zone.largeur - l) / 2, y: haut + (zone.hauteur - h) / 2, largeur: l, hauteur: h, tournee };
}

/** Feuilles de papier nécessaires : en recto verso, deux pages par feuille, chaque copie assemblée sur ses propres feuilles. */
export function feuillesNecessaires(pages: number, copies: number, rectoVerso: boolean, assembler: boolean): number {
  const fois = Math.max(1, copies);
  if (!rectoVerso) return pages * fois;
  return assembler ? Math.ceil(pages / 2) * fois : Math.ceil((pages * fois) / 2);
}
