// Où vont des fichiers reçus (clic droit, « Ouvrir avec », glisser-déposer, choix depuis l'accueil) :
// à un outil, au lecteur, ou à la question « Que faire de ce fichier ? ».

/** Ce que l'on sait d'un outil pour lui confier des fichiers. */
export interface OutilCible {
  cle: string;
}

export type Destination<O extends OutilCible> =
  | { sorte: "outil"; outil: O }
  | { sorte: "lecteur"; chemin: string }
  | { sorte: "choix"; outils: O[] }
  | { sorte: "aucun" };

const estPdf = (chemin: string) => chemin.toLowerCase().endsWith(".pdf");

/**
 * - L'outil demandé (clic droit sur « Automat-PDF › Compresser ») ou l'outil affiché les prend
 *   s'il en accepte au moins un : on ne l'interrompt pas.
 * - Sinon, un seul PDF, sans outil demandé, s'ouvre dans le lecteur, qui propose les outils à côté.
 * - Sinon, depuis l'accueil, un seul outil possible est choisi d'office ; plusieurs : la question.
 *
 * @param demande outil demandé par le clic droit, ou null
 * @param affichee écran affiché : « accueil », « lecteur » ou la clé d'un outil
 * @param prend l'outil accepte-t-il ce fichier ?
 */
export function destination<O extends OutilCible>(
  fichiers: string[],
  demande: string | null,
  affichee: string,
  outils: O[],
  prend: (outil: O, fichier: string) => boolean,
): Destination<O> {
  const cible = outils.find((outil) => outil.cle === demande) ?? outils.find((outil) => outil.cle === affichee);
  if (cible && fichiers.some((fichier) => prend(cible, fichier))) return { sorte: "outil", outil: cible };
  if (demande === null && fichiers.length === 1 && estPdf(fichiers[0])) return { sorte: "lecteur", chemin: fichiers[0] };
  const possibles = outils.filter((outil) => fichiers.some((fichier) => prend(outil, fichier)));
  if (possibles.length === 1 && affichee === "accueil") return { sorte: "outil", outil: possibles[0] };
  if (possibles.length > 0) return { sorte: "choix", outils: possibles };
  return { sorte: "aucun" };
}
