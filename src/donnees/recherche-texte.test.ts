import { describe, expect, it } from "vitest";
import { compter, motsCouverts, normaliser, occurrences } from "./recherche-texte";

describe("recherche dans le texte d'un PDF", () => {
  it("compare sans majuscules, accents ni espaces en trop", () => {
    expect(normaliser("  Net  À\tPAYER ")).toBe("net a payer");
    expect(normaliser("Période d’essai")).toBe("periode d'essai");
    expect(compter("NET À PAYER : 180 000\nnet a payer", "net a payer")).toBe(2);
    expect(compter("aaaa", "aa")).toBe(2);
    expect(compter("texte", "")).toBe(0);
  });

  it("donne les occurrences de tout le document, page par page", () => {
    const textes = ["Bulletin de paie\nNet à payer", "", "Net à payer\tNet à payer"];
    expect(occurrences(textes, "NET A PAYER")).toEqual([
      { page: 0, rang: 0 },
      { page: 2, rang: 0 },
      { page: 2, rang: 1 },
    ]);
    expect(occurrences(textes, "   ")).toEqual([]);
  });

  it("trouve les mots que couvre chaque occurrence, sur un ou plusieurs mots", () => {
    const mots = ["Net", "à", "payer", ":", "180", "000", "FCFA", "Payé"].map((texte) => ({ texte }));
    expect(motsCouverts(mots, "net à payer")).toEqual([[0, 1, 2]]);
    expect(motsCouverts(mots, "pay")).toEqual([[2], [7]]);
    expect(motsCouverts(mots, "180 000")).toEqual([[4, 5]]);
    expect(motsCouverts(mots, "")).toEqual([]);
  });
});
