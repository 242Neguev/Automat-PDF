// Pages choisies par l'utilisateur sous la forme « 1-3, 5, 8- » (numéros à partir de 1).

export type Plages = { pages: number[] } | { erreur: string };

/**
 * Numéros de pages (à partir de 0), dans l'ordre donné et sans doublon. « 8- » va jusqu'à la
 * dernière page, « -3 » part de la première ; virgules, points-virgules et espaces séparent.
 */
export function lirePlages(texte: string, total: number): Plages {
  const morceaux = texte.split(/[,;\s]+/).filter(Boolean);
  if (morceaux.length === 0) return { erreur: "Indiquez au moins une page, par exemple « 1-3, 5 »." };
  const pages: number[] = [];
  for (const morceau of morceaux) {
    const plage = /^(\d*)-(\d*)$/.exec(morceau);
    const seule = /^\d+$/.test(morceau) ? Number(morceau) : null;
    if (!plage && seule === null) return { erreur: `« ${morceau} » n'est pas un numéro de page ni une plage (ex. 2-5).` };
    const debut = plage ? (plage[1] ? Number(plage[1]) : 1) : seule!;
    const fin = plage ? (plage[2] ? Number(plage[2]) : total) : seule!;
    if (debut < 1 || fin > total || debut > fin) return { erreur: `« ${morceau} » sort du document, qui compte ${total} page${total > 1 ? "s" : ""}.` };
    for (let page = debut; page <= fin; page++) if (!pages.includes(page - 1)) pages.push(page - 1);
  }
  return { pages };
}

/** Pages (à partir de 0) écrites comme on les tape : « 3, 5-7 » ; dans l'ordre du document, sans doublon. */
export function ecrirePlages(pages: number[]): string {
  const triees = [...new Set(pages)].sort((a, b) => a - b);
  const morceaux: string[] = [];
  for (let debut = 0; debut < triees.length; ) {
    let fin = debut;
    while (fin + 1 < triees.length && triees[fin + 1] === triees[fin] + 1) fin++;
    morceaux.push(fin > debut ? `${triees[debut] + 1}-${triees[fin] + 1}` : `${triees[debut] + 1}`);
    debut = fin + 1;
  }
  return morceaux.join(", ");
}

/**
 * Nom proposé pour un PDF fait de pages choisies (à partir de 0) d'un PDF nommé `base` (sans
 * « .pdf ») : « Rapport - page 3.pdf », « Rapport - pages 3 à 5.pdf », « Rapport - pages 2, 5 et
 * 7.pdf », au-delà de quatre pages éparses « Rapport - 9 pages.pdf ».
 */
export function nomExtrait(base: string, pages: number[]): string {
  const numeros = [...new Set(pages)].sort((a, b) => a - b).map((page) => page + 1);
  const dernier = numeros[numeros.length - 1];
  if (numeros.length === 0) return `${base}.pdf`;
  if (numeros.length === 1) return `${base} - page ${dernier}.pdf`;
  if (dernier - numeros[0] === numeros.length - 1) return `${base} - pages ${numeros[0]} à ${dernier}.pdf`;
  if (numeros.length <= 4) return `${base} - pages ${numeros.slice(0, -1).join(", ")} et ${dernier}.pdf`;
  return `${base} - ${numeros.length} pages.pdf`;
}

/** Façons de choisir les pages concernées par une opération. */
export type ChoixPages = "toutes" | "sauf-premiere" | "cette" | "derniere" | "certaines";

/**
 * Pages concernées (à partir de 0) ; une liste vide veut dire « toutes ». `page` est la page
 * affichée (pour « cette page »), `plages` le texte saisi (pour « certaines »).
 */
export function pagesSelon(choix: ChoixPages, plages: string, total: number, page: number): Plages {
  switch (choix) {
    case "toutes":
      return { pages: [] };
    case "sauf-premiere":
      return total > 1 ? { pages: Array.from({ length: total - 1 }, (_, i) => i + 1) } : { erreur: "Le document n'a qu'une page." };
    case "cette":
      return { pages: [page] };
    case "derniere":
      return { pages: [total - 1] };
    case "certaines":
      return lirePlages(plages, total);
  }
}
