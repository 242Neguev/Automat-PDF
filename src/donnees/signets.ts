// Signets du lecteur (le sommaire d'un PDF) : lequel est déplié, lesquels se voient, lequel
// correspond à la page lue. Les signets arrivent à plat, dans l'ordre du sommaire, avec leur niveau.

export interface SignetPlace {
  niveau: number;
  ouvert: boolean;
  destination: { page: number } | null;
}

/** Le signet a-t-il des sous-signets ? (le suivant est d'un niveau plus profond) */
export function aDesSousSignets(signets: SignetPlace[], rang: number): boolean {
  return rang + 1 < signets.length && signets[rang + 1].niveau > signets[rang].niveau;
}

/** Signets dépliés à l'ouverture : ceux que le PDF déplie. */
export function ouvertsAuDepart(signets: SignetPlace[]): Set<number> {
  return new Set(signets.flatMap((signet, rang) => (signet.ouvert && aDesSousSignets(signets, rang) ? [rang] : [])));
}

/** Rangs des signets visibles : ceux dont tous les parents sont dépliés. */
export function signetsVisibles(signets: SignetPlace[], deplies: ReadonlySet<number>): number[] {
  const visibles: number[] = [];
  const parents: { niveau: number; deplie: boolean }[] = [];
  signets.forEach((signet, rang) => {
    while (parents.length > 0 && parents[parents.length - 1].niveau >= signet.niveau) parents.pop();
    if (parents.every((parent) => parent.deplie)) visibles.push(rang);
    parents.push({ niveau: signet.niveau, deplie: deplies.has(rang) });
  });
  return visibles;
}

/** Le signet de la page lue : le dernier, dans l'ordre du sommaire, qui mène à cette page ou avant. */
export function signetCourant(signets: SignetPlace[], page: number): number | null {
  let courant: number | null = null;
  let meilleure = -1;
  signets.forEach((signet, rang) => {
    const cible = signet.destination?.page;
    if (cible !== undefined && cible <= page && cible >= meilleure) {
      meilleure = cible;
      courant = rang;
    }
  });
  return courant;
}
