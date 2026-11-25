// Annotations du lecteur (éditeur de PDF, première étape) : surlignages, notes, texte et images,
// en points de la page affichée depuis son coin en haut à gauche, comme les mots du cœur. Elles
// restent dans l'interface (annuler et rétablir faciles) jusqu'à l'enregistrement d'un nouveau PDF.

export type Teinte = "jaune" | "vert" | "rose" | "bleu" | "noir" | "rouge" | "marine";
export const TEINTES_SURLIGNEUR: Teinte[] = ["jaune", "vert", "rose", "bleu"];
export const TEINTES_TEXTE: Teinte[] = ["noir", "marine", "rouge"];
/** Couleurs des traits : soulignement, rature, formes, dessin. */
export const TEINTES_TRAIT: Teinte[] = ["rouge", "marine", "noir"];

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

export type Forme = "rectangle" | "ellipse" | "ligne" | "fleche";

export type MotTampon = "approuve" | "paye" | "recu" | "vu" | "copie" | "confidentiel" | "annule" | "urgent";

/** Les tampons prêts : leur mot et leur encre, les mêmes que le cœur (annotations.rs, MotTampon). */
export const TAMPONS: Record<MotTampon, { texte: string; rvb: [number, number, number] }> = {
  approuve: { texte: "APPROUVÉ", rvb: [23, 122, 64] },
  paye: { texte: "PAYÉ", rvb: [26, 61, 140] },
  recu: { texte: "REÇU", rvb: [26, 61, 140] },
  vu: { texte: "VU", rvb: [26, 61, 140] },
  copie: { texte: "COPIE", rvb: [84, 89, 97] },
  confidentiel: { texte: "CONFIDENTIEL", rvb: [199, 31, 31] },
  annule: { texte: "ANNULÉ", rvb: [199, 31, 31] },
  urgent: { texte: "URGENT", rvb: [199, 31, 31] },
};

/** Longueur des branches d'une flèche, selon l'épaisseur du trait (comme le cœur). */
export const branche = (epaisseur: number) => Math.min(epaisseur * 3 + 6, 40);

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

/** Un mot avec son texte, sa police et sa couleur : de quoi le corriger. */
export interface MotComplet extends MotPlace {
  texte: string;
  sens: number;
  famille: string;
  gras: boolean;
  italique: boolean;
  couleur: [number, number, number];
}

export type Annotation =
  /** `extrait` : le texte surligné, gardé dans le commentaire du PDF (absent pour un cadre tracé). */
  | { id: string; sorte: "surlignage"; page: number; teinte: Teinte; zones: Zone[]; extrait?: string }
  | { id: string; sorte: "note"; page: number; x: number; y: number; texte: string }
  | { id: string; sorte: "texte"; page: number; x: number; y: number; texte: string; taille: number; teinte: Teinte }
  | { id: string; sorte: "image"; page: number; zone: Zone; chemin: string }
  | {
      id: string;
      sorte: "correction";
      page: number;
      /** L'ancien texte : sa boîte, ses bords gauche et droit, sa ligne de base. */
      zone: Zone;
      x: number;
      fin: number;
      ligne: number;
      /** Pour l'aperçu et pour revenir en arrière (pas envoyé au cœur). */
      ancien: string;
      texte: string;
      taille: number;
      famille: string;
      gras: boolean;
      italique: boolean;
      couleur: [number, number, number];
      /** Garder le bord droit (montant dans une colonne). */
      droite: boolean;
    }
  | { id: string; sorte: "blanc"; page: number; zone: Zone }
  | { id: string; sorte: "soulignage"; page: number; teinte: Teinte; zones: Zone[]; extrait?: string }
  | { id: string; sorte: "barre"; page: number; teinte: Teinte; zones: Zone[]; extrait?: string }
  /** Rectangle ou ellipse entre deux coins opposés ; ligne ou flèche de `debut` à `fin`. */
  | { id: string; sorte: "forme"; page: number; forme: Forme; debut: Point; fin: Point; teinte: Teinte; epaisseur: number }
  /** Un trait à main levée. */
  | { id: string; sorte: "dessin"; page: number; points: Point[]; teinte: Teinte; epaisseur: number }
  | { id: string; sorte: "tampon"; page: number; zone: Zone; mot: MotTampon; date: string | null };

/** Ce que le cœur reçoit : les annotations sans leur identifiant (ni l'ancien texte d'une
 * correction), en laissant de côté les vides. Une correction vide efface le mot. */
