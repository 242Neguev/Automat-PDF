// Recherche dans le texte d'un PDF (lecteur) : sans tenir compte des majuscules, des accents ni des
// espaces en trop ; « net a payer » trouve « Net à payer ».

/** Texte ramené à sa plus simple forme pour comparer : minuscules, sans accents, une espace entre les mots. */
export function normaliser(texte: string): string {
  return texte
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/[’‘`]/g, "'")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

/** Nombre de fois où `requete` (déjà normalisée) apparaît dans `texte`, sans chevauchement. */
export function compter(texte: string, requete: string): number {
  if (!requete) return 0;
  const cible = normaliser(texte);
  let nombre = 0;
  for (let i = cible.indexOf(requete); i >= 0; i = cible.indexOf(requete, i + requete.length)) nombre++;
  return nombre;
}

/** Une occurrence dans le document : sa page (à partir de 0) et son rang dans la page. */
export interface Occurrence {
  page: number;
  rang: number;
}

/** Les occurrences de `requete` dans les textes des pages, dans l'ordre du document. */
export function occurrences(textes: string[], requete: string): Occurrence[] {
  const cherchee = normaliser(requete);
  return textes.flatMap((texte, page) => Array.from({ length: compter(texte, cherchee) }, (_, rang) => ({ page, rang })));
}

/**
 * Les mots d'une page que couvre chaque occurrence de `requete`, dans l'ordre : une occurrence sur
 * plusieurs mots (« net à payer ») les couvre tous ; un morceau de mot couvre le mot entier.
 */
export function motsCouverts(mots: { texte: string }[], requete: string): number[][] {
  const cherchee = normaliser(requete);
  if (!cherchee) return [];
  let texte = "";
  const debuts: number[] = [];
  mots.forEach((mot, i) => {
    if (i > 0) texte += " ";
    debuts.push(texte.length);
    texte += normaliser(mot.texte);
  });
  const resultat: number[][] = [];
  for (let i = texte.indexOf(cherchee); i >= 0; i = texte.indexOf(cherchee, i + cherchee.length)) {
    const fin = i + cherchee.length;
    const couverts = debuts.flatMap((debut, j) => {
      const finMot = debut + normaliser(mots[j].texte).length;
      return debut < fin && finMot > i ? [j] : [];
    });
    if (couverts.length > 0) resultat.push(couverts);
  }
  return resultat;
}
