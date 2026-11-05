// À qui appartient chaque page d'un PDF (bulletins de paie, attestations…), d'après son texte :
// l'identifiant de chacun dans la liste (matricule, nom), ou la valeur qui suit un repère imprimé
// sur la page (« Matricule : 00123 »). Dans le doute, une page n'est attribuée à personne : mieux
// vaut la vérifier que l'envoyer à la mauvaise personne.

const MOT = /[\p{L}\p{N}]+/gu;
const NOMBRE = /^\d+$/;

const simplifier = (mot: string) => mot.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();

/** Mots comparables d'un texte (sans accents ni majuscules), dans l'ordre. */
export const motsDe = (texte: string) => [...texte.normalize("NFC").matchAll(MOT)].map((m) => simplifier(m[0]));

/** Clé de recherche d'un mot : un nombre sans ses zéros de tête. */
const cleDe = (mot: string) => (NOMBRE.test(mot) ? mot.replace(/^0+(?=\d)/, "") : mot);

/**
 * Le mot de la page correspond au mot cherché : le même, ou le même nombre écrit avec des zéros
 * en tête (Excel affiche « 124 » le matricule « 00124 » du bulletin). Jamais l'inverse : le
 * matricule « 00131 » ne doit pas être reconnu dans le montant « 131 000 ».
 */
function correspond(page: string, cherche: string): boolean {
  return page === cherche || (NOMBRE.test(page) && NOMBRE.test(cherche) && page.length > cherche.length && cleDe(page) === cleDe(cherche));
}

/** Texte d'une page, prêt pour y chercher des suites de mots. */
export class PageLue {
  readonly mots: string[];
  private readonly index = new Map<string, number[]>();

  constructor(texte: string) {
    this.mots = motsDe(texte);
    this.mots.forEach((mot, i) => {
      const cle = cleDe(mot);
      const positions = this.index.get(cle);
      if (positions) positions.push(i);
      else this.index.set(cle, [i]);
    });
  }

  /** La suite de mots figure sur la page, d'un seul tenant. */
  contient(cherches: string[]): boolean {
    if (cherches.length === 0) return false;
    return (this.index.get(cleDe(cherches[0])) ?? []).some((debut) =>
      cherches.every((mot, j) => debut + j < this.mots.length && correspond(this.mots[debut + j], mot)),
    );
  }
}

/** Ce que l'on sait d'une personne de la liste. */
export interface Personne {
  /** Ce qui l'identifie sur la page : matricule, nom… */
  identifiant: string;
  /** Ses autres informations, une par colonne de la liste (dans le même ordre pour tous). */
  informations: string[];
}

/** Ce que l'on a conclu pour une page ; `personne` est un rang dans la liste (ou dans les valeurs). */
export type EtatPage =
  | { etat: "attribuee"; personne: number }
  /** Page sans identifiant, rattachée à la précédente (bulletin sur plusieurs pages). */
  | { etat: "suite"; personne: number }
  | { etat: "sans-texte" }
  | { etat: "sans-personne" }
  /** Un seul identifiant trouvé, mais aucune autre information de la personne sur la page. */
  | { etat: "douteuse"; personnes: number[] }
  /** Plusieurs personnes possibles, impossibles à départager. */
  | { etat: "ambigue"; personnes: number[] };

/**
 * Pages sans identifiant rattachées à la page précédente, quand elle appartient à quelqu'un et
 * qu'elles ne mentionnent personne d'autre : un récapitulatif de la paie ne doit pas finir dans
 * le bulletin du dernier salarié.
 */
function rattacher(etats: EtatPage[], mentionneAutrui: (page: number, personne: number) => boolean): EtatPage[] {
  const resultat: EtatPage[] = [];
  etats.forEach((etat, page) => {
    const precedent = resultat[resultat.length - 1];
    if (etat.etat === "sans-personne" && precedent && (precedent.etat === "attribuee" || precedent.etat === "suite") && !mentionneAutrui(page, precedent.personne)) {
      resultat.push({ etat: "suite", personne: precedent.personne });
    } else resultat.push(etat);
  });
  return resultat;
}

