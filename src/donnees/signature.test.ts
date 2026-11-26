import { describe, expect, it } from "vitest";
import { chemin, dansLeCadre, ecrireSignatures, HAUTEUR_INITIALES, HAUTEUR_SIGNATURE, lireSignatures, normaliser, zoneDansCadre, zoneSignature } from "./signature";

describe("signature dessinée", () => {
  it("lisse un trait comme le cœur : des courbes par le milieu de chaque segment", () => {
    expect(chemin([])).toBe("");
    expect(chemin([{ x: 1, y: 2 }])).toBe("M1 2L1 2");
    expect(chemin([{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }])).toBe("M0 0Q10 0 10 5L10 10");
    // À l'échelle de la boîte : x par la proportion, y par 1.
    expect(chemin([{ x: 0.5, y: 0.5 }, { x: 1, y: 1 }], 3, 1)).toBe("M1.5 0.5L3 1");
  });

  it("recadre le dessin au plus près, en fractions de sa boîte", () => {
    expect(normaliser([], 4)).toBeNull();
    expect(normaliser([[]], 4)).toBeNull();
    // Un trait de 100 sur 40, épaisseur 4 : boîte de 104 sur 44 (la moitié de l'épaisseur autour).
    const trace = normaliser([[{ x: 50, y: 60 }, { x: 150, y: 100 }]], 4)!;
    expect(trace.proportion).toBeCloseTo(104 / 44, 3);
    expect(trace.epaisseur).toBeCloseTo(4 / 44, 3);
    expect(trace.traits[0][0]).toEqual({ x: 0.0192, y: 0.0455 });
    expect(trace.traits[0][1].x).toBeCloseTo(0.9808, 3);
    // Un trait droit garde au moins quatre épaisseurs de haut, centré.
    const plat = normaliser([[{ x: 0, y: 10 }, { x: 100, y: 10 }]], 4)!;
    expect(plat.proportion).toBeCloseTo(104 / 16, 3);
    expect(plat.traits[0][0].y).toBe(0.5);
  });

  it("pose la signature d'un clic, ou dans le cadre d'un champ, sans la déformer", () => {
    const trace = { proportion: 3, traits: [[{ x: 0, y: 0 }]], epaisseur: 0.05 };
    expect(zoneSignature(trace, { x: 300, y: 400 }, false)).toEqual({ x: 300 - 63, y: 400 - 21, largeur: 3 * HAUTEUR_SIGNATURE, hauteur: HAUTEUR_SIGNATURE });
    expect(zoneSignature(trace, { x: 0, y: 0 }, true).hauteur).toBe(HAUTEUR_INITIALES);
    // Très large : 240 points au plus.
    const large = zoneSignature({ ...trace, proportion: 10 }, { x: 0, y: 0 }, false);
    expect([large.largeur, large.hauteur]).toEqual([240, 24]);
    // Dans un cadre de 200 sur 50 : la hauteur disponible (marges de 4) fixe la taille, centrée.
    const dans = zoneDansCadre(trace, { x: 56, y: 500, largeur: 200, hauteur: 50 });
    expect(dans.hauteur).toBeCloseTo(42);
    expect(dans.largeur).toBeCloseTo(126);
    expect(dans.x + dans.largeur / 2).toBeCloseTo(156);
    expect(dansLeCadre(dans, { x: 56, y: 500, largeur: 200, hauteur: 50 })).toBe(true);
    expect(dansLeCadre(dans, { x: 300, y: 500, largeur: 200, hauteur: 50 })).toBe(false);
  });

  it("relit avec prudence la signature gardée sur l'ordinateur", () => {
    const gardee = { trace: { proportion: 2.5, epaisseur: 0.05, traits: [[{ x: 0.1, y: 0.5 }, { x: 0.9, y: 0.4 }]] }, teinte: "noir" as const };
    expect(lireSignatures(ecrireSignatures({ signature: gardee }))).toEqual({ signature: gardee });
    expect(lireSignatures("")).toEqual({});
    expect(lireSignatures("{abîmé")).toEqual({});
    // Une encre inconnue : bleu marine ; un point hors de la boîte : la signature n'est pas reprise.
    expect(lireSignatures(JSON.stringify({ initiales: { ...gardee, teinte: "jaune" } })).initiales?.teinte).toBe("marine");
    expect(lireSignatures(JSON.stringify({ signature: { ...gardee, trace: { ...gardee.trace, traits: [[{ x: 3, y: 0 }]] } } }))).toEqual({});
  });
});
