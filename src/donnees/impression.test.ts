import { describe, expect, it } from "vitest";
import { feuillesNecessaires, placementApercu } from "./impression";

// A4 portrait, marges de 5 mm : zone imprimable de 200 × 287 mm.
const A4 = { largeur: 210, hauteur: 297, marges: [5, 5, 5, 5] as [number, number, number, number] };
const PAGE_A4 = { largeur: 595.28, hauteur: 841.89 };
const PAGE_A5 = { largeur: 419.53, hauteur: 595.28 };

describe("aperçu de l'impression", () => {
  it("réduit une page A4 dans la zone imprimable, sans la déformer, centrée", () => {
    const p = placementApercu(PAGE_A4, A4, "reduire", "auto");
    expect(p.tournee).toBe(false);
    // La largeur limite : 200 / 210 < 287 / 297.
    expect(p.largeur).toBeCloseTo(200, 1);
    expect(p.hauteur).toBeCloseTo((297 * 200) / 210, 1);
    expect(p.x).toBeCloseTo(5 + (200 - p.largeur) / 2, 5);
  });

  it("garde une petite page à sa taille, ou l'agrandit si l'on ajuste", () => {
    const reelle = placementApercu(PAGE_A5, A4, "reduire", "auto");
    expect(reelle.largeur).toBeCloseTo(148, 0);
    expect(reelle.hauteur).toBeCloseTo(210, 0);
    const ajustee = placementApercu(PAGE_A5, A4, "ajuster", "auto");
    expect(ajustee.largeur).toBeCloseTo(200, 1);
    expect(ajustee.hauteur).toBeGreaterThan(210);
  });

  it("tourne une page paysage sur une feuille portrait, en automatique seulement", () => {
    const paysage = { largeur: 841.89, hauteur: 595.28 };
    expect(placementApercu(paysage, A4, "reduire", "auto").tournee).toBe(true);
    expect(placementApercu(paysage, A4, "reduire", "portrait").tournee).toBe(false);
  });

  it("compte les feuilles, en recto verso aussi", () => {
    expect(feuillesNecessaires(5, 2, false, true)).toBe(10);
    expect(feuillesNecessaires(5, 2, true, true), "Chaque copie sur ses feuilles : 3 + 3.").toBe(6);
    expect(feuillesNecessaires(5, 2, true, false)).toBe(5);
    expect(feuillesNecessaires(1, 0, false, true)).toBe(1);
  });
});
