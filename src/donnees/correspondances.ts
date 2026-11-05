// Association automatique des balises du modèle ({{NOM}}, {{Email}}…) aux colonnes de la liste.

/** Balise → colonne de la liste (null : à choisir par l'utilisateur). */
export type Correspondances = Record<string, string | null>;

/** Compare sans tenir compte des accents, de la casse, des espaces ni de la ponctuation. */
export const normaliser = (texte: string) =>
  texte
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");

// Chaque groupe réunit des intitulés courants qui désignent la même information.
const SYNONYMES = {
  prenom: ["prenom", "prenoms", "firstname", "givenname"],
  nom: ["nom", "noms", "nomdefamille", "lastname", "surname", "familyname"],
  fonction: ["fonction", "poste", "titre", "intitule", "intituledeposte", "jobtitle", "title"],
  email: ["email", "mail", "courriel", "adressemail", "adresseemail", "emailprofessionnel", "mailprofessionnel"],
  mobile: ["mobile", "portable", "gsm", "cellulaire", "telmobile", "telephonemobile", "telportable", "cell"],
  fixe: ["fixe", "telfixe", "telephonefixe", "bureau", "telbureau", "telephonebureau", "lignedirecte"],
  telephone: ["telephone", "tel", "phone", "numerodetelephone"],
  matricule: ["matricule", "identifiant", "numeroagent", "numerodagent", "id"],
  direction: ["direction", "departement", "service", "division", "pole"],
  site: ["site", "ville", "localisation", "agence", "implantation"],
};

/** Nature d'une information, d'après les intitulés courants. */
export type Nature = keyof typeof SYNONYMES;

const groupe = (nom: string) => Object.values(SYNONYMES).find((g) => g.includes(nom)) ?? [nom];

/** Nature d'une balise ou d'une colonne : « Courriel » → « email », « Tél. bureau » → « fixe ». */
export function nature(nom: string): Nature | null {
  const simple = normaliser(nom);
  const connue = (Object.keys(SYNONYMES) as Nature[]).find((cle) => SYNONYMES[cle].includes(simple));
  if (connue) return connue;
  if (simple.includes("mail")) return "email";
  if (/^(tel|phone|mobile|portable|gsm)/.test(simple)) return "telephone";
  return null;
}

/**
 * Associe chaque balise à une colonne : d'abord le même nom (au format près), puis un synonyme.
 * Une colonne n'est jamais associée à deux balises.
 */
export function associer(balises: string[], colonnes: string[]): Correspondances {
  const resultat: Correspondances = Object.fromEntries(balises.map((balise) => [balise, null]));
  const libres = new Set(colonnes);
  const essayer = (correspond: (balise: string, colonne: string) => boolean) => {
    for (const balise of balises) {
      if (resultat[balise]) continue;
      const colonne = colonnes.find((c) => libres.has(c) && correspond(normaliser(balise), normaliser(c)));
      if (colonne) {
        resultat[balise] = colonne;
        libres.delete(colonne);
      }
    }
  };
  essayer((balise, colonne) => balise === colonne);
  essayer((balise, colonne) => groupe(balise).includes(colonne));
  return resultat;
}

/**
 * Nouvelle association pour une liste, qui garde les choix déjà faits (à la main, ou enregistrés
 * avec le projet) tant que leur colonne existe dans la liste.
 */
export function fusionner(balises: string[], colonnes: string[], precedentes: Correspondances): Correspondances {
  const resultat = associer(balises, colonnes);
  for (const balise of balises) {
    const choisie = precedentes[balise];
    if (choisie !== undefined && (choisie === null || colonnes.includes(choisie))) resultat[balise] = choisie;
  }
  return resultat;
}
