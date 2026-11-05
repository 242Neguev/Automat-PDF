import { describe, expect, it } from "vitest";
import type { CompteMessagerie } from "../coeur";
import { avecAdresse, avecChiffrement, COMPTE_VIDE, lireReglagesMessagerie, manqueAuCompte, REGLAGES_MESSAGERIE_PAR_DEFAUT, suggestion } from "./messagerie";

const gmail: CompteMessagerie = { serveur: "smtp.gmail.com", port: 587, chiffrement: "starttls", identifiant: "rh.ecole@gmail.com", adresse: "rh.ecole@gmail.com", nom: "Service RH" };

describe("réglages de la messagerie", () => {
  it("se relisent tels qu'enregistrés", () => {
    const reglages = { moyen: "outlook" as const, compte: gmail };
    expect(lireReglagesMessagerie(JSON.stringify(reglages))).toEqual(reglages);
    expect(lireReglagesMessagerie("")).toEqual(REGLAGES_MESSAGERIE_PAR_DEFAUT);
  });

  it("écartent ce qui est illisible", () => {
    expect(lireReglagesMessagerie("{pas du json")).toEqual(REGLAGES_MESSAGERIE_PAR_DEFAUT);
    expect(lireReglagesMessagerie('{"moyen":"pigeon","compte":"texte"}')).toEqual({ moyen: "messagerie", compte: null });
    const abime = { compte: { serveur: "mail.exemple.cg", port: "465", chiffrement: "ssl", adresse: 12 } };
    expect(lireReglagesMessagerie(JSON.stringify(abime)).compte).toEqual({ ...COMPTE_VIDE, serveur: "mail.exemple.cg", port: 465, chiffrement: "ssl" });
  });

  it("disent ce qui manque au compte", () => {
    expect(manqueAuCompte(gmail)).toBeNull();
    expect(manqueAuCompte(COMPTE_VIDE)).toBe("adresse");
    expect(manqueAuCompte({ ...gmail, adresse: "rh chez gmail" })).toBe("adresse");
    expect(manqueAuCompte({ ...gmail, serveur: " " })).toBe("serveur");
    expect(manqueAuCompte({ ...gmail, port: 0 })).toBe("port");
  });
});

describe("messageries connues", () => {
  it("sont reconnues d'après l'adresse", () => {
    expect(suggestion("Rh.Ecole@GMAIL.com")).toEqual({ serveur: "smtp.gmail.com", port: 587, chiffrement: "starttls", conseil: "motDePasseApplication" });
    expect(suggestion("agent@yahoo.fr")).toEqual({ serveur: "smtp.mail.yahoo.com", port: 465, chiffrement: "ssl", conseil: "motDePasseApplication" });
    expect(suggestion("agent@hotmail.com")?.conseil).toBe("microsoft");
    expect(suggestion("rh@mairie-exemple.cg")).toBeNull();
    expect(suggestion("pas d'adresse")).toBeNull();
  });

  it("remplissent le serveur et l'identifiant, sans écraser ce qui a été saisi", () => {
    const tape = avecAdresse(COMPTE_VIDE, "rh.ecole@gmail.com");
    expect(tape).toEqual({ ...COMPTE_VIDE, adresse: "rh.ecole@gmail.com", identifiant: "rh.ecole@gmail.com", serveur: "smtp.gmail.com" });
    // Passage à Yahoo : le serveur proposé suit, tant qu'il n'a pas été changé à la main.
    expect(avecAdresse(tape, "rh.ecole@yahoo.fr")).toMatchObject({ serveur: "smtp.mail.yahoo.com", port: 465, chiffrement: "ssl", identifiant: "rh.ecole@yahoo.fr" });
    const maison = { ...tape, serveur: "mail.ecole.cg", identifiant: "rh" };
    expect(avecAdresse(maison, "direction@gmail.com")).toMatchObject({ serveur: "mail.ecole.cg", identifiant: "rh", adresse: "direction@gmail.com" });
  });

  it("ajustent le port et l'identifiant au chiffrement", () => {
    expect(avecChiffrement(gmail, "ssl")).toMatchObject({ chiffrement: "ssl", port: 465, identifiant: "rh.ecole@gmail.com" });
    expect(avecChiffrement(gmail, "aucun")).toMatchObject({ port: 25, identifiant: "" });
    expect(avecChiffrement({ ...gmail, port: 2525 }, "ssl").port).toBe(2525);
    expect(avecChiffrement({ ...gmail, chiffrement: "aucun", identifiant: "" }, "starttls").identifiant).toBe("rh.ecole@gmail.com");
  });
});
