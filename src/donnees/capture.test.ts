import { describe, expect, it } from "vitest";
import type { Mot } from "../coeur";
import { assezGrande, enFractions, tailleEnCm, texteDansZone, zoneEntre } from "./capture";

const A4 = { largeur: 595, hauteur: 842 };

function mot(texte: string, x: number, y: number, debutLigne = false, sens = 0): Mot {
  return { texte, x, y, longueur: texte.length * 6, taille: 12, sens, debutLigne, famille: "Arial", gras: false, italique: false, couleur: [0, 0, 0] };
}

describe("capturer une zone", () => {
  it("prend le cadre tracé dans les deux sens, gardé dans la page", () => {
    expect(zoneEntre({ x: 300, y: 200 }, { x: 100, y: 50 }, A4)).toEqual({ x: 100, y: 50, largeur: 200, hauteur: 150 });
    expect(zoneEntre({ x: -20, y: 800 }, { x: 700, y: 900 }, A4)).toEqual({ x: 0, y: 800, largeur: 595, hauteur: 42 });
    expect(assezGrande({ x: 10, y: 10, largeur: 2, hauteur: 50 })).toBe(false);
    expect(assezGrande({ x: 10, y: 10, largeur: 40, hauteur: 50 })).toBe(true);
  });

  it("donne la zone au cœur en fractions de la page, et sa taille en centimètres", () => {
    expect(enFractions({ x: 59.5, y: 421, largeur: 297.5, hauteur: 210.5 }, A4)).toEqual({ x: 0.1, y: 0.5, largeur: 0.5, hauteur: 0.25 });
    expect(tailleEnCm({ x: 0, y: 0, largeur: 72, hauteur: 141.73 })).toBe("2,5 × 5,0 cm");
  });

  it("garde le texte du cadre, ligne par ligne", () => {
    const mots = [
      mot("Bulletin", 72, 60, true),
      mot("de", 130, 60),
      mot("paie", 150, 60),
      mot("Nom", 72, 100, true),
      mot(":", 100, 100),
      mot("DIALLO", 110, 100),
      mot("Matricule", 400, 100),
      mot("Net", 72, 140, true),
      mot("à", 95, 140),
      mot("payer", 105, 140),
    ];
    // Le cadre prend les deux premières lignes, sans la colonne de droite.
    expect(texteDansZone(mots, { x: 60, y: 50, largeur: 200, hauteur: 70 })).toEqual({ texte: "Bulletin de paie\nNom : DIALLO", mots: 6 });
    // Une ligne sautée garde le saut de ligne.
    expect(texteDansZone(mots, { x: 60, y: 50, largeur: 100, hauteur: 110 }).texte).toBe("Bulletin de\nNom : DIALLO\nNet à payer");
    expect(texteDansZone(mots, { x: 300, y: 300, largeur: 50, hauteur: 50 })).toEqual({ texte: "", mots: 0 });
  });

  it("situe un mot écrit de haut en bas", () => {
    // Écrit vers le bas (un quart de tour) : sa boîte descend sous (x, y) et s'étend à gauche.
    const vertical = mot("Copie", 500, 300, true, 1);
    expect(texteDansZone([vertical], { x: 480, y: 300, largeur: 30, hauteur: 40 }).texte).toBe("Copie");
    expect(texteDansZone([vertical], { x: 500, y: 300, largeur: 30, hauteur: 40 }).texte).toBe("");
  });
});