export function pourLeCoeur(annotations: Annotation[]) {
  return annotations
    .filter((a) => {
      if (a.sorte === "texte") return a.texte.trim() !== "";
      if (a.sorte === "surlignage" || a.sorte === "soulignage" || a.sorte === "barre") return a.zones.length > 0;
      if (a.sorte === "dessin") return a.points.length >= 2;
      if (a.sorte === "forme") return Math.hypot(a.fin.x - a.debut.x, a.fin.y - a.debut.y) >= 1;
      return true;
    })
    .map((annotation) => {
      if (annotation.sorte === "correction") {
        const { id: _id, ancien: _ancien, ...reste } = annotation;
        return reste;
      }
      const { id: _id, ...reste } = annotation;
      return reste;
    });
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

/** Premier et dernier mots parcourus en glissant de `debut` à `fin`, si le geste part d'un mot. */
function motsParcourus(mots: MotPlace[], debut: Point, fin: Point, portee: number): [number, number] | null {
  const premier = motProche(mots, debut, portee);
  if (premier === null) return null;
  const dernier = motProche(mots, fin, Number.POSITIVE_INFINITY) ?? premier;
  return premier <= dernier ? [premier, dernier] : [dernier, premier];
}

/** Le texte surligné (souligné, barré) en glissant de `debut` à `fin` ; vide pour un cadre tracé. */
export function extraitSurligne(mots: MotComplet[], debut: Point, fin: Point, portee = 12): string {
  const bornes = motsParcourus(mots, debut, fin, portee);
  return bornes ? mots.slice(bornes[0], bornes[1] + 1).map((mot) => mot.texte).join(" ") : "";
}

/**
 * Zones à surligner quand on glisse de `debut` à `fin` : si le geste part d'un mot, les mots
 * parcourus dans l'ordre de lecture, une zone par ligne ; sinon le cadre tracé (page numérisée,
 * dessin, tableau).
 */
export function zonesSurlignees(mots: MotPlace[], debut: Point, fin: Point, portee = 12): Zone[] {
  const bornes = motsParcourus(mots, debut, fin, portee);
  if (!bornes) {
    const cadre = zoneEntre(debut, fin);
    return cadre.largeur > 2 && cadre.hauteur > 2 ? [cadre] : [];
  }
  const [a, b] = bornes;
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

/**
 * Mots à corriger quand on clique (ou glisse) de `debut` à `fin` : du mot sous le premier point
 * jusqu'à celui sous le second, sur la seule ligne du premier, écrits de gauche à droite. Rien si
 * le geste ne part pas d'un mot.
 */
export function motsDeLaLigne(mots: MotComplet[], debut: Point, fin: Point, portee = 6): number[] {
  const premier = motProche(mots, debut, portee);
  if (premier === null || mots[premier].sens !== 0) return [];
  let premierDeLigne = premier;
  while (premierDeLigne > 0 && !mots[premierDeLigne].debutLigne) premierDeLigne--;
  let dernierDeLigne = premier;
  while (dernierDeLigne + 1 < mots.length && !mots[dernierDeLigne + 1].debutLigne) dernierDeLigne++;
  const ligne = mots.slice(premierDeLigne, dernierDeLigne + 1);
  const second = premierDeLigne + (motProche(ligne, fin, Number.POSITIVE_INFINITY) ?? premier - premierDeLigne);
  const [a, b] = premier <= second ? [premier, second] : [second, premier];
  return Array.from({ length: b - a + 1 }, (_, i) => a + i);
}

/** Un montant : des chiffres, avec leurs séparateurs, un signe, une devise. Il garde son bord droit. */
export function estMontant(texte: string): boolean {
  const sansDevise = texte.trim().replace(/\s*([A-Z]{2,4}|€|\$|£|%)$/u, "").replace(/^([A-Z]{2,4}|€|\$|£)\s*/u, "");
  return /\d/.test(sansDevise) && /^[-+−(]?[\d\s\u00a0\u202f.,'’]+\)?$/u.test(sansDevise);
}

/** Correction de ces mots, prête à être écrite : le nouveau texte est d'abord l'ancien. */
export function correctionDe(mots: MotComplet[], rangs: number[], page: number): Annotation & { sorte: "correction" } {
  const choisis = rangs.map((rang) => mots[rang]);
  const gauche = Math.min(...choisis.map((m) => m.x));
  const droite = Math.max(...choisis.map((m) => m.x + m.longueur));
  const haut = Math.min(...choisis.map((m) => m.y));
  const bas = Math.max(...choisis.map((m) => m.y + m.taille));
  const premier = choisis[0];
  const ancien = choisis.map((m) => m.texte).join(" ");
  return {
    id: nouvelId(),
    sorte: "correction",
    page,
    zone: { x: gauche, y: haut, largeur: droite - gauche, hauteur: bas - haut },
    x: gauche,
    fin: droite,
    ligne: premier.y + 0.8 * premier.taille,
    ancien,
    texte: ancien,
    taille: premier.taille,
    famille: premier.famille,
    gras: premier.gras,
    italique: premier.italique,
    couleur: premier.couleur,
    droite: estMontant(ancien),
  };
}

/** Police d'affichage proche, comme le cœur la choisit (police.rs, Famille::de). */
export function policeCss(famille: string): string {
  const nom = famille.toLowerCase();
  if (["courier", "mono", "consolas", "console"].some((m) => nom.includes(m))) return '"Courier New", Courier, monospace';
  if (!nom.includes("sans") && ["times", "georgia", "garamond", "cambria", "serif", "book", "palatino", "century", "roman"].some((m) => nom.includes(m)))
    return '"Times New Roman", Times, serif';
  return "Arial, Helvetica, sans-serif";
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

/** Zone d'un tampon posé d'un clic, centrée sur le point : assez large pour son mot. */
export function zoneTampon(mot: MotTampon, centre: Point): Zone {
  const hauteur = 54;
  const largeur = Math.max(110, TAMPONS[mot].texte.length * 17 + 40);
  return { x: centre.x - largeur / 2, y: centre.y - hauteur / 2, largeur, hauteur };
}

/** Date du jour pour un tampon : « 25/11/2026 ». */
export function dateDuJour(maintenant = new Date()): string {
  const deux = (n: number) => String(n).padStart(2, "0");
  return `${deux(maintenant.getDate())}/${deux(maintenant.getMonth() + 1)}/${maintenant.getFullYear()}`;
}

/** Le trait prolongé jusqu'au point, s'il s'est assez éloigné du dernier : un dessin plus léger, aussi net. */
export function prolonger(points: Point[], point: Point, ecart = 0.75): Point[] {
  const dernier = points[points.length - 1];
  return !dernier || Math.hypot(point.x - dernier.x, point.y - dernier.y) >= ecart ? [...points, point] : points;
}

/** Rectangle qui contient l'annotation, en points de la page : pour la retrouver et la montrer. */
export function zoneDe(annotation: Annotation): Zone {
  switch (annotation.sorte) {
    case "surlignage":
    case "soulignage":
    case "barre": {
      const xs = annotation.zones.flatMap((z) => [z.x, z.x + z.largeur]);
      const ys = annotation.zones.flatMap((z) => [z.y, z.y + z.hauteur]);
      return zoneEntre({ x: Math.min(...xs), y: Math.min(...ys) }, { x: Math.max(...xs), y: Math.max(...ys) });
    }
    case "forme":
      return zoneEntre(annotation.debut, annotation.fin);
    case "dessin": {
      const xs = annotation.points.map((p) => p.x);
      const ys = annotation.points.map((p) => p.y);
      return zoneEntre({ x: Math.min(...xs), y: Math.min(...ys) }, { x: Math.max(...xs), y: Math.max(...ys) });
    }
    case "image":
    case "blanc":
    case "correction":
    case "tampon":
      return annotation.zone;
    case "note":
      return { x: annotation.x, y: annotation.y, largeur: 20, hauteur: 20 };
    case "texte":
      return { x: annotation.x, y: annotation.y, largeur: annotation.taille * 8, hauteur: annotation.taille * INTERLIGNE };
  }
}

/** Les annotations dans l'ordre de lecture : page après page, de haut en bas, puis de gauche à droite. */
export function ordreDeLecture(annotations: Annotation[]): Annotation[] {
  const places = new Map(annotations.map((a) => [a.id, zoneDe(a)]));
  return [...annotations].sort((a, b) => {
    const [za, zb] = [places.get(a.id)!, places.get(b.id)!];
    // Sur une même ligne (à quelques points près), la plus à gauche d'abord.
    return a.page - b.page || (Math.abs(za.y - zb.y) > 4 ? za.y - zb.y : za.x - zb.x);
  });
}
