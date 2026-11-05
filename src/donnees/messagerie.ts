// Réglages de la messagerie, propres à ce PC : le moyen d'envoi (la messagerie de l'utilisateur, par
// SMTP, ou Outlook) et le compte d'envoi. Le mot de passe n'y figure jamais : le système le garde.
import type { CompteMessagerie } from "../coeur";

export type MoyenEnvoi = "messagerie" | "outlook";
export type Chiffrement = CompteMessagerie["chiffrement"];

export interface ReglagesMessagerie {
  moyen: MoyenEnvoi;
  compte: CompteMessagerie | null;
}

/** Port habituel de chaque chiffrement. */
export const PORTS: Record<Chiffrement, number> = { starttls: 587, ssl: 465, aucun: 25 };

export const COMPTE_VIDE: CompteMessagerie = { serveur: "", port: PORTS.starttls, chiffrement: "starttls", identifiant: "", adresse: "", nom: "" };

export const REGLAGES_MESSAGERIE_PAR_DEFAUT: ReglagesMessagerie = { moyen: "messagerie", compte: null };

const CHIFFREMENTS: Chiffrement[] = ["starttls", "ssl", "aucun"];

const estObjet = (valeur: unknown): valeur is Record<string, unknown> => typeof valeur === "object" && valeur !== null && !Array.isArray(valeur);
const texte = (valeur: unknown) => (typeof valeur === "string" ? valeur : "");
const portValide = (port: unknown): port is number => typeof port === "number" && Number.isInteger(port) && port > 0 && port < 65536;

/** Réglages lus dans leur fichier ; ce qui est illisible reprend sa valeur par défaut. */
export function lireReglagesMessagerie(contenu: string): ReglagesMessagerie {
  let lu: unknown;
  try {
    lu = JSON.parse(contenu || "{}");
  } catch {
    return REGLAGES_MESSAGERIE_PAR_DEFAUT;
  }
  if (!estObjet(lu)) return REGLAGES_MESSAGERIE_PAR_DEFAUT;
  const moyen: MoyenEnvoi = lu.moyen === "outlook" ? "outlook" : "messagerie";
  const compte = lu.compte;
  if (!estObjet(compte)) return { moyen, compte: null };
  const chiffrement = CHIFFREMENTS.find((valeur) => valeur === compte.chiffrement) ?? "starttls";
  return {
    moyen,
    compte: {
      serveur: texte(compte.serveur),
      port: portValide(compte.port) ? compte.port : PORTS[chiffrement],
      chiffrement,
      identifiant: texte(compte.identifiant),
      adresse: texte(compte.adresse),
      nom: texte(compte.nom),
    },
  };
}

/** Ce qui manque au compte pour envoyer (dans l'ordre du formulaire) ; null s'il est complet. */
export function manqueAuCompte(compte: CompteMessagerie): "adresse" | "serveur" | "port" | null {
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(compte.adresse.trim())) return "adresse";
  if (!compte.serveur.trim()) return "serveur";
  if (!portValide(compte.port)) return "port";
  return null;
}

/** Ce que l'utilisateur doit savoir de sa messagerie avant de saisir son mot de passe. */
export type ConseilMessagerie = "motDePasseApplication" | "microsoft";

export interface Suggestion {
  serveur: string;
  port: number;
  chiffrement: Chiffrement;
  conseil: ConseilMessagerie | null;
}

/** Réglages publiés par les messageries courantes. */
const CONNUES: { domaines: string[]; serveur: string; chiffrement: Chiffrement; conseil: ConseilMessagerie | null }[] = [
  { domaines: ["gmail.com", "googlemail.com"], serveur: "smtp.gmail.com", chiffrement: "starttls", conseil: "motDePasseApplication" },
  { domaines: ["yahoo.com", "yahoo.fr", "ymail.com", "rocketmail.com"], serveur: "smtp.mail.yahoo.com", chiffrement: "ssl", conseil: "motDePasseApplication" },
  { domaines: ["icloud.com", "me.com", "mac.com"], serveur: "smtp.mail.me.com", chiffrement: "starttls", conseil: "motDePasseApplication" },
  { domaines: ["outlook.com", "outlook.fr", "hotmail.com", "hotmail.fr", "live.com", "live.fr", "msn.com"], serveur: "smtp-mail.outlook.com", chiffrement: "starttls", conseil: "microsoft" },
  { domaines: ["orange.fr", "wanadoo.fr"], serveur: "smtp.orange.fr", chiffrement: "ssl", conseil: null },
  { domaines: ["free.fr"], serveur: "smtp.free.fr", chiffrement: "ssl", conseil: null },
  { domaines: ["sfr.fr", "neuf.fr"], serveur: "smtp.sfr.fr", chiffrement: "ssl", conseil: null },
  { domaines: ["laposte.net"], serveur: "smtp.laposte.net", chiffrement: "ssl", conseil: null },
];

/** Réglages d'une messagerie connue, d'après le domaine de l'adresse ; null pour les autres. */
export function suggestion(adresse: string): Suggestion | null {
  const domaine = adresse.trim().toLowerCase().split("@")[1];
  const connue = domaine ? CONNUES.find(({ domaines }) => domaines.includes(domaine)) : undefined;
  return connue ? { serveur: connue.serveur, port: PORTS[connue.chiffrement], chiffrement: connue.chiffrement, conseil: connue.conseil } : null;
}

/**
 * Le compte quand l'adresse change : l'identifiant la suit tant qu'il n'a pas été changé à la main,
 * et le serveur d'une messagerie connue est proposé tant qu'aucun autre n'a été saisi.
 */
export function avecAdresse(compte: CompteMessagerie, adresse: string): CompteMessagerie {
  const avant = suggestion(compte.adresse);
  const connue = suggestion(adresse);
  const identifiant = compte.identifiant === "" || compte.identifiant === compte.adresse ? adresse : compte.identifiant;
  const serveurLibre = compte.serveur === "" || compte.serveur === avant?.serveur;
  return {
    ...compte,
    adresse,
    identifiant: compte.chiffrement === "aucun" ? compte.identifiant : identifiant,
    ...(connue && serveurLibre ? { serveur: connue.serveur, port: connue.port, chiffrement: connue.chiffrement } : {}),
  };
}

/** Le compte quand le chiffrement change : le port suit tant qu'il est l'un des ports habituels ; sans chiffrement, pas de mot de passe. */
export function avecChiffrement(compte: CompteMessagerie, chiffrement: Chiffrement): CompteMessagerie {
  const portHabituel = Object.values(PORTS).includes(compte.port);
  return {
    ...compte,
    chiffrement,
    port: portHabituel ? PORTS[chiffrement] : compte.port,
    identifiant: chiffrement === "aucun" ? "" : compte.identifiant || compte.adresse,
  };
}
