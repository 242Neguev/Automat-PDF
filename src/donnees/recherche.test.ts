import { describe, expect, it } from "vitest";
import { textes } from "../textes";
import { chercher, mots } from "./recherche";

type Outil = keyof typeof textes.outils;
const OUTILS = Object.keys(textes.outils) as Outil[];
const texte = (cle: Outil) => {
  const { titre, description, motsCles } = textes.outils[cle];
  return `${titre} ${description} ${motsCles}`;
};
const trouver = (recherche: string) => chercher(recherche, OUTILS, texte);

describe("recherche d'une tâche", () => {
  it("découpe en mots sans accents ni majuscules", () => {
    expect(mots("Réduire la TAILLE d'un PDF")).toEqual(["reduire", "la", "taille", "d", "un", "pdf"]);
  });

  it("ne cherche rien sans mot utile", () => {
    expect(trouver("")).toBeNull();
    expect(trouver("  je veux  ")).toBeNull();
  });

  it("trouve l'outil avec les mots de tous les jours", () => {
    expect(trouver("signer")).toEqual(["signature"]);
    expect(trouver("alléger")).toEqual(["compresser"]);
    expect(trouver("réduire la taille")).toEqual(["compresser"]);
    expect(trouver("mot de passe")).toEqual(["motDePasse"]);
    expect(trouver("bulletins de paie")).toEqual(["parPersonne"]);
    expect(trouver("réunir")).toContain("fusionner");
    expect(trouver("numéroter les pages")).toEqual(["marques"]);
    expect(trouver("cacher un nom")).toEqual(["caviarder"]);
    expect(trouver("excel")).toEqual(["officeEnPdf"]);
    expect(trouver("recto verso")).toEqual(["intercaler"]);
  });

  it("accepte un début de mot et un pluriel", () => {
    expect(trouver("sign")).toEqual(["signature"]);
    expect(trouver("signatures")).toEqual(["signature"]);
  });

  it("ne trouve rien pour un mot inconnu", () => {
    expect(trouver("xyzzy")).toEqual([]);
  });
});
