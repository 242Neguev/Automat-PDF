// Enchaînements enregistrés : les réglages d'une séparation par personne (repérage, noms des
// fichiers, mot de passe, message, liste des personnes), à reprendre d'un clic chaque mois.
import type { Correspondances } from "./correspondances";
import type { PositionValeur } from "./reperage";

/** Message envoyé avec chaque fichier, par Outlook. */
export interface ReglagesEnvoi {
  actif: boolean;
  /** Modèles à balises : {{E-mail}}, {{Prénom}}… */
  destinataire: string;
  objet: string;
  message: string;
  mode: "brouillons" | "direct";
}

/** Réglages de la séparation par personne. */
export interface Reglages {
  mode: "liste" | "repere";
  identifiant: string;
  repere: string;
  position: PositionValeur;
  suite: boolean;
  modeleNom: string;
  modeleNomRepere: string;
  proteger: boolean;
  modeleMotDePasse: string;
  envoi: ReglagesEnvoi;
}

export interface Enchainement extends Reglages {
  nom: string;
  /** Liste des personnes : son fichier, sa feuille et les colonnes choisies. */
  liste: { chemin: string; feuille: string | null; correspondances: Correspondances } | null;
}

export const REGLAGES_PAR_DEFAUT: Reglages = {
  mode: "liste",
  identifiant: "{{Matricule}}",
  repere: "Matricule :",
  position: "droite",
  suite: false,
  modeleNom: "Bulletin {{NOM}} {{Prénom}}",
  modeleNomRepere: "Bulletin {{Repère}}",
  proteger: false,
  modeleMotDePasse: "{{Mot de passe}}",
  envoi: {
    actif: false,
    destinataire: "{{E-mail}}",
    objet: "Votre bulletin de paie",
    message: "Bonjour {{Prénom}},\n\nVeuillez trouver ci-joint votre bulletin de paie.\n\nCordialement,",
    mode: "brouillons",
  },
};

const estObjet = (valeur: unknown): valeur is Record<string, unknown> => typeof valeur === "object" && valeur !== null && !Array.isArray(valeur);

/** Valeur enregistrée si elle a le bon type, sinon celle par défaut (fichier ancien ou abîmé). */
function reprendre<T extends object>(defaut: T, enregistre: unknown): T {
  if (!estObjet(enregistre)) return defaut;
  const resultat = { ...defaut };
  for (const cle of Object.keys(defaut) as (keyof T)[]) {
    const valeur = enregistre[cle as string];
    const attendu = defaut[cle];
    if (estObjet(attendu)) resultat[cle] = reprendre(attendu as object, valeur) as T[keyof T];
    else if (typeof valeur === typeof attendu) resultat[cle] = valeur as T[keyof T];
  }
  return resultat;
}

/** Enchaînements lus dans le fichier des réglages ; ce qui est illisible est écarté. */
export function lireEnchainements(contenu: string): Enchainement[] {
  let lu: unknown;
  try {
    lu = JSON.parse(contenu || "[]");
  } catch {
    return [];
  }
  if (!Array.isArray(lu)) return [];
  return lu.filter((element) => estObjet(element) && typeof element.nom === "string" && element.nom.trim()).map((element) => {
    const liste = element.liste;
    return {
      ...reprendre(REGLAGES_PAR_DEFAUT, element),
      nom: String(element.nom).trim(),
      liste:
        estObjet(liste) && typeof liste.chemin === "string"
          ? {
              chemin: liste.chemin,
              feuille: typeof liste.feuille === "string" ? liste.feuille : null,
              correspondances: estObjet(liste.correspondances)
                ? Object.fromEntries(Object.entries(liste.correspondances).filter(([, colonne]) => colonne === null || typeof colonne === "string")) as Correspondances
                : {},
            }
          : null,
    };
  });
}

/** Les réglages d'un enchaînement, sans son nom ni sa liste. */
export const reglagesDe = (enchainement: Enchainement): Reglages => reprendre(REGLAGES_PAR_DEFAUT, enchainement);

const memeNom = (a: string, b: string) => a.trim().toLocaleLowerCase("fr") === b.trim().toLocaleLowerCase("fr");

/** Ajoute l'enchaînement, ou remplace celui du même nom (à sa place). */
export function avecEnchainement(enchainements: Enchainement[], nouveau: Enchainement): Enchainement[] {
  const rang = enchainements.findIndex((existant) => memeNom(existant.nom, nouveau.nom));
  return rang < 0 ? [...enchainements, nouveau] : enchainements.map((existant, i) => (i === rang ? nouveau : existant));
}

export const sansEnchainement = (enchainements: Enchainement[], nom: string) => enchainements.filter((existant) => !memeNom(existant.nom, nom));
