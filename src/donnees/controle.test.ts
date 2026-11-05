import { describe, expect, it } from "vitest";
import { controlerListe, emailPlausible, telephonePlausible } from "./controle";
import { nature } from "./correspondances";
import type { Liste } from "./liste";

const colonnes = ["Prénom", "NOM", "Courriel", "GSM", "Tél. bureau", "Service"];
const ligne = (numero: number, ...valeurs: string[]) => ({ numero, valeurs: Object.fromEntries(colonnes.map((c, i) => [c, valeurs[i] ?? ""])) });
const liste: Liste = {
  nomFichier: "agents.xlsx",
  feuille: "Feuil1",
  colonnes,
  lignes: [
    ligne(2, "Aïssatou", "DIALLO", "a.diallo@exemple.com", "+237 612 34 56 78", "", ""),
    ligne(3, "Éloïse", "MARTIN", "eloise.martin@exemple", "06 98 76 54 32", "0222110099", ""),
    ligne(4, "Kouamé", "KOFFI", "k.koffi@exemple.com", "à demander", "", ""),
    ligne(5, " aïssatou ", "Diallo", "a.diallo@exemple.com", "+237 612 34 56 78", "", ""),
    ligne(6, "", "", "contact@exemple.com", "", "", ""),
  ],
};
const balises = ["Prénom", "NOM", "Email", "Mobile", "Fixe", "Direction"];
const correspondances = { Prénom: "Prénom", NOM: "NOM", Email: "Courriel", Mobile: "GSM", Fixe: "Tél. bureau", Direction: "Service" };

describe("contrôle des données de la liste", () => {
  it("reconnaît la nature des informations", () => {
    expect([nature("Courriel"), nature("E-mail pro"), nature("Tél. bureau"), nature("Téléphone portable"), nature("Service")]).toEqual([
      "email",
      "email",
      "fixe",
      "telephone",
      "direction",
    ]);
  });

  it("vérifie les e-mails et les numéros", () => {
    expect(emailPlausible("prenom.nom@exemple.cm")).toBe(true);
    expect(emailPlausible("prenom.nom@exemple")).toBe(false);
    expect(telephonePlausible("+237 6 99 12 34 56")).toBe(true);
    expect(telephonePlausible("(+33) 1.23.45.67.89")).toBe(true);
    expect(telephonePlausible("à demander")).toBe(false);
    expect(telephonePlausible("12 34")).toBe(false);
  });

  it("signale les anomalies avec leurs lignes, les plus importantes d'abord", () => {
    const anomalies = controlerListe(liste, balises, correspondances);
    expect(anomalies.map(({ type, balise, lignes, exemple }) => ({ type, balise, lignes, exemple }))).toEqual([
      { type: "colonne-vide", balise: "Direction", lignes: [], exemple: undefined },
      { type: "doublon", balise: "Prénom NOM", lignes: [2, 5], exemple: "Aïssatou DIALLO" },
      { type: "doublon", balise: "Email", lignes: [2, 5], exemple: "a.diallo@exemple.com" },
      { type: "email", balise: "Email", lignes: [3], exemple: "eloise.martin@exemple" },
      { type: "telephone", balise: "Mobile", lignes: [4], exemple: "à demander" },
      { type: "vide", balise: "Prénom", lignes: [6], exemple: undefined },
      { type: "vide", balise: "NOM", lignes: [6], exemple: undefined },
      { type: "vide", balise: "Mobile", lignes: [6], exemple: undefined },
      { type: "vide", balise: "Fixe", lignes: [2, 4, 5, 6], exemple: undefined },
    ]);
    // Un nom manquant est à corriger ; un fixe manquant masque seulement sa ligne sur la carte.
    expect(anomalies.find((a) => a.type === "vide" && a.balise === "NOM")!.gravite).toBe("attention");
    expect(anomalies.find((a) => a.type === "vide" && a.balise === "Fixe")!.gravite).toBe("info");
  });
});