/**
 * Attribue chaque page à la personne dont l'identifiant y figure. Une autre information de la
 * personne (nom, prénom, fonction…) doit aussi y figurer pour confirmer, ou pour départager
 * plusieurs candidats ; seules comptent les colonnes présentes quelque part dans le PDF (l'e-mail,
 * qui n'y est jamais imprimé, ne peut rien confirmer). Une page qui cite aussi quelqu'un d'autre
 * (un récapitulatif de la paie) n'est attribuée à personne.
 */
export function attribuerParListe(textes: string[], personnes: Personne[], suite: boolean): EtatPage[] {
  const pages = textes.map((texte) => new PageLue(texte));
  const identifiants = personnes.map((personne) => motsDe(personne.identifiant));
  const colonnes = Math.max(0, ...personnes.map((personne) => personne.informations.length));
  // Informations de chacun qui peuvent confirmer : assez longues, et pas déjà dans l'identifiant.
  const preuves = personnes.map((personne, p) =>
    personne.informations.map((information) => {
      const mots = motsDe(information);
      const dejaDansIdentifiant = new PageLue(identifiants[p].join(" ")).contient(mots);
      return mots.join("").length >= 3 && !dejaDansIdentifiant ? mots : null;
    }),
  );
  const deLaColonne = (c: number) => preuves.flatMap((parColonne) => (parColonne[c] ? [parColonne[c]] : []));
  const utiles = Array.from({ length: colonnes }, (_, c) => deLaColonne(c).some((mots) => pages.some((page) => page.contient(mots))));
  // Colonnes qui distinguent les personnes (nom, prénom…), à la différence d'un service ou d'un
  // site que beaucoup partagent.
  const distinctives = Array.from({ length: colonnes }, (_, c) => {
    const valeurs = deLaColonne(c).map((mots) => mots.join(" "));
    return valeurs.length > 0 && new Set(valeurs).size >= 0.8 * valeurs.length;
  });
  const confirmations = (p: number, page: PageLue) => preuves[p].filter((mots, c) => utiles[c] && mots !== null && page.contient(mots)).length;
  const peutConfirmer = (p: number) => preuves[p].some((mots, c) => utiles[c] && mots !== null);
  // Un nombre court (« 124 ») se retrouve dans les montants : il ne prouve pas qu'une page cite
  // quelqu'un, à la différence d'un nom ou d'un long numéro (CNSS, téléphone).
  const parlante = (mots: string[]) => mots.some((mot) => /\p{L}/u.test(mot)) || mots.join("").length >= 6;
  /** Autres personnes que la page cite : une information distinctive d'elles qui n'est pas aussi celle de `p` (même nom de famille…). */
  const autresCites = (page: PageLue, p: number) => {
    const siennes = new Set(preuves[p].flatMap((mots) => (mots ? [mots.join(" ")] : [])));
    return preuves.flatMap((parColonne, autre) =>
      autre !== p &&
      parColonne.some((mots, c) => utiles[c] && distinctives[c] && mots !== null && parlante(mots) && !siennes.has(mots.join(" ")) && page.contient(mots))
        ? [autre]
        : [],
    );
  };
  const attribuer = (page: PageLue, p: number): EtatPage => {
    const autres = autresCites(page, p);
    return autres.length === 0 ? { etat: "attribuee", personne: p } : { etat: "ambigue", personnes: [p, ...autres] };
  };

  const etats = pages.map((page): EtatPage => {
    if (page.mots.length === 0) return { etat: "sans-texte" };
    const candidats = identifiants.flatMap((mots, p) => (page.contient(mots) ? [p] : []));
    if (candidats.length === 0) return { etat: "sans-personne" };
    const scores = candidats.map((p) => confirmations(p, page));
    if (candidats.length === 1) {
      const [p] = candidats;
      return scores[0] > 0 || !peutConfirmer(p) ? attribuer(page, p) : { etat: "douteuse", personnes: candidats };
    }
    const meilleur = Math.max(...scores);
    const premiers = candidats.filter((_, i) => scores[i] === meilleur);
    if (meilleur > 0 && premiers.length === 1) return attribuer(page, premiers[0]);
    return { etat: "ambigue", personnes: meilleur > 0 ? premiers : candidats };
  });
  return suite ? rattacher(etats, (page, personne) => autresCites(pages[page], personne).length > 0) : etats;
}

