import { describe, expect, it } from "vitest";
import { ecrireReglages, lireReglages, nomNumerisation, REGLAGES_PAR_DEFAUT } from "./numerisation";

describe("numérisation", () => {
  it("propose un nom daté pour le PDF", () => {
    expect(nomNumerisation(new Date(2026, 10, 26, 17, 5))).toBe("Numérisation du 26 novembre 2026 à 17 h 05.pdf");
    expect(nomNumerisation(new Date(2027, 0, 3, 9, 30))).toBe("Numérisation du 3 janvier 2027 à 9 h 30.pdf");
  });

  it("garde les réglages et ignore ce qui est abîmé", () => {
    const memoire = new Map<string, string>();
    const stockage = { getItem: (cle: string) => memoire.get(cle) ?? null, setItem: (cle: string, valeur: string) => void memoire.set(cle, valeur) };
    expect(lireReglages(stockage)).toEqual(REGLAGES_PAR_DEFAUT);
    const reglages = { scanner: "ABC", source: "chargeur", rectoVerso: true, couleur: "gris", resolution: 600, reconnaitre: false } as const;
    ecrireReglages(reglages, stockage);
    expect(lireReglages(stockage)).toEqual(reglages);
    memoire.set("automat-pdf.numerisation", JSON.stringify({ source: "plafond", resolution: 72, couleur: "gris" }));
    expect(lireReglages(stockage)).toEqual({ ...REGLAGES_PAR_DEFAUT, couleur: "gris" });
    memoire.set("automat-pdf.numerisation", "pas du JSON");
    expect(lireReglages(stockage)).toEqual(REGLAGES_PAR_DEFAUT);
    expect(lireReglages(null)).toEqual(REGLAGES_PAR_DEFAUT);
  });
});
