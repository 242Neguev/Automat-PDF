// Signature et initiales dessinées (0.7.0) : du dessin fait dans le pavé au trait gardé, puis posé
// sur la page. Le lissage est celui du cœur (annotations.rs) : l'aperçu et le PDF se ressemblent.
import type { Point, Teinte, Trace, Zone } from "./annotations";

/** Hauteur d'une signature posée d'un clic, et d'initiales, en points ; largeur au plus. */
export const HAUTEUR_SIGNATURE = 42;
export const HAUTEUR_INITIALES = 28;
const LARGEUR_MAX = 240;
/** Encres proposées pour signer. */
export const ENCRES: Teinte[] = ["marine", "noir"];
/** Au-delà, ce n'est plus une signature : le reste du dessin n'est pas gardé. */
const TRAITS_MAX = 400;
const POINTS_MAX = 20_000;

/** Une signature (ou des initiales) choisie pour être posée. */
export interface SignatureChoisie {
  trace: Trace;
  teinte: Teinte;
  initiales: boolean;
}

/** Ce qui est gardé sur l'ordinateur pour la prochaine fois. */
export interface SignatureGardee {
  trace: Trace;
  teinte: Teinte;
}
export interface SignaturesGardees {
  signature?: SignatureGardee;
  initiales?: SignatureGardee;
}

const arrondi = (n: number) => Math.round(n * 10000) / 10000;

/**
 * Le chemin SVG lissé d'un trait, ses points mis à l'échelle : des courbes qui passent par le
 * milieu de chaque segment, comme dans le PDF. Un point seul : un trait de longueur nulle (point
 * rond, avec des bouts arrondis).
 */
export function chemin(points: Point[], echelleX = 1, echelleY = 1): string {
  const p = points.map((q) => ({ x: Number((q.x * echelleX).toFixed(2)), y: Number((q.y * echelleY).toFixed(2)) }));
  if (p.length === 0) return "";
  if (p.length === 1) return `M${p[0].x} ${p[0].y}L${p[0].x} ${p[0].y}`;
  let d = `M${p[0].x} ${p[0].y}`;
  for (let i = 1; i < p.length - 1; i++) {
    const fin = { x: Number(((p[i].x + p[i + 1].x) / 2).toFixed(2)), y: Number(((p[i].y + p[i + 1].y) / 2).toFixed(2)) };
    d += `Q${p[i].x} ${p[i].y} ${fin.x} ${fin.y}`;
  }
  const dernier = p[p.length - 1];
  return `${d}L${dernier.x} ${dernier.y}`;
}

/**
 * Le trait gardé d'un dessin fait dans le pavé (en pixels du pavé, trait d'épaisseur `epaisseur`) :
 * recadré au plus près, la moitié de l'épaisseur autour, en fractions de sa boîte. Une boîte trop
 * plate (un trait droit) ou trop étroite garde au moins quatre épaisseurs. `null` : rien de dessiné.
 */
export function normaliser(traits: Point[][], epaisseur: number): Trace | null {
  const gardes = traits.filter((trait) => trait.length > 0).slice(0, TRAITS_MAX);
  const points = gardes.flat().slice(0, POINTS_MAX);
  if (points.length === 0) return null;
  const marge = epaisseur / 2;
  const [minX, maxX] = [Math.min(...points.map((p) => p.x)) - marge, Math.max(...points.map((p) => p.x)) + marge];
  const [minY, maxY] = [Math.min(...points.map((p) => p.y)) - marge, Math.max(...points.map((p) => p.y)) + marge];
  const largeur = Math.max(maxX - minX, epaisseur * 4);
  const hauteur = Math.max(maxY - minY, epaisseur * 4);
  const [x0, y0] = [(minX + maxX - largeur) / 2, (minY + maxY - hauteur) / 2];
  let restants = POINTS_MAX;
  return {
    proportion: arrondi(largeur / hauteur),
    traits: gardes
      .map((trait) => {
        const pris = trait.slice(0, Math.max(0, restants));
        restants -= pris.length;
        return pris.map((p) => ({ x: arrondi((p.x - x0) / largeur), y: arrondi((p.y - y0) / hauteur) }));
      })
      .filter((trait) => trait.length > 0),
    epaisseur: arrondi(epaisseur / hauteur),
  };
}

