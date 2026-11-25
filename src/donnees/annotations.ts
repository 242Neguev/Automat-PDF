// Annotations du lecteur (éditeur de PDF, première étape) : surlignages, notes, texte et images,
// en points de la page affichée depuis son coin en haut à gauche, comme les mots du cœur. Elles
// restent dans l'interface (annuler et rétablir faciles) jusqu'à l'enregistrement d'un nouveau PDF.

export type Teinte = "jaune" | "vert" | "rose" | "bleu" | "noir" | "rouge" | "marine";
export const TEINTES_SURLIGNEUR: Teinte[] = ["jaune", "vert", "rose", "bleu"];
export const TEINTES_TEXTE: Teinte[] = ["noir", "marine", "rouge"];

/** Les mêmes couleurs que le cœur (annotations.rs), pour l'aperçu. */
export const RVB: Record<Teinte, [number, number, number]> = {
  jaune: [255, 219, 31],
  vert: [107, 224, 115],
  rose: [255, 133, 194],
  bleu: [115, 194, 255],
  noir: [26, 28, 33],
  rouge: [199, 31, 31],
  marine: [26, 61, 140],
};

/** Interligne du texte écrit et place de sa ligne de base, en tailles de police (comme le cœur). */
export const INTERLIGNE = 1.2;
export const LIGNE_DE_BASE = (INTERLIGNE - 0.925) / 2 + 0.718;

export interface Point {
  x: number;
  y: number;
}

export interface Zone {
  x: number;
  y: number;
  largeur: number;
  hauteur: number;
}

/** Un mot placé sur la page (texte.rs) : seules sa place et sa ligne comptent ici. */
export interface MotPlace {
  x: number;
  y: number;
  longueur: number;
  taille: number;
  debutLigne: boolean;
}

export type Annotation =
  | { id: string; sorte: "surlignage"; page: number; teinte: Teinte; zones: Zone[] }
  | { id: string; sorte: "note"; page: number; x: number; y: number; texte: string }
  | { id: string; sorte: "texte"; page: number; x: number; y: number; texte: string; taille: number; teinte: Teinte }
  | { id: string; sorte: "image"; page: number; zone: Zone; chemin: string };

/** Ce que le cœur reçoit : les annotations sans leur identifiant, en laissant de côté les vides. */
export function pourLeCoeur(annotations: Annotation[]) {
  return annotations
    .filter((a) => (a.sorte === "texte" ? a.texte.trim() !== "" : a.sorte === "surlignage" ? a.zones.length > 0 : true))
    .map(({ id: _id, ...reste }) => reste);
}

let compteur = 0;
/** Identifiant d'une nouvelle annotation, unique dans la session. */
export const nouvelId = () => `annotation-${Date.now().toString(36)}-${(compteur++).toString(36)}`;

/** Annotations d'un document, avec ce qu'il faut pour annuler et rétablir. */
export interface Historique {
  passe: Annotation[][];
  present: Annotation[];
  futur: Annotation[][];
}

export const HISTORIQUE_VIDE: Historique = { passe: [], present: [], futur: [] };
/** Nombre d'étapes gardées pour « Annuler ». */
const PROFONDEUR = 100;

export function appliquer(historique: Historique, suivant: Annotation[]): Historique {
  if (suivant === historique.present) return historique;
  return { passe: [...historique.passe, historique.present].slice(-PROFONDEUR), present: suivant, futur: [] };
}

export function annuler(historique: Historique): Historique {
  if (historique.passe.length === 0) return historique;
  return {
    passe: historique.passe.slice(0, -1),
    present: historique.passe[historique.passe.length - 1],
    futur: [historique.present, ...historique.futur],
  };
}

export function retablir(historique: Historique): Historique {
  if (historique.futur.length === 0) return historique;
  return { passe: [...historique.passe, historique.present], present: historique.futur[0], futur: historique.futur.slice(1) };
}

/** Rectangle entre deux points, quel que soit le sens du geste. */
export function zoneEntre(a: Point, b: Point): Zone {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), largeur: Math.abs(a.x - b.x), hauteur: Math.abs(a.y - b.y) };
}

/** Distance d'un point à la boîte d'un mot (0 dedans). */
function distance(point: Point, mot: MotPlace): number {
  const dx = Math.max(mot.x - point.x, 0, point.x - (mot.x + mot.longueur));
  const dy = Math.max(mot.y - point.y, 0, point.y - (mot.y + mot.taille));
  return Math.hypot(dx, dy);
}

/** Rang du mot le plus proche du point, s'il est à moins de `portee` points. */
export function motProche(mots: MotPlace[], point: Point, portee: number): number | null {
  let meilleur: number | null = null;
  let plusPetite = portee;
  mots.forEach((mot, rang) => {
    const d = distance(point, mot);
    if (d <= plusPetite) {
      plusPetite = d;
      meilleur = rang;
    }
  });
  return meilleur;
}

/**
 * Zones à surligner quand on glisse de `debut` à `fin` : si le geste part d'un mot, les mots
 * parcourus dans l'ordre de lecture, une zone par ligne ; sinon le cadre tracé (page numérisée,
 * dessin, tableau).
 */
export function zonesSurlignees(mots: MotPlace[], debut: Point, fin: Point, portee = 12): Zone[] {
  const premier = motProche(mots, debut, portee);
  if (premier === null) {
    const cadre = zoneEntre(debut, fin);
    return cadre.largeur > 2 && cadre.hauteur > 2 ? [cadre] : [];
  }
  const dernier = motProche(mots, fin, Number.POSITIVE_INFINITY) ?? premier;
  const [a, b] = premier <= dernier ? [premier, dernier] : [dernier, premier];
  const zones: Zone[] = [];
  let ligne: MotPlace[] = [];
  const fermer = () => {
    if (ligne.length === 0) return;
    const gauche = Math.min(...ligne.map((m) => m.x));
    const droite = Math.max(...ligne.map((m) => m.x + m.longueur));
    const haut = Math.min(...ligne.map((m) => m.y));
    const bas = Math.max(...ligne.map((m) => m.y + m.taille));
    zones.push({ x: gauche, y: haut, largeur: droite - gauche, hauteur: bas - haut });
    ligne = [];
  };
  for (let rang = a; rang <= b; rang++) {
    if (rang > a && mots[rang].debutLigne) fermer();
    ligne.push(mots[rang]);
  }
  fermer();
  return zones;
}

/** Nom proposé pour le PDF annoté : « Contrat.pdf » → « Contrat - annoté.pdf ». */
export function nomAnnote(nom: string): string {
  const base = nom.replace(/\.pdf$/i, "");
  return `${base} - annoté.pdf`;
}

/** Les annotations sans celles effacées d'un geste de gomme (une seule étape à annuler). */
export function sans(annotations: Annotation[], effacees: Iterable<string>): Annotation[] {
  const ids = new Set(effacees);
  return ids.size === 0 ? annotations : annotations.filter((a) => !ids.has(a.id));
}
