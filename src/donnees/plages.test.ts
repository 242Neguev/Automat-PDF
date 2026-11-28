import { describe, expect, it } from "vitest";
import { ecrirePlages, lirePlages, nomExtrait, pagesSelon } from "./plages";

describe("plages de pages", () => {
  it("lit pages seules, plages et plages ouvertes", () => {
    expect(lirePlages("1-3, 5", 10)).toEqual({ pages: [0, 1, 2, 4] });
    expect(lirePlages("8-", 10)).toEqual({ pages: [7, 8, 9] });
    expect(lirePlages("-2;4  6", 10)).toEqual({ pages: [0, 1, 3, 5] });
  });

  it("garde l'ordre donné, sans doublon", () => {
    expect(lirePlages("5, 1-2, 2", 10)).toEqual({ pages: [4, 0, 1] });
  });

  it("explique ce qui ne va pas", () => {
    expect(lirePlages("", 10)).toHaveProperty("erreur");
    expect(lirePlages("2-a", 10)).toEqual({ erreur: "« 2-a » n'est pas un numéro de page ni une plage (ex. 2-5)." });
    expect(lirePlages("9-12", 10)).toEqual({ erreur: "« 9-12 » sort du document, qui compte 10 pages." });
    expect(lirePlages("0", 3)).toHaveProperty("erreur");
  });
});

describe("pages choisies en les voyant", () => {
  it("les écrit comme on les tape, et se relit", () => {
    expect(ecrirePlages([])).toBe("");
    expect(ecrirePlages([2])).toBe("3");
    expect(ecrirePlages([6, 4, 2, 5, 5])).toBe("3, 5-7");
    expect(ecrirePlages([0, 1])).toBe("1-2");
    expect(lirePlages(ecrirePlages([9, 0, 3, 4, 5]), 10)).toEqual({ pages: [0, 3, 4, 5, 9] });
  });

  it("nomme le PDF d'après ses pages", () => {
    expect(nomExtrait("Rapport", [2])).toBe("Rapport - page 3.pdf");
    expect(nomExtrait("Rapport", [4, 2, 3])).toBe("Rapport - pages 3 à 5.pdf");
    expect(nomExtrait("Rapport", [1, 4])).toBe("Rapport - pages 2 et 5.pdf");
    expect(nomExtrait("Rapport", [6, 1, 4, 4])).toBe("Rapport - pages 2, 5 et 7.pdf");
    expect(nomExtrait("Rapport", [0, 2, 4, 6, 8])).toBe("Rapport - 5 pages.pdf");
    expect(nomExtrait("Rapport", [])).toBe("Rapport.pdf");
  });
});

describe("pages concernées par une opération", () => {
  it("traduit chaque choix en pages (vide : toutes)", () => {
    expect(pagesSelon("toutes", "", 5, 2)).toEqual({ pages: [] });
    expect(pagesSelon("sauf-premiere", "", 4, 0)).toEqual({ pages: [1, 2, 3] });
    expect(pagesSelon("cette", "", 5, 2)).toEqual({ pages: [2] });
    expect(pagesSelon("derniere", "", 5, 0)).toEqual({ pages: [4] });
    expect(pagesSelon("certaines", "2, 4", 5, 0)).toEqual({ pages: [1, 3] });
    expect(pagesSelon("sauf-premiere", "", 1, 0)).toHaveProperty("erreur");
  });
});
