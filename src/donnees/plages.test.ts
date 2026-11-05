import { describe, expect, it } from "vitest";
import { lirePlages, pagesSelon } from "./plages";

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
