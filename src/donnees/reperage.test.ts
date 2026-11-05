import { describe, expect, it } from "vitest";
import { attribuerParListe, attribuerParRepere, motsDe, PageLue, pagesAVerifier, pagesParPersonne, valeurApres, type Personne } from "./reperage";

// Texte tel que l'extrait le cœur Rust d'un PDF de paie (colonnes séparées par des tabulations).
// Pièges : la prime de transport vaut « 00131 » (le matricule de Mme Moukoko) et l'ancienneté
// « 124 » (le matricule 00124, tel qu'Excel l'affiche) sur tous les bulletins.
const bulletin = (matricule: string, nom: string, net: string, page = "1/1") =>
  `EAU SÈCHE SARL\tBULLETIN DE PAIE\nBrazzaville\tPériode : octobre 2026 · Page ${page}\nMatricule : ${matricule}\tNom : ${nom}\n` +
  `Rubrique\tBase\tTaux\tMontant\nSalaire de base\t173,33\t2 500\t433 325\nPrime de transport\t00131\nAncienneté\t124\t3 %\t12 999\n` +
  `Net à payer : ${net} FCFA\n`;
const suiteSansMatricule = "EAU SÈCHE SARL\tBULLETIN DE PAIE\nBrazzaville\tPériode : octobre 2026 · Page 2/2\nSuite du bulletin\nCongés payés : 12 jours acquis.\nNet à payer : 456 780 FCFA\n";
const recapitulatif =
  "Récapitulatif de la paie — octobre 2026\nMatricule\tNom\tNet\n00123\tNKOULOU MBARGA Jean-Baptiste\t389 250\n00124\tDIALLO Aïssatou\t512 000\n" +
  "00131\tMOUKOKO Éloïse\t245 600\n00140\tDIALLO Marc\t131 000\n00152\tOKEMBA Grâce\t456 780\n00160\tNGOMA Paul\t124 500\n";
const PAGES = [
  bulletin("00123", "NKOULOU MBARGA Jean-Baptiste", "389 250"),
  bulletin("00124", "DIALLO Aïssatou", "512 000"),
  bulletin("00131", "MOUKOKO Éloïse", "245 600", "1/2"),
  bulletin("00131", "MOUKOKO Éloïse", "245 600", "2/2"),
  bulletin("00140", "DIALLO Marc", "131 000"),
  bulletin("00152", "OKEMBA Grâce", "456 780", "1/2"),
  suiteSansMatricule,
  bulletin("00160", "NGOMA Paul", "124 500"),
  recapitulatif,
];
// Liste : Matricule, Prénom, NOM, Date de naissance, E-mail.
const LISTE = [
  ["00123", "Jean-Baptiste", "NKOULOU MBARGA", "12/04/1980", "jb.nkoulou@exemple.com"],
  ["124", "Aïssatou", "DIALLO", "03/11/1987", "a.diallo@exemple.com"],
  ["00131", "Éloïse", "MOUKOKO", "21/06/1992", "e.moukoko@exemple.com"],
  ["00140", "Marc", "DIALLO", "30/01/1975", "m.diallo@exemple.com"],
  ["00152", "Grâce", "OKEMBA", "08/09/1990", "g.okemba@exemple.com"],
  ["00160", "Paul", "NGOMA", "17/02/1983", ""],
];
const parMatricule: Personne[] = LISTE.map((ligne) => ({ identifiant: ligne[0], informations: ligne }));

describe("mots et recherche", () => {
  it("compare sans accents ni majuscules, et retrouve les matricules qu'Excel a privés de leurs zéros", () => {
    expect(motsDe("Aïssatou DIALLO, n° 00124")).toEqual(["aissatou", "diallo", "n", "00124"]);
    const page = new PageLue("Matricule : 00124\tNet à payer : 131 000 FCFA");
    expect(page.contient(motsDe("124"))).toBe(true);
    expect(page.contient(motsDe("00124"))).toBe(true);
    // « 00131 » n'est pas le montant « 131 000 », ni « 0124 » le matricule « 00124 » écrit en entier.
    expect(page.contient(motsDe("00131"))).toBe(false);
    expect(page.contient(motsDe("131"))).toBe(true);
    expect(page.contient(motsDe("net a payer"))).toBe(true);
    expect(page.contient(motsDe("payer net"))).toBe(false);
  });
});

