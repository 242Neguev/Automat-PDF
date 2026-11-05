// Ordre des pages quand on réunit des rectos et des versos scannés séparément.

export interface PageIntercalee {
  cote: "recto" | "verso";
  /** Page dans son PDF, à partir de 0. */
  page: number;
}

/**
 * Recto 1, verso 1, recto 2, verso 2… Les versos sont souvent dans l'ordre inverse : on retourne la
 * pile de feuilles pour les scanner, le dernier verso passe donc en premier. Les pages en trop
 * (un PDF plus long que l'autre) sont ajoutées à la fin.
 */
export function intercaler(rectos: number, versos: number, versosInverses: boolean): PageIntercalee[] {
  const ordreVersos = Array.from({ length: versos }, (_, i) => (versosInverses ? versos - 1 - i : i));
  const pages: PageIntercalee[] = [];
  for (let i = 0; i < Math.max(rectos, versos); i++) {
    if (i < rectos) pages.push({ cote: "recto", page: i });
    if (i < versos) pages.push({ cote: "verso", page: ordreVersos[i] });
  }
  return pages;
}
