import { describe, expect, it } from "vitest";
import { avecEnchainement, lireEnchainements, REGLAGES_PAR_DEFAUT, sansEnchainement, type Enchainement } from "./enchainements";

const paie: Enchainement = {
  ...REGLAGES_PAR_DEFAUT,
  nom: "Bulletins de paie",
  proteger: true,
  envoi: { ...REGLAGES_PAR_DEFAUT.envoi, actif: true, mode: "direct" },
  liste: { chemin: "C:\\RH\\personnel.xlsx", feuille: "Agents", correspondances: { Matricule: "Matricule", "Mot de passe": "Date de naissance", "E-mail": null } },
};

describe("enchaînements", () => {
  it("se relisent tels qu'enregistrés", () => {
    expect(lireEnchainements(JSON.stringify([paie]))).toEqual([paie]);
    expect(lireEnchainements("")).toEqual([]);
  });

  it("complètent un enregistrement ancien et écartent ce qui est illisible", () => {
    const ancien = { nom: " Attestations ", mode: "repere", suite: "oui", envoi: { actif: true }, liste: { chemin: 12 } };
    const [lu] = lireEnchainements(JSON.stringify([ancien, { mode: "liste" }, "texte", null]));
    expect(lu.nom).toBe("Attestations");
    expect(lu.mode).toBe("repere");
    expect(lu.suite).toBe(false);
    expect(lu.envoi).toEqual({ ...REGLAGES_PAR_DEFAUT.envoi, actif: true });
    expect(lu.liste).toBeNull();
    expect(lireEnchainements("{pas du json")).toEqual([]);
    expect(lireEnchainements('{"nom":"seul"}')).toEqual([]);
  });

  it("remplacent celui du même nom, à sa place", () => {
    const autre = { ...paie, nom: "Attestations" };
    const modifie = { ...paie, nom: "bulletins de paie", suite: true };
    expect(avecEnchainement([paie, autre], modifie)).toEqual([modifie, autre]);
    expect(avecEnchainement([autre], paie)).toEqual([autre, paie]);
    expect(sansEnchainement([paie, autre], "BULLETINS DE PAIE")).toEqual([autre]);
  });
});
