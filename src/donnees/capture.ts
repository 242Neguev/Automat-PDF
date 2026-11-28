// Capturer une zone d'une page (lecteur) : la zone tracée, en points de la page affichée, sa taille,
// son texte, et sa forme pour le cœur (fractions de la page).
import type { FormatPage, Mot, ZoneFractions } from "../coeur";

/** Zone tracée sur une page, en points de la page affichée, depuis son coin en haut à gauche. */
export interface ZonePoints {
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
}

interface Point {
  x: number;
  y: number;
}

const borne = (valeur: number, max: number) => Math.min(max, Math.max(0, valeur));

/** La zone entre deux points tracés à la souris, gardée dans la page. */
export function zoneEntre(a: Point, b: Point, format: FormatPage): ZonePoints {
  const [x0, x1] = [borne(Math.min(a.x, b.x), format.largeur), borne(Math.max(a.x, b.x), format.largeur)];
  const [y0, y1] = [borne(Math.min(a.y, b.y), format.hauteur), borne(Math.max(a.y, b.y), format.hauteur)];
  return { x: x0, y: y0, largeur: x1 - x0, hauteur: y1 - y0 };
}

/** Un vrai cadre, pas un simple clic : au moins 4 points (1,4 mm) de côté. */
export const assezGrande = (zone: ZonePoints) => zone.largeur >= 4 && zone.hauteur >= 4;

/** La zone en fractions de la page, comme le cœur la prend. */
export function enFractions(zone: ZonePoints, format: FormatPage): ZoneFractions {
  return { x: zone.x / format.largeur, y: zone.y / format.hauteur, largeur: zone.largeur / format.largeur, hauteur: zone.hauteur / format.hauteur };
}

/** « 8,2 × 5,1 cm » */
export function tailleEnCm(zone: ZonePoints): string {
  const cm = (points: number) => ((points / 72) * 2.54).toFixed(1).replace(".", ",");
  return `${cm(zone.largeur)} × ${cm(zone.hauteur)} cm`;
}

/**
 * Centre d'un mot sur la page : sa boîte part de (x, y) et tourne autour de ce coin d'un quart de
 * tour par unité de `sens` (sens des aiguilles d'une montre), comme dans la couche de texte.
 */
function centreDe(mot: Mot): Point {
  const [l, t] = [mot.longueur / 2, mot.taille / 2];
  switch (mot.sens % 4) {
    case 1:
      return { x: mot.x - t, y: mot.y + l };
    case 2:
      return { x: mot.x - l, y: mot.y - t };
    case 3:
      return { x: mot.x + t, y: mot.y - l };
    default:
      return { x: mot.x + l, y: mot.y + t };
  }
}

/**
 * Le texte des mots dont le centre est dans la zone, dans l'ordre de lecture : un saut de ligne entre
 * deux lignes, une espace entre deux mots d'une même ligne.
 */
export function texteDansZone(mots: Mot[], zone: ZonePoints): { texte: string; mots: number } {
  const morceaux: string[] = [];
  let nombre = 0;
  let nouvelleLigne = false;
  for (const mot of mots) {
    if (mot.debutLigne) nouvelleLigne = true;
    const centre = centreDe(mot);
    if (centre.x < zone.x || centre.x > zone.x + zone.largeur || centre.y < zone.y || centre.y > zone.y + zone.hauteur) continue;
    if (nombre > 0) morceaux.push(nouvelleLigne ? "\n" : " ");
    morceaux.push(mot.texte);
    nombre++;
    nouvelleLigne = false;
  }
  return { texte: morceaux.join(""), mots: nombre };
}
