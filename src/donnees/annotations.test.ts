import { describe, expect, it } from "vitest";
import {
  annuler,
  appliquer,
  correctionDe,
  estMontant,
  extraitSurligne,
  HISTORIQUE_VIDE,
  motsDeLaLigne,
  memeValeur,
  nomAnnote,
  nombreDeModifications,
  nomRempli,
  nomSigne,
  ordreDeLecture,
  policeCss,
  pourLeCoeur,
  retablir,
  sans,
  zonesSurlignees,
  dateDuJour,
  prolonger,
  retouchesDe,
  sorteDEnregistrement,
  TAMPONS,
  zoneDe,
  zoneTampon,
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

describe("commentaires : souligner, barrer, formes, dessin, tampons", () => {
  it("n'envoie au cœur ni trait vide, ni forme sans longueur", () => {
    const debut = { x: 10, y: 10 };
    const annotations: Annotation[] = [
      { id: "a", sorte: "soulignage", page: 0, teinte: "rouge", zones: [] },
      { id: "b", sorte: "barre", page: 0, teinte: "rouge", zones: [{ x: 1, y: 2, largeur: 3, hauteur: 4 }] },
      { id: "c", sorte: "forme", page: 0, forme: "fleche", debut, fin: { x: 10.2, y: 10.3 }, teinte: "rouge", epaisseur: 2 },
      { id: "d", sorte: "forme", page: 0, forme: "rectangle", debut, fin: { x: 60, y: 40 }, teinte: "rouge", epaisseur: 2 },
      { id: "e", sorte: "dessin", page: 0, points: [debut], teinte: "noir", epaisseur: 1 },
      { id: "f", sorte: "tampon", page: 1, zone: { x: 0, y: 0, largeur: 110, hauteur: 54 }, mot: "paye", date: "25/11/2026" },
    ];
    expect(pourLeCoeur(annotations).map((a) => a.sorte)).toEqual(["barre", "forme", "tampon"]);
    expect(pourLeCoeur(annotations)[2]).toEqual({ sorte: "tampon", page: 1, zone: { x: 0, y: 0, largeur: 110, hauteur: 54 }, mot: "paye", date: "25/11/2026" });
  });

  it("pose un tampon assez large pour son mot, centré sur le clic, daté du jour", () => {
    expect(zoneTampon("vu", { x: 200, y: 100 })).toEqual({ x: 145, y: 73, largeur: 110, hauteur: 54 });
    const confidentiel = zoneTampon("confidentiel", { x: 300, y: 300 });
    expect(confidentiel.largeur).toBe(TAMPONS.confidentiel.texte.length * 17 + 40);
    expect(confidentiel.x + confidentiel.largeur / 2).toBe(300);
    expect(dateDuJour(new Date(2026, 10, 5))).toBe("05/11/2026");
  });

  it("garde le texte souligné ou barré, rien pour un cadre tracé", () => {
    const complet = (texte: string, x: number, y: number, longueur: number, debutLigne = false): MotComplet => ({
      texte, x, y, longueur, taille: 10, debutLigne, sens: 0, famille: "Helvetica", gras: false, italique: false, couleur: [0, 0, 0],
    });
    const mots = [complet("Délai", 100, 100, 30, true), complet("de", 134, 100, 12), complet("paiement", 150, 100, 50), complet("30", 100, 120, 12, true), complet("jours", 116, 120, 30)];
    // De « de » jusqu'à « 30 », à la ligne suivante, dans l'ordre de lecture même en remontant.
    expect(extraitSurligne(mots, { x: 138, y: 104 }, { x: 104, y: 124 })).toBe("de paiement 30");
    expect(extraitSurligne(mots, { x: 104, y: 124 }, { x: 138, y: 104 })).toBe("de paiement 30");
    expect(extraitSurligne(mots, { x: 400, y: 400 }, { x: 450, y: 450 })).toBe("");
  });

  it("allège le trait d'un dessin sans perdre sa forme", () => {
    let trait = prolonger([], { x: 0, y: 0 });
    trait = prolonger(trait, { x: 0.2, y: 0.1 });
    trait = prolonger(trait, { x: 3, y: 4 });
    expect(trait).toEqual([{ x: 0, y: 0 }, { x: 3, y: 4 }]);
  });

  it("tient les retouches des commentaires du PDF, sans les envoyer comme annotations", () => {
    const annotations: Annotation[] = [
      { id: "a", sorte: "retouche", page: 0, cible: "12 0", texte: "Vu" },
      { id: "b", sorte: "suppression", page: 1, cible: "p1-3" },
      { id: "c", sorte: "retouche", page: 0, cible: "9 0", texte: "Premier" },
      { id: "d", sorte: "retouche", page: 0, cible: "9 0", texte: "Second" },
      { id: "e", sorte: "retouche", page: 1, cible: "p1-3", texte: "Perdu" },
      note("f"),
    ];
    expect(retouchesDe(annotations)).toEqual({ supprimes: ["p1-3"], textes: [["12 0", "Vu"], ["9 0", "Second"]], champs: [] });
    expect(retouchesDe([])).toEqual({ supprimes: [], textes: [], champs: [] });
    expect(pourLeCoeur(annotations).map((a) => a.sorte)).toEqual(["note"]);
  });

  it("tient les champs remplis : la dernière valeur de chacun, sauf revenue à celle du PDF", () => {
    const champ = (id: string, nom: string, valeur: string | string[]): Annotation => ({ id, sorte: "champ", page: 0, nom, valeur });
    const annotations: Annotation[] = [
      champ("a", "Nom", "NKOULOU"),
      champ("b", "Type", "RTT"),
      champ("c", "Nom", "MOUKOKO"),
      champ("d", "Certifie", "Oui"),
      champ("e", "Certifie", "Off"),
      champ("f", "Langues", ["en", "fr"]),
      note("g"),
    ];
    // « Certifie » est revenu à sa valeur reçue ; « Langues » aussi, dans un autre ordre.
    const initiales = { Nom: "", Type: "Off", Certifie: "Off", Langues: ["fr", "en"] };
    expect(retouchesDe(annotations, initiales).champs).toEqual([
      ["Nom", "MOUKOKO"],
      ["Type", "RTT"],
    ]);
    expect(pourLeCoeur(annotations).map((a) => a.sorte)).toEqual(["note"]);
    // Les modifications : la note, et chaque champ une fois.
    expect(nombreDeModifications(annotations)).toBe(5);
    expect(memeValeur("", [])).toBe(true);
    expect(memeValeur(["a"], "a")).toBe(true);
    expect(memeValeur("Off", "Oui")).toBe(false);
    expect(nomRempli("Demande de congés.pdf")).toBe("Demande de congés - rempli.pdf");
  });

  it("nomme le PDF enregistré d'après ce qui a été fait : signé, rempli ou annoté", () => {
    const signature = { id: "s", sorte: "signature" as const, page: 0, zone: { x: 0, y: 0, largeur: 120, hauteur: 40 }, trace: { proportion: 3, traits: [[{ x: 0, y: 0 }]], epaisseur: 0.05 }, teinte: "marine" as const, initiales: false };
    const champ: Annotation = { id: "c", sorte: "champ", page: 0, nom: "Nom", valeur: "NKOULOU" };
    expect(sorteDEnregistrement([signature])).toBe("signe");
    expect(sorteDEnregistrement([champ, signature])).toBe("rempli");
    expect(sorteDEnregistrement([champ, note("n")])).toBe("annote");
    expect(sorteDEnregistrement([])).toBe("annote");
    expect(nomSigne("Contrat.pdf")).toBe("Contrat - signé.pdf");
    // Une signature vide ne part pas au cœur.
    expect(pourLeCoeur([{ ...signature, trace: { ...signature.trace, traits: [] } }])).toEqual([]);
  });

  it("liste les commentaires dans l'ordre de lecture", () => {
    const ici = (id: string, page: number, x: number, y: number): Annotation => ({ id, sorte: "note", page, x, y, texte: id });
    const liste = [ici("bas", 0, 10, 500), ici("page2", 1, 0, 0), ici("droite", 0, 300, 102), ici("gauche", 0, 40, 100)];
    expect(ordreDeLecture(liste).map((a) => a.id)).toEqual(["gauche", "droite", "bas", "page2"]);
  });

  it("retrouve la zone de chaque annotation", () => {
    expect(zoneDe({ id: "a", sorte: "forme", page: 0, forme: "ligne", debut: { x: 50, y: 80 }, fin: { x: 10, y: 20 }, teinte: "noir", epaisseur: 1 })).toEqual({ x: 10, y: 20, largeur: 40, hauteur: 60 });
    expect(zoneDe({ id: "b", sorte: "dessin", page: 0, points: [{ x: 5, y: 9 }, { x: 1, y: 3 }, { x: 4, y: 12 }], teinte: "noir", epaisseur: 1 })).toEqual({ x: 1, y: 3, largeur: 4, hauteur: 9 });
  });
});
