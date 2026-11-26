import { describe, expect, it } from "vitest";
import { partFaite } from "./avancement";

describe("avancement d'un travail sur des fichiers", () => {
  it("compte les fichiers finis", () => {
    expect(partFaite([0, 4])).toBe(0);
    expect(partFaite([1, 4])).toBe(0.25);
    expect(partFaite([4, 4])).toBe(1);
  });

  it("ajoute la part du fichier en cours, page par page", () => {
    expect(partFaite([1, 4, 5, 10])).toBeCloseTo(0.375);
    expect(partFaite([0, 1, 3, 4])).toBeCloseTo(0.75);
    // Jamais plus que tout, même si le cœur annonce une page de trop.
    expect(partFaite([3, 4, 12, 10])).toBe(1);
    expect(partFaite([0, 0])).toBe(0);
  });
});