describe("attribution d'après la liste", () => {
  it("départage les candidats par les autres informations de la personne, et met de côté le récapitulatif", () => {
    const etats = attribuerParListe(PAGES, parMatricule, false);
    expect(etats.slice(0, 6)).toEqual([0, 1, 2, 2, 3, 4].map((personne) => ({ etat: "attribuee", personne })));
    expect(etats[6]).toEqual({ etat: "sans-personne" });
    expect(etats[7]).toEqual({ etat: "attribuee", personne: 5 });
    expect(etats[8]).toEqual({ etat: "ambigue", personnes: [0, 1, 2, 3, 4, 5] });
    expect(pagesParPersonne(etats)).toEqual([
      { personne: 0, pages: [0] },
      { personne: 1, pages: [1] },
      { personne: 2, pages: [2, 3] },
      { personne: 3, pages: [4] },
      { personne: 4, pages: [5] },
      { personne: 5, pages: [7] },
    ]);
    expect(pagesAVerifier(etats)).toEqual([6, 8]);
  });

  it("rattache à la précédente une page sans identifiant, si on le demande", () => {
    const etats = attribuerParListe(PAGES, parMatricule, true);
    expect(etats[6]).toEqual({ etat: "suite", personne: 4 });
    expect(pagesParPersonne(etats)[4]).toEqual({ personne: 4, pages: [5, 6] });
    // Le récapitulatif reste à vérifier.
    expect(pagesAVerifier(etats)).toEqual([8]);
  });

  it("n'attribue ni ne rattache à personne un récapitulatif qui cite plusieurs personnes", () => {
    // Sans matricules : « 124 » (Mme Diallo, d'après la liste) figure dans le net « 124 500 » de M. Ngoma.
    const sansMatricules = recapitulatif.replace(/\d{5}\t/g, "");
    const etats = attribuerParListe([PAGES[7], sansMatricules, suiteSansMatricule], parMatricule, true);
    expect(etats[0]).toEqual({ etat: "attribuee", personne: 5 });
    expect(etats[1]).toEqual({ etat: "ambigue", personnes: [1, 0, 2, 3, 4, 5] });
    expect(etats[2]).toEqual({ etat: "sans-personne" });
    // Une page de suite qui ne cite que la personne, ou son seul nom de famille partagé, est rattachée.
    const suiteDeMarc = "Suite du bulletin de DIALLO Marc\nCongés payés : 12 jours acquis.\n";
    expect(attribuerParListe([PAGES[4], suiteDeMarc], parMatricule, true)[1]).toEqual({ etat: "suite", personne: 3 });
  });

  it("ne tient pas compte d'une colonne que beaucoup partagent (site, service)", () => {
    // « Brazzaville », site de tous sauf M. Ngoma, figure dans l'adresse de l'entreprise.
    const avecSite = LISTE.map((ligne, i) => ({ identifiant: ligne[0], informations: [...ligne, i === 5 ? "Pointe-Noire" : "Brazzaville"] }));
    expect(attribuerParListe([PAGES[7]], avecSite, false)).toEqual([{ etat: "attribuee", personne: 5 }]);
  });

  it("met de côté la page d'un inconnu où figure par hasard le matricule de quelqu'un", () => {
    const inconnu = "BULLETIN DE PAIE\nMatricule : 00999\tNom : KOUMBA Rose\nPrime de transport\t00131\nNet à payer : 98 000 FCFA\n";
    expect(attribuerParListe([inconnu, ...PAGES], parMatricule, false)[0]).toEqual({ etat: "douteuse", personnes: [2] });
  });

  it("se fie à l'identifiant seul quand aucune autre colonne de la liste ne figure dans le PDF", () => {
    const seulement = LISTE.map((ligne) => ({ identifiant: ligne[0], informations: [ligne[0], ligne[4]] }));
    const etats = attribuerParListe([bulletin("00152", "OKEMBA Grâce", "456 780")], seulement, false);
    expect(etats).toEqual([{ etat: "ambigue", personnes: [1, 2, 4] }]);
    const sansPiege = "Matricule : 00152\tNom : OKEMBA Grâce\nNet à payer : 456 780 FCFA\n";
    expect(attribuerParListe([sansPiege], seulement, false)).toEqual([{ etat: "attribuee", personne: 4 }]);
  });

  it("reconnaît une personne à son nom, et signale une page sans texte", () => {
    const parNom = LISTE.map((ligne) => ({ identifiant: `${ligne[2]} ${ligne[1]}`, informations: ligne }));
    const etats = attribuerParListe([PAGES[4], ""], parNom, false);
    expect(etats).toEqual([{ etat: "attribuee", personne: 3 }, { etat: "sans-texte" }]);
  });
});

describe("attribution d'après un repère", () => {
  it("lit la valeur à droite du repère, ou dessous dans un tableau à en-têtes", () => {
    expect(valeurApres(PAGES[0], "Matricule", "droite")).toBe("00123");
    expect(valeurApres(PAGES[0], "nom :", "droite")).toBe("NKOULOU");
    expect(valeurApres(recapitulatif, "Matricule", "dessous")).toBe("00123");
    expect(valeurApres(recapitulatif, "Net", "dessous")).toBe("389");
    expect(valeurApres(suiteSansMatricule, "Matricule", "droite")).toBeNull();
    expect(valeurApres("N° salarié : E-1234.", "N° salarié", "droite")).toBe("E-1234");
  });

  it("regroupe les pages qui portent la même valeur", () => {
    const { etats, valeurs } = attribuerParRepere(PAGES, "Matricule :", "droite", true);
    expect(valeurs).toEqual(["00123", "00124", "00131", "00140", "00152", "00160"]);
    expect(pagesParPersonne(etats).map((groupe) => groupe.pages)).toEqual([[0], [1], [2, 3], [4], [5, 6], [7]]);
    // « Matricule : » n'est pas l'en-tête de colonne « Matricule » du récapitulatif.
    expect(pagesAVerifier(etats)).toEqual([8]);
    expect(attribuerParRepere(PAGES, "Matricule", "droite", true).valeurs).toContain("Nom");
  });
});
