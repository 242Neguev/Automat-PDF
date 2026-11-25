// Documents récents : les derniers PDF ouverts dans le lecteur, du plus récent au plus ancien. La liste
// reste sur l'ordinateur (stockage de l'application) et ne contient que des chemins et des dates.

export interface DocumentRecent {
  chemin: string;
  /** Dernière ouverture, en millisecondes depuis 1970. */
  ouvert: number;
}

/** Nombre de documents gardés dans la liste. */
export const NOMBRE_RECENTS = 8;
const CLE = "automat-pdf.documents-recents";

/** La liste après l'ouverture de `chemin` : en tête, sans doublon, `NOMBRE_RECENTS` au plus. */
export function ajouterRecent(liste: DocumentRecent[], chemin: string, maintenant: number): DocumentRecent[] {
  return [{ chemin, ouvert: maintenant }, ...liste.filter((document) => document.chemin !== chemin)].slice(0, NOMBRE_RECENTS);
}

/** La liste sans `chemin`. */
export const retirerRecent = (liste: DocumentRecent[], chemin: string) => liste.filter((document) => document.chemin !== chemin);

/** Depuis quand un document a été ouvert, en mots : « à l'instant », « il y a 12 min », « hier », « le 12/11 ». */
export function ilYa(ouvert: number, maintenant: number): string {
  const minutes = Math.floor((maintenant - ouvert) / 60000);
  if (minutes < 1) return "à l'instant";
  if (minutes < 60) return `il y a ${minutes} min`;
  const avant = new Date(ouvert);
  const aujourdhui = new Date(maintenant);
  const jour = (date: Date) => new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const jours = Math.round((jour(aujourdhui) - jour(avant)) / 86400000);
  if (jours === 0) return `il y a ${Math.floor(minutes / 60)} h`;
  if (jours === 1) return "hier";
  if (jours < 7) return `il y a ${jours} jours`;
  return `le ${String(avant.getDate()).padStart(2, "0")}/${String(avant.getMonth() + 1).padStart(2, "0")}`;
}

function stockageParDefaut(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

/** La liste gardée ; vide si elle manque, est abîmée ou si le stockage est indisponible. */
export function lireRecents(stockage: Pick<Storage, "getItem"> | null = stockageParDefaut()): DocumentRecent[] {
  try {
    const lue: unknown = JSON.parse(stockage?.getItem(CLE) ?? "[]");
    if (!Array.isArray(lue)) return [];
    return lue
      .filter((document): document is DocumentRecent => typeof document?.chemin === "string" && typeof document?.ouvert === "number")
      .slice(0, NOMBRE_RECENTS);
  } catch {
    return [];
  }
}

/** Garde la liste ; si le stockage est plein ou refusé, elle ne vaut que jusqu'à la fermeture. */
export function ecrireRecents(liste: DocumentRecent[], stockage: Pick<Storage, "setItem"> | null = stockageParDefaut()) {
  try {
    stockage?.setItem(CLE, JSON.stringify(liste));
  } catch {
    // rien de plus à faire : la liste reste en mémoire
  }
}
