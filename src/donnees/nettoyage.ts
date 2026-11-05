// Mise au propre des valeurs de la liste avant de les placer sur les documents.

export type Transformation = "majuscules" | "minuscules" | "nom-propre";

/** Supprime les espaces superflus puis applique la transformation demandée par le modèle. */
export function nettoyer(valeur: string | undefined, transformation?: Transformation): string {
  const texte = (valeur ?? "").replace(/\s+/g, " ").trim();
  switch (transformation) {
    case "majuscules":
      return texte.toLocaleUpperCase("fr");
    case "minuscules":
      return texte.toLocaleLowerCase("fr");
    case "nom-propre":
      // « jean-baptiste » → « Jean-Baptiste », « ÉLOÏSE » → « Éloïse »
      return texte
        .toLocaleLowerCase("fr")
        .replace(/(^|[\s'’-])(\p{L})/gu, (_, separateur: string, lettre: string) => separateur + lettre.toLocaleUpperCase("fr"));
    default:
      return texte;
  }
}
