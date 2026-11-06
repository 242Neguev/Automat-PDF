import { describe, expect, it } from "vitest";
import { aCote, dossierDe, nomDe } from "./chemins";

describe("chemins", () => {
  it("donne le nom et le dossier d'un fichier", () => {
    expect(nomDe("C:\\Paie\\Octobre\\bulletins.pdf")).toBe("bulletins.pdf");
    expect(dossierDe("C:\\Paie\\Octobre\\bulletins.pdf")).toBe("C:\\Paie\\Octobre");
    expect(dossierDe("/Users/rh/paie.pdf")).toBe("/Users/rh");
    expect(dossierDe("paie.pdf")).toBe("");
  });

  it("garde la barre de la racine", () => {
    expect(dossierDe("C:\\paie.pdf")).toBe("C:\\");
    expect(dossierDe("/paie.pdf")).toBe("/");
  });

  it("propose le nouveau fichier à côté de l'original", () => {
    expect(aCote("C:\\Paie\\Octobre\\bulletins.pdf", "bulletins - compressé.pdf")).toBe("C:\\Paie\\Octobre\\bulletins - compressé.pdf");
    expect(aCote("C:\\bulletins.pdf", "fusion.pdf")).toBe("C:\\fusion.pdf");
    expect(aCote("/Users/rh/paie.pdf", "paie - signé.pdf")).toBe("/Users/rh/paie - signé.pdf");
    expect(aCote(undefined, "fusion.pdf")).toBe("fusion.pdf");
    expect(aCote("paie.pdf", "fusion.pdf")).toBe("fusion.pdf");
  });
});