/** Où se trouve la valeur, par rapport au repère : à sa droite, ou dessous (tableau à en-têtes). */
export type PositionValeur = "droite" | "dessous";

/** Premier mot d'un texte (« E-1234 », « 00123 »), sans la ponctuation qui l'entoure. */
function premierMot(texte: string): string | null {
  for (const morceau of texte.split(/\s+/)) {
    const propre = morceau.replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, "");
    if (propre) return propre;
  }
  return null;
}

/**
 * Valeur qui suit le repère sur la page (« Matricule : 00123 » → « 00123 »), ou null. La
 * ponctuation tapée à la fin du repère doit le suivre sur la page : « Matricule : » écarte l'en-tête
 * de colonne « Matricule » d'un tableau récapitulatif.
 */
export function valeurApres(texte: string, repere: string, position: PositionValeur): string | null {
  const cherches = motsDe(repere);
  if (cherches.length === 0) return null;
  const ponctuation = (repere.trim().match(/[^\p{L}\p{N}]+$/u)?.[0] ?? "").replace(/\s/g, "");
  const lignes = texte.normalize("NFC").split("\n");
  for (let n = 0; n < lignes.length; n++) {
    const ligne = lignes[n];
    const mots = [...ligne.matchAll(MOT)];
    for (let debut = 0; debut + cherches.length <= mots.length; debut++) {
      if (!cherches.every((mot, j) => correspond(simplifier(mots[debut + j][0]), mot))) continue;
      const dernier = mots[debut + cherches.length - 1];
      const apres = ligne.slice((dernier.index ?? 0) + dernier[0].length);
      if (!apres.replace(/\s/g, "").startsWith(ponctuation)) continue;
      const valeur =
        position === "droite"
          ? premierMot(apres)
          : premierMot(
              lignes
                .slice(n + 1)
                .find((suivante) => suivante.trim())
                ?.split("\t")[ligne.slice(0, mots[debut].index).split("\t").length - 1] ?? "",
            );
      if (valeur) return valeur;
    }
  }
  return null;
}

/**
 * Valeur du repère sur chaque page ; les pages qui portent la même valeur vont ensemble. Les
 * valeurs sont rangées dans l'ordre de leur première page.
 */
export function attribuerParRepere(textes: string[], repere: string, position: PositionValeur, suite: boolean): { etats: EtatPage[]; valeurs: string[] } {
  const valeurs: string[] = [];
  const rangs = new Map<string, number>();
  const etats = textes.map((texte): EtatPage => {
    if (motsDe(texte).length === 0) return { etat: "sans-texte" };
    const valeur = valeurApres(texte, repere, position);
    if (!valeur) return { etat: "sans-personne" };
    const cle = motsDe(valeur).map(cleDe).join(" ");
    let rang = rangs.get(cle);
    if (rang === undefined) {
      rang = valeurs.push(valeur) - 1;
      rangs.set(cle, rang);
    }
    return { etat: "attribuee", personne: rang };
  });
  if (!suite) return { etats, valeurs };
  const recherches = valeurs.map(motsDe);
  const mentionneAutrui = (page: number, personne: number) => {
    const lue = new PageLue(textes[page]);
    return recherches.some((mots, autre) => autre !== personne && lue.contient(mots));
  };
  return { etats: rattacher(etats, mentionneAutrui), valeurs };
}

/** Pages de chacun (numérotées à partir de 0), chacun dans l'ordre de sa première page. */
export function pagesParPersonne(etats: EtatPage[]): { personne: number; pages: number[] }[] {
  const groupes = new Map<number, number[]>();
  etats.forEach((etat, page) => {
    if (etat.etat !== "attribuee" && etat.etat !== "suite") return;
    const pages = groupes.get(etat.personne);
    if (pages) pages.push(page);
    else groupes.set(etat.personne, [page]);
  });
  return [...groupes].map(([personne, pages]) => ({ personne, pages }));
}

/** Pages qui ne vont à personne : à vérifier. */
export const pagesAVerifier = (etats: EtatPage[]) => etats.flatMap((etat, page) => (etat.etat === "attribuee" || etat.etat === "suite" ? [] : [page]));
