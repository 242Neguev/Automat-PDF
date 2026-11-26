// Signatures numériques des PDF reçus (0.7.0) : le verdict montré à l'utilisateur, d'après ce que le
// cœur a vérifié (intégrité, modifications depuis la signature, confiance dans le certificat).
import type { ModificationsSignature, SignaturePdf } from "../coeur";

/** Valide ; à regarder de plus près ; pas valable. */
export type EtatSignature = "valide" | "attention" | "invalide";

/** Les sortes de modifications, dans l'ordre où on les présente. */
export const SORTES_MODIFICATIONS: (keyof ModificationsSignature)[] = ["pages", "autres", "commentaires", "formulaire", "signatures"];

/** Ce qui a changé depuis la signature. */
export const modificationsDe = (signature: SignaturePdf) => SORTES_MODIFICATIONS.filter((sorte) => signature.modifications[sorte]);

/**
 * Les modifications que la signature permet après elle. Une certification en fixe la liste :
 * aucune (1), remplir et signer (2), aussi commenter (3). Une signature ordinaire laisse remplir,
 * signer et commenter, comme les lecteurs de PDF courants ; jamais changer les pages.
 */
export function permises(signature: SignaturePdf): (keyof ModificationsSignature)[] {
  if (signature.sorte !== "certification") return ["formulaire", "signatures", "commentaires"];
  const niveau = signature.permissions ?? 2;
  return niveau >= 3 ? ["formulaire", "signatures", "commentaires"] : niveau === 2 ? ["formulaire", "signatures"] : ["signatures"];
}

/** Les modifications faites depuis la signature qu'elle ne permet pas. */
export const nonPermises = (signature: SignaturePdf) => modificationsDe(signature).filter((sorte) => !permises(signature).includes(sorte));

/** Le verdict d'une signature. */
export function etatDe(signature: SignaturePdf): EtatSignature {
  if (signature.integrite === "alteree" || signature.integrite === "fausse" || signature.confiance === "revoquee") return "invalide";
  // Une certification dont l'auteur n'a pas permis ce qui a été fait ensuite n'est plus valable.
  if (signature.sorte === "certification" && nonPermises(signature).length > 0) return "invalide";
  if (signature.integrite !== "intacte") return "attention";
  if (signature.confiance !== "reconnue") return "attention";
  if (signature.empreinteAncienne || nonPermises(signature).length > 0) return "attention";
  if (signature.horodatage && !signature.horodatage.valide) return "attention";
  return "valide";
}

const GRAVITE: Record<EtatSignature, number> = { valide: 0, attention: 1, invalide: 2 };

/** Le verdict de toutes les signatures d'un PDF : celui de la moins bonne. */
export const etatGeneral = (signatures: SignaturePdf[]): EtatSignature =>
  signatures.map(etatDe).reduce<EtatSignature>((pire, etat) => (GRAVITE[etat] > GRAVITE[pire] ? etat : pire), "valide");

/** Une date ISO 8601 écrite pour l'utilisateur, à l'heure de son ordinateur : « 26 novembre 2026 à 09:12 ». */
export function dateLisible(iso: string | null): string | null {
  if (!iso) return null;
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return null;
  const jour = date.toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
  const heure = date.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
  return `${jour} à ${heure}`;
}

/** Une date ISO 8601, sans l'heure : « 1 janvier 2026 ». */
export function jourLisible(iso: string | null): string | null {
  if (!iso) return null;
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? null : date.toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
}

/** Une empreinte en hexadécimal, par paires : « 3F 2A … ». */
export const parPaires = (hexadecimal: string) => hexadecimal.match(/.{1,2}/g)?.join(" ") ?? "";

/** Les signatures visibles d'une page. */
export const signaturesDeLaPage = (signatures: SignaturePdf[], page: number) => signatures.filter((s) => s.page === page && s.zone !== null);
