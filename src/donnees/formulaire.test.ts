import { describe, expect, it } from "vitest";
import type { ChampFormulaire, WidgetChamp } from "../coeur";
import { apresClic, coche, initialesDe, libelleDe, remplissable, tailleSaisie, texteDe } from "./formulaire";

const widget = (etat: string | null, plus: Partial<WidgetChamp> = {}): WidgetChamp => ({ page: 0, x: 0, y: 0, largeur: 100, hauteur: 20, etat, taille: 0, alignement: 0, ...plus });

const champ = (plus: Partial<ChampFormulaire>): ChampFormulaire => ({
  nom: "Champ",
  libelle: null,
  sorte: "texte",
  valeur: "",
  lectureSeule: false,
  obligatoire: false,
  multiligne: false,
  secret: false,
  longueurMax: null,
  peigne: false,
  options: [],
  multiple: false,
  saisieLibre: false,
  sansDecocher: false,
  widgets: [widget(null)],
  ...plus,
});

describe("formulaires", () => {
  it("ne propose de remplir que les champs modifiables et affichés", () => {
    expect(remplissable(champ({}))).toBe(true);
    expect(remplissable(champ({ lectureSeule: true }))).toBe(false);
    expect(remplissable(champ({ sorte: "signature" }))).toBe(false);
    expect(remplissable(champ({ sorte: "bouton" }))).toBe(false);
    expect(remplissable(champ({ widgets: [] }))).toBe(false);
  });

  it("coche et décoche une case d'un clic", () => {
    const certifie = champ({ sorte: "case", widgets: [widget("Oui")] });
    const w = certifie.widgets[0];
    expect(coche(w, "Off")).toBe(false);
    expect(apresClic(certifie, w, "Off")).toBe("Oui");
    expect(apresClic(certifie, w, "Oui")).toBe("Off");
  });

  it("choisit un bouton radio, et ne le décoche que si le formulaire le permet", () => {
    const [conges, rtt] = [widget("Conges"), widget("RTT")];
    const libre = champ({ sorte: "radio", widgets: [conges, rtt] });
    expect(apresClic(libre, rtt, "Conges")).toBe("RTT");
    expect(apresClic(libre, rtt, "RTT")).toBe("Off");
    expect(apresClic({ ...libre, sansDecocher: true }, rtt, "RTT")).toBeNull();
    expect(apresClic(libre, widget(null), "Off")).toBeNull();
  });

  it("écrit à la taille que demande le champ, ou à celle que choisira le cœur", () => {
    expect(tailleSaisie(champ({}), widget(null, { taille: 11 }))).toBe(11);
    expect(tailleSaisie(champ({}), widget(null, { hauteur: 20 }))).toBeCloseTo(12.8);
    expect(tailleSaisie(champ({ multiligne: true }), widget(null, { hauteur: 80 }))).toBe(12);
    expect(tailleSaisie(champ({}), widget(null, { hauteur: 6 }))).toBe(4);
  });

  it("montre le libellé des options choisies, et les valeurs reçues", () => {
    const service = champ({ sorte: "combo", options: [{ valeur: "INF", libelle: "Informatique" }, { valeur: "RH", libelle: "Ressources humaines" }] });
    expect(libelleDe(service, "INF")).toBe("Informatique");
    expect(libelleDe(service, ["RH", "Autre"])).toBe("Ressources humaines, Autre");
    expect(texteDe(["a", "b"])).toBe("a");
    expect(texteDe(undefined)).toBe("");
    expect(initialesDe({ champs: [champ({ nom: "Nom", valeur: "NKOULOU" })], xfa: false, xfaSeul: false, calculs: false })).toEqual({ Nom: "NKOULOU" });
    expect(initialesDe(null)).toEqual({});
  });
});
