import { describe, expect, it } from "vitest";
import {
  annuler,
  appliquer,
  correctionDe,
  estMontant,
  HISTORIQUE_VIDE,
  motsDeLaLigne,
  nomAnnote,
  policeCss,
  pourLeCoeur,
  retablir,
  sans,
  zonesSurlignees,
  type Annotation,
  type MotComplet,
  type MotPlace,
} from "./annotations";

// Deux lignes : « Net à payer : 389 250 » puis « Signature ».
const mot = (x: number, y: number, longueur: number, debutLigne = false): MotPlace => ({ x, y, longueur, taille: 10, debutLigne });
const MOTS = [mot(100, 100, 20, true), mot(124, 100, 8), mot(136, 100, 30), mot(170, 100, 4), mot(178, 100, 18), mot(200, 100, 18), mot(100, 120, 45, true)];

const note = (id: string, texte = "Vu"): Annotation => ({ id, sorte: "note", page: 0, x: 10, y: 10, texte });

describe("surligner en glissant", () => {
  it("surligne les mots parcourus, une zone par ligne", () => {
    // Du milieu de « à » jusque sur « Signature », à la ligne suivante.
    expect(zonesSurlignees(MOTS, { x: 127, y: 105 }, { x: 110, y: 125 })).toEqual([
      { x: 124, y: 100, largeur: 94, hauteur: 10 },
      { x: 100, y: 120, largeur: 45, hauteur: 10 },
    ]);
  });

  it("donne le même résultat dans l'autre sens", () => {
    expect(zonesSurlignees(MOTS, { x: 110, y: 125 }, { x: 127, y: 105 })).toEqual(zonesSurlignees(MOTS, { x: 127, y: 105 }, { x: 110, y: 125 }));
  });

  it("surligne un seul mot sur un simple clic", () => {
    expect(zonesSurlignees(MOTS, { x: 140, y: 104 }, { x: 140, y: 104 })).toEqual([{ x: 136, y: 100, largeur: 30, hauteur: 10 }]);
  });

  it("trace un cadre loin du texte, et rien pour un clic dans le vide", () => {
    expect(zonesSurlignees(MOTS, { x: 300, y: 400 }, { x: 250, y: 380 })).toEqual([{ x: 250, y: 380, largeur: 50, hauteur: 20 }]);
    expect(zonesSurlignees([], { x: 300, y: 400 }, { x: 301, y: 401 })).toEqual([]);
  });
});

describe("annuler et rétablir", () => {
  it("revient en arrière puis en avant, et oublie l'avenir après une nouvelle action", () => {
    const un = appliquer(HISTORIQUE_VIDE, [note("a")]);
    const deux = appliquer(un, [note("a"), note("b")]);
    expect(annuler(deux).present.map((a) => a.id)).toEqual(["a"]);
    expect(retablir(annuler(deux)).present.map((a) => a.id)).toEqual(["a", "b"]);
    const autre = appliquer(annuler(deux), [note("c")]);
    expect(autre.futur).toEqual([]);
    expect(annuler(annuler(autre)).present).toEqual([]);
    expect(annuler(HISTORIQUE_VIDE)).toBe(HISTORIQUE_VIDE);
  });
});

describe("envoi au cœur", () => {
  it("retire les identifiants et laisse de côté les textes vides", () => {
    const annotations: Annotation[] = [
      note("a", "À voir"),
      { id: "b", sorte: "texte", page: 1, x: 5, y: 6, texte: "  ", taille: 12, teinte: "noir" },
      { id: "c", sorte: "surlignage", page: 0, teinte: "jaune", zones: [] },
    ];
    expect(pourLeCoeur(annotations)).toEqual([{ sorte: "note", page: 0, x: 10, y: 10, texte: "À voir" }]);
  });

  it("propose un nom à côté de l'original", () => {
    expect(nomAnnote("Contrat de travail.PDF")).toBe("Contrat de travail - annoté.pdf");
  });
});

describe("gomme", () => {
  it("efface d'un geste toutes les annotations touchées, en une seule étape à annuler", () => {
    const avant = appliquer(HISTORIQUE_VIDE, [note("a"), note("b"), note("c")]);
    const apres = appliquer(avant, sans(avant.present, ["a", "c", "a"]));
    expect(apres.present.map((a) => a.id)).toEqual(["b"]);
    expect(annuler(apres).present.map((a) => a.id)).toEqual(["a", "b", "c"]);
    expect(sans(avant.present, [])).toBe(avant.present);
  });
});

describe("corriger un mot ou un montant", () => {
  const complet = (texte: string, x: number, y: number, longueur: number, debutLigne = false): MotComplet => ({
    texte, x, y, longueur, taille: 10, debutLigne, sens: 0, famille: "Times New Roman", gras: true, italique: false, couleur: [20, 30, 40],
  });
  // « Net à payer : 389 250 FCFA », puis « Signature » à la ligne suivante.
  const LIGNE = [complet("Net", 100, 100, 20, true), complet("à", 124, 100, 8), complet("payer", 136, 100, 30), complet(":", 170, 100, 4), complet("389", 300, 100, 18), complet("250", 322, 100, 18), complet("FCFA", 344, 100, 30), complet("Signature", 100, 120, 45, true)];

  it("choisit le mot cliqué, ou les mots parcourus sur sa seule ligne", () => {
    expect(motsDeLaLigne(LIGNE, { x: 305, y: 104 }, { x: 305, y: 104 })).toEqual([4]);
    expect(motsDeLaLigne(LIGNE, { x: 305, y: 104 }, { x: 330, y: 104 })).toEqual([4, 5]);
    // Glisser jusqu'à la ligne suivante reste sur la ligne du premier mot : jusqu'au mot le plus
    // proche du point d'arrivée (« Net », juste au-dessus).
    expect(motsDeLaLigne(LIGNE, { x: 305, y: 104 }, { x: 120, y: 124 })).toEqual([0, 1, 2, 3, 4]);
    expect(motsDeLaLigne(LIGNE, { x: 250, y: 104 }, { x: 330, y: 104 })).toEqual([]);
  });

  it("reconnaît un montant, qui garde son bord droit", () => {
    for (const montant of ["389 250", "389 250 FCFA", "1 250,50 €", "-17 333", "12,5 %", "€ 40.00", "(1 200)"]) expect(estMontant(montant), montant).toBe(true);
    for (const autre of ["25/11/2026", "Payé", "NKOULOU MBARGA", "Article 3", ""]) expect(estMontant(autre), autre).toBe(false);
  });

  it("prépare la correction avec la place, la police et la couleur de l'ancien texte", () => {
    const correction = correctionDe(LIGNE, [4, 5, 6], 2);
    expect(correction).toMatchObject({
      sorte: "correction",
      page: 2,
      zone: { x: 300, y: 100, largeur: 74, hauteur: 10 },
      x: 300,
      fin: 374,
      ligne: 108,
      ancien: "389 250 FCFA",
      texte: "389 250 FCFA",
      famille: "Times New Roman",
      gras: true,
      couleur: [20, 30, 40],
      droite: true,
    });
    expect(Object.keys(pourLeCoeur([correction])[0])).not.toContain("ancien");
  });

  it("affiche dans une police proche, comme le cœur l'écrit", () => {
    expect(policeCss("Times New Roman")).toContain("Times");
    expect(policeCss("Courier New")).toContain("Courier");
    expect(policeCss("DejaVu Sans")).toContain("Arial");
    expect(policeCss("Calibri")).toContain("Arial");
  });
});
