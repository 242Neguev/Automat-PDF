// Noms de fichiers lisibles et uniques dans un dossier, à partir des noms des personnes.

/** Noms sans caractère interdit par Windows, raccourcis, rendus uniques (« Nom (2) ») ; sans extension. */
export function nomsUniques(noms: string[]): string[] {
  const deja = new Map<string, number>();
  return noms.map((nom) => {
    const base = nom.replace(/[\\/:*?"<>|]/g, "-").replace(/\s+/g, " ").trim().slice(0, 80) || "Document";
    const n = (deja.get(base.toLowerCase()) ?? 0) + 1;
    deja.set(base.toLowerCase(), n);
    return n === 1 ? base : `${base} (${n})`;
  });
}
