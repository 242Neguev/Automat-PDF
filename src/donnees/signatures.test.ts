import { describe, expect, it } from "vitest";
import type { ModificationsSignature, SignaturePdf } from "../coeur";
import { dateLisible, etatDe, etatGeneral, jourLisible, modificationsDe, nonPermises, parPaires, permises, signaturesDeLaPage } from "./signatures";

const aucune: ModificationsSignature = { formulaire: false, signatures: false, commentaires: false, pages: false, autres: false };

function signature(plus: Partial<SignaturePdf> = {}): SignaturePdf {
  return {
    champ: "Signature1",
    page: 0,
    zone: [300, 662, 220, 60],
    sorte: "approbation",
    permissions: null,
    signataire: "Jeanne Martin",
    raison: null,
    lieu: null,
    contact: null,
    date: "2026-11-26T08:12:00Z",
    dateHorodatee: false,
    horodatage: null,
    integrite: "intacte",
    nonPrisEnCharge: null,
    empreinte: "SHA-256",
    empreinteAncienne: false,
    confiance: "reconnue",
    certificat: null,
    entiere: true,
    modifications: aucune,
    fin: 1000,
    ...plus,
  };
}

const modifiee = (plus: Partial<ModificationsSignature>, autre: Partial<SignaturePdf> = {}) => signature({ entiere: false, modifications: { ...aucune, ...plus }, ...autre });

describe("signatures numériques", () => {
  it("est valide quand rien n'a changé et que le système reconnaît le certificat", () => {
    expect(etatDe(signature())).toBe("valide");
  });

  it("reste valide quand on a seulement commenté, rempli ou signé ensuite", () => {
    expect(etatDe(modifiee({ commentaires: true, formulaire: true, signatures: true }))).toBe("valide");
    expect(modificationsDe(modifiee({ commentaires: true, signatures: true }))).toEqual(["commentaires", "signatures"]);
  });

  it("demande attention : pages changées, identité non confirmée, algorithme ancien, horodatage douteux", () => {
    expect(etatDe(modifiee({ pages: true }))).toBe("attention");
    expect(etatDe(modifiee({ autres: true }))).toBe("attention");
    expect(etatDe(signature({ confiance: "nonReconnue" }))).toBe("attention");
    expect(etatDe(signature({ confiance: "horsValidite" }))).toBe("attention");
    expect(etatDe(signature({ confiance: "inconnue" }))).toBe("attention");
    expect(etatDe(signature({ empreinteAncienne: true }))).toBe("attention");
    expect(etatDe(signature({ integrite: "nonPriseEnCharge" }))).toBe("attention");
    expect(etatDe(signature({ integrite: "illisible" }))).toBe("attention");
    expect(etatDe(signature({ horodatage: { date: null, autorite: null, valide: false, confiance: "reconnue" } }))).toBe("attention");
  });

  it("n'est pas valable : version signée modifiée, signature fausse, certificat révoqué", () => {
    expect(etatDe(signature({ integrite: "alteree" }))).toBe("invalide");
    expect(etatDe(signature({ integrite: "fausse" }))).toBe("invalide");
    expect(etatDe(signature({ confiance: "revoquee" }))).toBe("invalide");
  });

  it("applique les permissions d'une certification", () => {
    const certifiee = (niveau: number, plus: Partial<ModificationsSignature>) => modifiee(plus, { sorte: "certification", permissions: niveau });
    expect(permises(signature({ sorte: "certification", permissions: 1 }))).toEqual(["signatures"]);
    expect(etatDe(certifiee(1, { formulaire: true }))).toBe("invalide");
    expect(etatDe(certifiee(2, { formulaire: true, signatures: true }))).toBe("valide");
    expect(etatDe(certifiee(2, { commentaires: true }))).toBe("invalide");
    expect(nonPermises(certifiee(2, { commentaires: true, formulaire: true }))).toEqual(["commentaires"]);
    expect(etatDe(certifiee(3, { commentaires: true }))).toBe("valide");
  });

  it("donne le verdict de la moins bonne signature", () => {
    expect(etatGeneral([])).toBe("valide");
    expect(etatGeneral([signature(), signature({ confiance: "nonReconnue" })])).toBe("attention");
    expect(etatGeneral([signature({ integrite: "alteree" }), signature({ confiance: "nonReconnue" })])).toBe("invalide");
  });

  it("écrit les dates et les empreintes pour l'utilisateur", () => {
    expect(dateLisible("2026-11-26T08:12:00Z")).toMatch(/^2[67] novembre 2026 à \d\d:\d\d$/);
    expect(dateLisible(null)).toBeNull();
    expect(dateLisible("pas une date")).toBeNull();
    expect(jourLisible("2026-01-01T12:00:00Z")).toBe("1 janvier 2026");
    expect(parPaires("03E9")).toBe("03 E9");
  });

  it("place les signatures visibles sur leur page", () => {
    const invisible = signature({ champ: "Horodatage", page: null, zone: null });
    const deuxieme = signature({ champ: "Signature2", page: 1 });
    expect(signaturesDeLaPage([signature(), invisible, deuxieme], 0).map((s) => s.champ)).toEqual(["Signature1"]);
    expect(signaturesDeLaPage([signature(), invisible, deuxieme], 1).map((s) => s.champ)).toEqual(["Signature2"]);
  });
});