/** La zone d'une signature posée d'un clic : centrée sur le point cliqué, à sa taille habituelle. */
export function zoneSignature(trace: Trace, centre: Point, initiales: boolean): Zone {
  let hauteur = initiales ? HAUTEUR_INITIALES : HAUTEUR_SIGNATURE;
  let largeur = hauteur * trace.proportion;
  if (largeur > LARGEUR_MAX) {
    largeur = LARGEUR_MAX;
    hauteur = largeur / trace.proportion;
  }
  return { x: centre.x - largeur / 2, y: centre.y - hauteur / 2, largeur, hauteur };
}

/** La zone d'une signature posée dans un cadre (un champ de signature) : aussi grande que possible, centrée. */
export function zoneDansCadre(trace: Trace, cadre: Zone): Zone {
  const marge = Math.min(cadre.largeur, cadre.hauteur) * 0.08;
  const [disponibleL, disponibleH] = [Math.max(1, cadre.largeur - 2 * marge), Math.max(1, cadre.hauteur - 2 * marge)];
  let hauteur = disponibleH;
  let largeur = hauteur * trace.proportion;
  if (largeur > disponibleL) {
    largeur = disponibleL;
    hauteur = largeur / trace.proportion;
  }
  return { x: cadre.x + (cadre.largeur - largeur) / 2, y: cadre.y + (cadre.hauteur - hauteur) / 2, largeur, hauteur };
}

/** Le milieu de `zone` est-il dans `cadre` ? (une signature posée dans un champ de signature) */
export function dansLeCadre(zone: Zone, cadre: Zone): boolean {
  const [x, y] = [zone.x + zone.largeur / 2, zone.y + zone.hauteur / 2];
  return x >= cadre.x && x <= cadre.x + cadre.largeur && y >= cadre.y && y <= cadre.y + cadre.hauteur;
}

const nombre = (v: unknown, min: number, max: number): number | null => (typeof v === "number" && Number.isFinite(v) && v >= min && v <= max ? v : null);

/** Un trait gardé, relu avec prudence (fichier abîmé ou modifié à la main : ignoré). */
function traceLue(brute: unknown): Trace | null {
  if (!brute || typeof brute !== "object") return null;
  const { proportion, epaisseur, traits } = brute as Record<string, unknown>;
  const p = nombre(proportion, 0.01, 100);
  const e = nombre(epaisseur, 0.0001, 1);
  if (p === null || e === null || !Array.isArray(traits)) return null;
  const lus: Point[][] = [];
  for (const trait of traits.slice(0, TRAITS_MAX)) {
    if (!Array.isArray(trait)) return null;
    const points: Point[] = [];
    for (const point of trait) {
      const x = nombre((point as Record<string, unknown>)?.x, 0, 1);
      const y = nombre((point as Record<string, unknown>)?.y, 0, 1);
      if (x === null || y === null) return null;
      points.push({ x, y });
    }
    if (points.length > 0) lus.push(points);
  }
  return lus.length > 0 ? { proportion: p, epaisseur: e, traits: lus } : null;
}

function gardeeLue(brute: unknown): SignatureGardee | undefined {
  if (!brute || typeof brute !== "object") return undefined;
  const { trace, teinte } = brute as Record<string, unknown>;
  const lue = traceLue(trace);
  if (!lue) return undefined;
  return { trace: lue, teinte: ENCRES.includes(teinte as Teinte) ? (teinte as Teinte) : "marine" };
}

/** La signature et les initiales gardées (fichier vide, absent ou illisible : aucune). */
export function lireSignatures(contenu: string): SignaturesGardees {
  try {
    const brut = JSON.parse(contenu) as Record<string, unknown>;
    const gardees: SignaturesGardees = {};
    const signature = gardeeLue(brut?.signature);
    const initiales = gardeeLue(brut?.initiales);
    if (signature) gardees.signature = signature;
    if (initiales) gardees.initiales = initiales;
    return gardees;
  } catch {
    return {};
  }
}

/** Ce qui s'écrit dans le fichier des signatures gardées. */
export const ecrireSignatures = (gardees: SignaturesGardees) => JSON.stringify(gardees);
