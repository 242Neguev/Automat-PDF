import { describe, expect, it } from "vitest";
import { echelleAjustee, hautAffiche, premiereDeRangee, rangees, tailleAffichee, tourner, transformation, versLaPage, type Rotation } from "./affichage";

const A4 = { largeur: 595, hauteur: 842 };
const PAYSAGE = { largeur: 842, hauteur: 595 };

/** Où la transformation CSS envoie un point de la page droite : la même formule que le navigateur. */
function affiche(x: number, y: number, rotation: Rotation, l: number, h: number) {
  const css = transformation(rotation, l, h);
  if (!css) return { x, y };
  const [, tx, ty] = /translate\(([-\d.]+)px, ([-\d.]+)px\)/.exec(css)!.map(Number);
  const angle = (rotation * Math.PI) / 180;
  return { x: Math.round(tx + x * Math.cos(angle) - y * Math.sin(angle)), y: Math.round(ty + x * Math.sin(angle) + y * Math.cos(angle)) };
}

describe("rotation de l'affichage", () => {
  it("tourne d'un quart de tour, dans un sens ou dans l'autre", () => {
    expect(tourner(0)).toBe(90);
    expect(tourner(270)).toBe(0);
    expect(tourner(0, -1)).toBe(270);
    expect(tourner(90, -1)).toBe(0);
  });

  it("échange largeur et hauteur d'un quart de tour", () => {
    expect(tailleAffichee(A4, 90)).toEqual(PAYSAGE);
    expect(tailleAffichee(A4, 180)).toEqual(A4);
  });

  it("garde la page tournée dans sa boîte, et retrouve le point de la page sous le pointeur", () => {
    const [l, h] = [200, 300];
    for (const rotation of [0, 90, 180, 270] as Rotation[]) {
      const boite = tailleAffichee({ largeur: l, hauteur: h }, rotation);
      for (const [x, y] of [[0, 0], [l, h], [20, 50], [150, 10]]) {
        const vu = affiche(x, y, rotation, l, h);
        expect(vu.x, `rotation ${rotation}`).toBeGreaterThanOrEqual(0);
        expect(vu.x).toBeLessThanOrEqual(boite.largeur);
        expect(vu.y).toBeGreaterThanOrEqual(0);
        expect(vu.y).toBeLessThanOrEqual(boite.hauteur);
        expect(versLaPage(vu.x, vu.y, l, h, rotation)).toEqual({ x, y });
      }
    }
  });

  it("mesure la hauteur affichée d'un point, pour y faire défiler", () => {
    // Le haut de la page droite (y = 0) : en haut, sur le côté ou en bas selon la rotation.
    expect(hautAffiche(A4, 100, 0, 0)).toBe(0);
    expect(hautAffiche(A4, 100, 0, 180)).toBe(842);
    expect(hautAffiche(A4, 100, 50, 90)).toBe(100);
    expect(hautAffiche(A4, 100, 50, 270)).toBe(495);
    for (const rotation of [0, 90, 180, 270] as Rotation[]) {
      expect(hautAffiche(A4, 120, 300, rotation)).toBe(affiche(120, 300, rotation, A4.largeur, A4.hauteur).y);
    }
  });
});

describe("une ou deux pages", () => {
  it("range les pages deux par deux, la dernière seule s'il le faut", () => {
    expect(rangees(5, "deux")).toEqual([[0, 1], [2, 3], [4]]);
    expect(rangees(3, "une")).toEqual([[0], [1], [2]]);
    expect(rangees(0, "deux")).toEqual([]);
    expect(premiereDeRangee(3, "deux")).toBe(2);
    expect(premiereDeRangee(3, "une")).toBe(3);
  });

  it("ajuste l'échelle à la plus large rangée, ou à la page entière", () => {
    const zone = { largeur: 1232, hauteur: 900 };
    // Une page A4 : 1 200 pixels pour 595 points ; deux côte à côte (et un écart) : la moitié, à peu près.
    expect(echelleAjustee([A4, A4], 0, "une", zone, "largeur", 16)).toBeCloseTo(1200 / 595);
    expect(echelleAjustee([A4, A4], 0, "deux", zone, "largeur", 16)).toBeCloseTo(1184 / 1190);
    // Page entière : la hauteur l'emporte.
    expect(echelleAjustee([A4], 0, "une", zone, "page", 16)).toBeCloseTo(868 / 842);
    // Tournées d'un quart de tour, les pages A4 sont couchées : plus larges que hautes.
    expect(echelleAjustee([A4], 90, "une", zone, "largeur", 16)).toBeCloseTo(1200 / 842);
    // Une seule page en « deux pages » : pas d'écart à garder.
    expect(echelleAjustee([A4], 0, "deux", zone, "largeur", 16)).toBeCloseTo(1200 / 595);
  });
});
