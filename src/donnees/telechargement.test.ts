import { describe, expect, it } from "vitest";
import type { DownloadEvent } from "@tauri-apps/plugin-updater";
import { avancer, pourcentage, signatureRefusee, TELECHARGEMENT_VIDE } from "./telechargement";

const suivre = (evenements: DownloadEvent[]) => evenements.reduce(avancer, TELECHARGEMENT_VIDE);

describe("téléchargement d'une mise à jour", () => {
  it("avance d'après la taille annoncée", () => {
    const debut = suivre([{ event: "Started", data: { contentLength: 1000 } }]);
    expect(pourcentage(debut)).toBe(0);
    const milieu = suivre([
      { event: "Started", data: { contentLength: 1000 } },
      { event: "Progress", data: { chunkLength: 250 } },
      { event: "Progress", data: { chunkLength: 259 } },
    ]);
    expect(milieu).toEqual({ recus: 509, total: 1000 });
    expect(pourcentage(milieu)).toBe(50);
    expect(pourcentage(avancer(milieu, { event: "Finished" }))).toBe(100);
  });

  it("reste sans pourcentage quand la taille n'est pas annoncée", () => {
    const inconnu = suivre([{ event: "Started", data: {} }, { event: "Progress", data: { chunkLength: 4096 } }]);
    expect(inconnu).toEqual({ recus: 4096, total: null });
    expect(pourcentage(inconnu)).toBeNull();
    expect(pourcentage(avancer(inconnu, { event: "Finished" }))).toBeNull();
    expect(pourcentage(suivre([{ event: "Started", data: { contentLength: 0 } }]))).toBeNull();
  });

  it("ne dépasse jamais 100 %", () => {
    const trop = suivre([
      { event: "Started", data: { contentLength: 100 } },
      { event: "Progress", data: { chunkLength: 150 } },
    ]);
    expect(pourcentage(trop)).toBe(100);
  });
});

describe("mise à jour refusée", () => {
  it("reconnaît une signature refusée", () => {
    // Messages relevés dans l'application : fichier modifié, ancienne version resservie.
    expect(signatureRefusee("The signature verification failed")).toBe(true);
    expect(
      signatureRefusee(
        "The update was signed for version 0.0.1 but the update endpoint announced version 9.9.9. The endpoint response may have been tampered with to force installing a different release.",
      ),
    ).toBe(true);
    expect(signatureRefusee("The signature was created with a different key than the one provided")).toBe(true);
    expect(signatureRefusee("The update signature does not specify the version it was signed for, which `requireSignedVersion` requires.")).toBe(true);
    expect(signatureRefusee("Invalid encoding in minisign data")).toBe(true);
  });

  it("laisse réessayer après une panne de connexion", () => {
    expect(signatureRefusee("error sending request for url (https://github.com/242Neguev/Automat-PDF/releases/download/v0.2.1/Automat-PDF_0.2.1_x64-setup.exe)")).toBe(false);
    expect(signatureRefusee("`Download request failed with status: 404 Not Found`")).toBe(false);
    expect(signatureRefusee("I/O error: Espace insuffisant sur le disque. (os error 112)")).toBe(false);
  });
});
