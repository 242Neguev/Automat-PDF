// Chemins des fichiers choisis : leur nom, leur dossier, et l'emplacement proposé pour un résultat.

/** Nom du fichier, sans son dossier. */
export const nomDe = (chemin: string) => chemin.split(/[\\/]/).pop() ?? chemin;

/** Dossier qui contient le fichier (« C:\Paie » pour « C:\Paie\octobre.pdf ») ; vide pour un nom seul. */
export function dossierDe(chemin: string): string {
  const fin = Math.max(chemin.lastIndexOf("\\"), chemin.lastIndexOf("/"));
  if (fin < 0) return "";
  const dossier = chemin.slice(0, fin);
  // La racine garde sa barre : « C:\ », « / ».
  return dossier === "" || /^[A-Za-z]:$/.test(dossier) ? chemin.slice(0, fin + 1) : dossier;
}

/**
 * Chemin proposé pour un nouveau fichier : à côté de `source`, sous le nom `nom`. Sans source, le
 * nom seul (la fenêtre d'enregistrement choisit alors son dossier).
 */
export function aCote(source: string | null | undefined, nom: string): string {
  const dossier = source ? dossierDe(source) : "";
  if (!dossier) return nom;
  const separateur = source!.includes("\\") ? "\\" : "/";
  return dossier.endsWith(separateur) ? dossier + nom : dossier + separateur + nom;
}
