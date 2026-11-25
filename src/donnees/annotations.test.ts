import { describe, expect, it } from "vitest";
import { annuler, appliquer, HISTORIQUE_VIDE, nomAnnote, pourLeCoeur, retablir, zonesSurlignees, type Annotation, type MotPlace } from "./annotations";

// Deux lignes : « Net à payer : 389 250 » puis « Signature ».
const mot = (x: number, y: number, longueur: number, debutLigne = false): MotPlace => ({ x, y, longueur, taille: 10, debutLigne });
const MOTS = [mot(100, 100, 20, true), mot(124, 100, 8), mot(136, 100, 30), mot(170, 100, 4), mot(178, 100, 18), mot(200, 100, 18), mot(100, 120, 45, true)];

const note = (id: string, texte = "Vu"): Annotation => ({ id, sorte: "note", page: 0, x: 10, y: 10, texte });

describe("surligner en glissant", () => {
  it("surligne les mots parcourus, une zone par ligne", () => {
    // Du milieu de « à » jusque sur « Signature », à la ligne suivante.
    expect(zonesSurlignees(MOTS, { x: 127, y: 105 }, { x: 110, y: 125 })).toEqual([
      { x: 124, y: 100, largeur: 94, hauteur: 10 },
      { x: 100, y: 120, largeur: 45, hauteur: 10 },
    ]);
  });

  it("donne le même résultat dans l'autre sens", () => {
    expect(zonesSurlignees(MOTS, { x: 110, y: 125 }, { x: 127, y: 105 })).toEqual(zonesSurlignees(MOTS, { x: 127, y: 105 }, { x: 110, y: 125 }));
  });

  it("surligne un seul mot sur un simple clic", () => {
    expect(zonesSurlignees(MOTS, { x: 140, y: 104 }, { x: 140, y: 104 })).toEqual([{ x: 136, y: 100, largeur: 30, hauteur: 10 }]);
  });

  it("trace un cadre loin du texte, et rien pour un clic dans le vide", () => {
    expect(zonesSurlignees(MOTS, { x: 300, y: 400 }, { x: 250, y: 380 })).toEqual([{ x: 250, y: 380, largeur: 50, hauteur: 20 }]);
    expect(zonesSurlignees([], { x: 300, y: 400 }, { x: 301, y: 401 })).toEqual([]);
  });
});

describe("annuler et rétablir", () => {
  it("revient en arrière puis en avant, et oublie l'avenir après une nouvelle action", () => {
    const un = appliquer(HISTORIQUE_VIDE, [note("a")]);
    const deux = appliquer(un, [note("a"), note("b")]);
    expect(annuler(deux).present.map((a) => a.id)).toEqual(["a"]);
    expect(retablir(annuler(deux)).present.map((a) => a.id)).toEqual(["a", "b"]);
    const autre = appliquer(annuler(deux), [note("c")]);
    expect(autre.futur).toEqual([]);
    expect(annuler(annuler(autre)).present).toEqual([]);
    expect(annuler(HISTORIQUE_VIDE)).toBe(HISTORIQUE_VIDE);
  });
});

describe("envoi au cœur", () => {
  it("retire les identifiants et laisse de côté les textes vides", () => {
    const annotations: Annotation[] = [
      note("a", "À voir"),
      { id: "b", sorte: "texte", page: 1, x: 5, y: 6, texte: "  ", taille: 12, teinte: "noir" },
      { id: "c", sorte: "surlignage", page: 0, teinte: "jaune", zones: [] },
    ];
    expect(pourLeCoeur(annotations)).toEqual([{ sorte: "note", page: 0, x: 10, y: 10, texte: "À voir" }]);
  });

  it("propose un nom à côté de l'original", () => {
    expect(nomAnnote("Contrat de travail.PDF")).toBe("Contrat de travail - annoté.pdf");
  });
});
