import { describe, expect, it } from "vitest";
import { estMac } from "./systeme";

describe("estMac", () => {
  it("reconnaît la vue web de macOS, pas celle de Windows", () => {
    expect(estMac("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)")).toBe(true);
    expect(estMac("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36 Edg/142.0.0.0")).toBe(false);
    expect(estMac("Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15")).toBe(false);
    expect(estMac("")).toBe(false);
  });
});
