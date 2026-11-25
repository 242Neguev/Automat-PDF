import { describe, expect, it } from "vitest";
import { aProposer } from "./lecteur-par-defaut";

describe("proposer Automat-PDF comme lecteur de PDF", () => {
  const pasEncore = { possible: true, parDefaut: false };

  it("le propose quand le système le permet et que ce n'est pas déjà fait", () => {
    expect(aProposer(pasEncore, false, false)).toBe(true);
  });

  it("ne le propose ni avant d'avoir la réponse du système, ni quand c'est impossible ou déjà fait", () => {
    expect(aProposer(null, false, false)).toBe(false);
    expect(aProposer({ possible: false, parDefaut: false }, false, false)).toBe(false);
    expect(aProposer({ possible: true, parDefaut: true }, false, false)).toBe(false);
  });

  it("respecte « Ne plus proposer » et « Plus tard »", () => {
    expect(aProposer(pasEncore, true, false)).toBe(false);
    expect(aProposer(pasEncore, false, true)).toBe(false);
  });
});
