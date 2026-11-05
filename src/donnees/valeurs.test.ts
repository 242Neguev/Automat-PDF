import { describe, expect, it } from "vitest";
import { associer } from "./correspondances";
import { nettoyer } from "./nettoyage";
import { balisesDe, remplir, transformationsParDefaut, valeursDeLigne } from "./valeurs";

describe("nettoyage des valeurs", () => {
  it("met en forme noms et e-mails", () => {
    expect(nettoyer("  jean-baptiste ", "nom-propre")).toBe("Jean-Baptiste");
    expect(nettoyer("ÉLOÏSE", "nom-propre")).toBe("Éloïse");
    expect(nettoyer("Nkoulou   Mbarga", "majuscules")).toBe("NKOULOU MBARGA");
    expect(nettoyer("A.DIALLO@EXEMPLE.COM", "minuscules")).toBe("a.diallo@exemple.com");
    expect(nettoyer(undefined)).toBe("");
  });

  it("déduit les mises en forme du nom des balises", () => {
    expect(transformationsParDefaut(["NOM", "Prénom", "E-mail", "Fonction"])).toEqual({ NOM: "majuscules", Prénom: "nom-propre", "E-mail": "minuscules" });
  });
});

describe("remplissage des noms de fichiers", () => {
  it("vide le nom quand toutes ses balises sont vides", () => {
    expect(remplir("Bulletin {{Matricule}}", { Matricule: "" })).toBe("");
    expect(remplir("Bulletin {{Matricule}}", { Matricule: "A001" })).toBe("Bulletin A001");
  });

  it("ne laisse pas d'espace parasite quand une partie manque", () => {
    expect(remplir("{{Prénom}} {{NOM}}", { Prénom: "", NOM: "DIALLO" })).toBe("DIALLO");
  });

  it("prend les valeurs d'une ligne de la liste, mises en forme", () => {
    const balises = balisesDe("{{Prénom}} {{NOM}} - {{Prénom}}");
    expect(balises).toEqual(["Prénom", "NOM"]);
    const correspondances = associer(balises, ["Matricule", "Prénom", "Nom"]);
    const ligne = { numero: 2, valeurs: { Matricule: "A001", Prénom: " jean-baptiste", Nom: "Nkoulou Mbarga" } };
    const valeurs = valeursDeLigne({ transformations: transformationsParDefaut(balises) }, ligne, correspondances);
    expect(remplir("{{Prénom}} {{NOM}}", valeurs)).toBe("Jean-Baptiste NKOULOU MBARGA");
  });
});
