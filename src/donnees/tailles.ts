// Tailles de fichiers lisibles : « 850 Ko », « 12,4 Mo ».

export function formaterTaille(octets: number): string {
  if (octets < 1024) return `${octets} o`;
  const unites = ["Ko", "Mo", "Go"];
  let valeur = octets / 1024;
  let rang = 0;
  while (valeur >= 1024 && rang < unites.length - 1) {
    valeur /= 1024;
    rang++;
  }
  const arrondi = valeur >= 100 ? Math.round(valeur).toString() : valeur.toFixed(1).replace(".", ",").replace(/,0$/, "");
  return `${arrondi} ${unites[rang]}`;
}
