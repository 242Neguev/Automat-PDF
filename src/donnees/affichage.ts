// Affichage du lecteur (0.6.0) : les pages une par une ou deux côte à côte, et la rotation de
// l'affichage, d'un quart de tour à la fois (le fichier n'est pas modifié, comme dans Acrobat).
// Une page tournée garde ses coordonnées : elle est dessinée droite, puis tournée dans sa boîte.

export type Rotation = 0 | 90 | 180 | 270;
export type Disposition = "une" | "deux";

interface Taille {
  largeur: number;
  hauteur: number;
}

/** La rotation suivante, d'un quart de tour dans le sens des aiguilles d'une montre (ou à l'inverse). */
export function tourner(rotation: Rotation, sens: 1 | -1 = 1): Rotation {
  return ((((rotation + sens * 90) % 360) + 360) % 360) as Rotation;
}

/** Largeur et hauteur de la page telle qu'elle s'affiche : échangées par un quart de tour. */
export function tailleAffichee(format: Taille, rotation: Rotation): Taille {
  return rotation % 180 === 0 ? { largeur: format.largeur, hauteur: format.hauteur } : { largeur: format.hauteur, hauteur: format.largeur };
}

/**
 * Transformation CSS (origine en haut à gauche) qui tourne la page droite, de `l` × `h` pixels,
 * dans sa boîte affichée.
 */
export function transformation(rotation: Rotation, l: number, h: number): string | undefined {
  switch (rotation) {
    case 90:
      return `translate(${h}px, 0px) rotate(90deg)`;
    case 180:
      return `translate(${l}px, ${h}px) rotate(180deg)`;
    case 270:
      return `translate(0px, ${l}px) rotate(270deg)`;
    default:
      return undefined;
  }
}

/** Le point de la page droite (pixels) qui s'affiche en (vx, vy) dans sa boîte ; `l` × `h` : la page droite. */
export function versLaPage(vx: number, vy: number, l: number, h: number, rotation: Rotation): { x: number; y: number } {
  switch (rotation) {
    case 90:
      return { x: vy, y: h - vx };
    case 180:
      return { x: l - vx, y: h - vy };
    case 270:
      return { x: l - vy, y: vx };
    default:
      return { x: vx, y: vy };
  }
}

/** Distance du haut de la page affichée au point (x, y) de la page, en points : pour y faire défiler. */
export function hautAffiche(format: Taille, x: number, y: number, rotation: Rotation): number {
  switch (rotation) {
    case 90:
      return x;
    case 180:
      return format.hauteur - y;
    case 270:
      return format.largeur - x;
    default:
      return y;
  }
}

/** Les pages rangées pour l'affichage : une par rangée, ou deux côte à côte (1-2, 3-4…). */
export function rangees(nombre: number, disposition: Disposition): number[][] {
  const pas = disposition === "deux" ? 2 : 1;
  return Array.from({ length: Math.ceil(nombre / pas) }, (_, rangee) => Array.from({ length: Math.min(pas, nombre - rangee * pas) }, (_, i) => rangee * pas + i));
}

/** La première page de la rangée de cette page : celle que le lecteur annonce. */
export const premiereDeRangee = (page: number, disposition: Disposition) => (disposition === "deux" ? page - (page % 2) : page);

/**
 * Échelle (pixels par point) qui fait tenir la plus large rangée de pages dans la largeur de la
 * zone et, pour « page entière », la plus haute page dans sa hauteur ; `marge` : autour des pages
 * et entre deux pages côte à côte.
 */
export function echelleAjustee(formats: Taille[], rotation: Rotation, disposition: Disposition, zone: Taille, mode: "largeur" | "page", marge: number): number {
  const tailles = formats.map((format) => tailleAffichee(format, rotation));
  const lignes = rangees(formats.length, disposition);
  const largeurMax = Math.max(...lignes.map((ligne) => ligne.reduce((somme, page) => somme + tailles[page].largeur, 0)));
  const ecart = lignes.some((ligne) => ligne.length > 1) ? marge : 0;
  const hauteurMax = Math.max(...tailles.map((taille) => taille.hauteur));
  const parLargeur = (zone.largeur - 2 * marge - ecart) / largeurMax;
  const parHauteur = (zone.hauteur - 2 * marge) / hauteurMax;
  return mode === "page" ? Math.min(parLargeur, parHauteur) : parLargeur;
}
