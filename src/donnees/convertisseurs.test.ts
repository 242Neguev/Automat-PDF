import { describe, expect, it } from "vitest";
import { besoinLibreoffice, dureeLisible } from "./convertisseurs";

const poste = { word: false, office: false, libreoffice: false, installable: true };

describe("convertisseurs", () => {
  it("disent si LibreOffice manque ou s'il est seulement conseillé", () => {
    expect(besoinLibreoffice(null)).toBeNull();
    expect(besoinLibreoffice(poste)).toBe("manque");
    expect(besoinLibreoffice({ ...poste, word: true, office: true })).toBe("conseille");
    expect(besoinLibreoffice({ ...poste, libreoffice: true })).toBeNull();
    expect(besoinLibreoffice({ ...poste, office: true, libreoffice: true })).toBeNull();
  });

  it("donnent une durée lisible", () => {
    expect(dureeLisible(0)).toBe("0 s");
    expect(dureeLisible(45.8)).toBe("45 s");
    expect(dureeLisible(125)).toBe("2 min 05 s");
    expect(dureeLisible(-3)).toBe("0 s");
  });
});
