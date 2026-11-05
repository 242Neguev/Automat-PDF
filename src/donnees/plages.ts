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
