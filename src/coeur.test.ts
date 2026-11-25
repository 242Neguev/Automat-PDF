import { describe, expect, it } from "vitest";
import { enErreur, MOT_DE_PASSE } from "./coeur";

describe("erreurs du cœur", () => {
  it("garde le message, le détail et, s'il y en a un, le motif", () => {
    expect(enErreur({ message: "« Bulletin.pdf » est protégé par un mot de passe.", details: [], motif: MOT_DE_PASSE })).toEqual({
      message: "« Bulletin.pdf » est protégé par un mot de passe.",
      details: [],
      motif: "mot_de_passe",
    });
    expect(enErreur({ message: "Illisible", details: ["page 2"] })).toEqual({ message: "Illisible", details: ["page 2"] });
    expect(enErreur("texte")).toEqual({ message: "texte", details: [] });
  });
});
