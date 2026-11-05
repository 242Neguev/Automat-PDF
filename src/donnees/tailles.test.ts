import { describe, expect, it } from "vitest";
import { formaterTaille } from "./tailles";

describe("tailles de fichiers", () => {
  it("choisit l'unité et arrondit à la française", () => {
    expect(formaterTaille(512)).toBe("512 o");
    expect(formaterTaille(850 * 1024)).toBe("850 Ko");
    expect(formaterTaille(12.44 * 1024 * 1024)).toBe("12,4 Mo");
    expect(formaterTaille(3 * 1024 * 1024)).toBe("3 Mo");
    expect(formaterTaille(1.5 * 1024 ** 3)).toBe("1,5 Go");
  });
});
