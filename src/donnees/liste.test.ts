import { describe, expect, it } from "vitest";
import * as XLSX from "xlsx";
import { decoderTexte, lireFeuille, ouvrirClasseur } from "./liste";

/** Classeur « réaliste » : titre au-dessus du tableau, ligne vide, colonne vide, numéros mis en forme. */
function classeurRh(): ArrayBuffer {
  const feuille = XLSX.utils.aoa_to_sheet([
    ["Liste des agents 2026"],
    [],
    ["Matricule", "Prénom", "NOM", "Mobile", "", "E-mail"],
    ["A001", "Jean-Baptiste", "Nkoulou", 699123456, "", "jb@exemple.com"],
    [],
    ["A002", " Aïssatou ", "DIALLO", 612345678, "", ""],
  ]);
  // Numéros stockés comme nombres avec un format personnalisé, comme dans les fichiers RH.
  for (const cellule of ["D4", "D6"]) feuille[cellule].z = '"+237 "000 00 00 00';
  const classeur = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(classeur, feuille, "Agents");
  return XLSX.write(classeur, { type: "array", bookType: "xlsx" });
}

describe("lecture d'un classeur Excel", () => {
  const liste = lireFeuille(ouvrirClasseur(classeurRh(), "agents.xlsx"), "agents.xlsx", "Agents");

  it("trouve l'en-tête sous le titre et ignore la colonne vide", () => {
    expect(liste.colonnes).toEqual(["Matricule", "Prénom", "NOM", "Mobile", "E-mail"]);
  });

  it("saute les lignes vides en gardant les numéros de ligne Excel", () => {
    expect(liste.lignes.map((ligne) => ligne.numero)).toEqual([4, 6]);
  });

  it("garde les numéros tels qu'Excel les affiche", () => {
    expect(liste.lignes[0].valeurs.Mobile).toBe("+237 699 12 34 56");
    expect(liste.lignes[1].valeurs.Mobile).toBe("+237 612 34 56 78");
  });
});

describe("lecture d'un CSV", () => {
  it("lit un CSV enregistré par Excel en français (point-virgule, Windows-1252)", () => {
    const texte = "Prénom;NOM;Mobile\r\nÉloïse;Martin;0698765432\r\n";
    const octets = Uint8Array.from([...texte].map((caractere) => caractere.charCodeAt(0)));
    const classeur = ouvrirClasseur(octets.buffer, "agents.csv");
    const liste = lireFeuille(classeur, "agents.csv", classeur.SheetNames[0]);
    expect(liste.colonnes).toEqual(["Prénom", "NOM", "Mobile"]);
    expect(liste.lignes[0].valeurs).toEqual({ Prénom: "Éloïse", NOM: "Martin", Mobile: "0698765432" });
  });

  it("décode l'UTF-8 et retire la marque d'ordre des octets", () => {
    const octets = new TextEncoder().encode("﻿Nom,Ville\nNguema,Libreville");
    expect(decoderTexte(octets)).toBe("Nom,Ville\nNguema,Libreville");
  });
});
