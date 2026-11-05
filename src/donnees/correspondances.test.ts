import { describe, expect, it } from "vitest";
import { associer, fusionner, normaliser } from "./correspondances";

describe("association des balises aux colonnes", () => {
  it("compare sans accents, casse ni ponctuation", () => {
    expect(normaliser("E-mail Professionnel")).toBe("emailprofessionnel");
    expect(normaliser("Prénom")).toBe(normaliser("PRENOM"));
  });

  it("associe par nom puis par synonyme", () => {
    const balises = ["Prénom", "NOM", "Email", "Mobile", "Fixe", "Fonction"];
    const colonnes = ["Matricule", "PRENOM", "Nom", "Courriel", "GSM", "Tél. bureau", "Poste"];
    expect(associer(balises, colonnes)).toEqual({
      Prénom: "PRENOM",
      NOM: "Nom",
      Email: "Courriel",
      Mobile: "GSM",
      Fixe: "Tél. bureau",
      Fonction: "Poste",
    });
  });

  it("n'associe jamais une colonne à deux balises et laisse l'inconnu à choisir", () => {
    expect(associer(["Email", "Mail", "Signature"], ["E-mail"])).toEqual({ Email: "E-mail", Mail: null, Signature: null });
  });

  it("garde les choix enregistrés tant que leur colonne existe", () => {
    const enregistrees = { NOM: "Nom de famille", Email: null, Fonction: "Intitulé" };
    const colonnes = ["Nom de famille", "Nom", "E-mail", "Poste"];
    expect(fusionner(["NOM", "Email", "Fonction"], colonnes, enregistrees)).toEqual({
      NOM: "Nom de famille", // choix gardé, même si « Nom » correspondrait mieux
      Email: null, // « aucune colonne » est aussi un choix
      Fonction: "Poste", // la colonne choisie n'existe plus : nouvelle association
    });
  });
});
