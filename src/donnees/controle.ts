// Contrôle des données de la liste : e-mails et téléphones mal formés, personnes en double, informations
// vides. Les anomalies sont signalées avec les numéros de ligne du fichier, pour le corriger à la source.
import { nature, normaliser, type Correspondances } from "./correspondances";
import type { Liste } from "./liste";
import { nettoyer } from "./nettoyage";

export type TypeAnomalie = "colonne-vide" | "email" | "telephone" | "doublon" | "vide";

export interface Anomalie {
  type: TypeAnomalie;
  /** « attention » : à corriger sans doute ; « info » : sans conséquence grave. */
  gravite: "attention" | "info";
  /** Information concernée (balise), ou balises qui identifient une personne pour les doublons. */
  balise: string;
  /** Numéros des lignes concernées, comme dans le fichier. */
  lignes: number[];
  /** Première valeur en cause, pour l'exemple. */
  exemple?: string;
}

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@.]{2,}$/;
const CARACTERES_TELEPHONE = /^[\d\s+().\/-]+$/;

/** Un numéro plausible : chiffres et séparateurs usuels, de 6 à 15 chiffres (norme internationale). */
export function telephonePlausible(valeur: string): boolean {
  const chiffres = valeur.replace(/\D/g, "").length;
  return CARACTERES_TELEPHONE.test(valeur) && chiffres >= 6 && chiffres <= 15;
}

export const emailPlausible = (valeur: string) => EMAIL.test(valeur);

/** Anomalies de la liste pour les balises reliées à une colonne, les plus importantes d'abord. */
export function controlerListe(liste: Liste, balises: string[], correspondances: Correspondances): Anomalie[] {
  const anomalies: Anomalie[] = [];
  const reliees = balises.filter((balise) => correspondances[balise]);
  const valeur = (balise: string, ligne: Liste["lignes"][number]) => nettoyer(ligne.valeurs[correspondances[balise]!]);
  const natureDe = (balise: string) => nature(balise) ?? nature(correspondances[balise]!);

  for (const balise of reliees) {
    const genre = natureDe(balise);
    const vides = liste.lignes.filter((ligne) => !valeur(balise, ligne)).map((ligne) => ligne.numero);
    if (vides.length === liste.lignes.length) {
      anomalies.push({ type: "colonne-vide", gravite: "attention", balise, lignes: [] });
      continue;
    }
    if (vides.length > 0) {
      // Sans nom, le fichier ne peut pas être nommé ; une autre information vide est moins grave.
      anomalies.push({ type: "vide", gravite: genre === "nom" || genre === "prenom" ? "attention" : "info", balise, lignes: vides });
    }
    const verifier = genre === "email" ? emailPlausible : genre === "mobile" || genre === "fixe" || genre === "telephone" ? telephonePlausible : null;
    if (!verifier) continue;
    const fautives = liste.lignes.filter((ligne) => {
      const texte = valeur(balise, ligne);
      return texte !== "" && !verifier(texte);
    });
    if (fautives.length > 0) {
      anomalies.push({
        type: genre === "email" ? "email" : "telephone",
        gravite: "attention",
        balise,
        lignes: fautives.map((ligne) => ligne.numero),
        exemple: valeur(balise, fautives[0]),
      });
    }
  }

  // Personnes en double : même prénom et même nom, ou même e-mail.
  const identite = reliees.filter((balise) => ["prenom", "nom"].includes(natureDe(balise) ?? ""));
  const emails = reliees.filter((balise) => natureDe(balise) === "email");
  for (const cles of [identite, emails]) {
    if (cles.length === 0) continue;
    const parCle = new Map<string, { lignes: number[]; exemple: string }>();
    for (const ligne of liste.lignes) {
      const valeurs = cles.map((balise) => valeur(balise, ligne));
      const cle = valeurs.map(normaliser).join("|");
      if (valeurs.every((v) => v === "")) continue;
      const deja = parCle.get(cle);
      if (deja) deja.lignes.push(ligne.numero);
      else parCle.set(cle, { lignes: [ligne.numero], exemple: valeurs.filter(Boolean).join(" ") });
    }
    for (const { lignes, exemple } of parCle.values()) {
      if (lignes.length > 1) anomalies.push({ type: "doublon", gravite: "attention", balise: cles.join(" "), lignes, exemple });
    }
  }

  const ordre: TypeAnomalie[] = ["colonne-vide", "doublon", "email", "telephone", "vide"];
  return anomalies.sort((a, b) => Number(a.gravite === "info") - Number(b.gravite === "info") || ordre.indexOf(a.type) - ordre.indexOf(b.type));
}
