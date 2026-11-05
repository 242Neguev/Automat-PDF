import { describe, expect, it } from "vitest";
import { intercaler } from "../intercaler/ordre";
import { deplacer, pivoter, plage, type PageTravail } from "./pages";

const doc = (n: number): PageTravail[] => Array.from({ length: n }, (_, i) => ({ id: `p${i + 1}`, source: 0, page: i, rotation: 0 }));
const ids = (pages: PageTravail[]) => pages.map((page) => page.id).join(" ");

describe("organisation des pages", () => {
  it("déplace une page à la place d'une autre", () => {
    expect(ids(deplacer(doc(5), ["p1"], "p1", "p4"))).toBe("p2 p3 p4 p1 p5");
    expect(ids(deplacer(doc(5), ["p5"], "p5", "p2"))).toBe("p1 p5 p2 p3 p4");
  });

  it("déplace un groupe de pages sélectionnées, dans leur ordre", () => {
    expect(ids(deplacer(doc(6), ["p2", "p5"], "p5", "p1"))).toBe("p2 p5 p1 p3 p4 p6");
    expect(ids(deplacer(doc(6), ["p1", "p2"], "p1", "p6"))).toBe("p3 p4 p5 p6 p1 p2");
  });

  it("tourne d'un quart de tour, dans les deux sens", () => {
    const tournees = pivoter(pivoter(doc(2), new Set(["p2"]), -1), new Set(["p1", "p2"]), 1);
    expect(tournees.map((page) => page.rotation)).toEqual([90, 0]);
    expect(pivoter(doc(1), new Set(["p1"]), -1)[0].rotation).toBe(270);
  });

  it("sélectionne une plage dans les deux sens", () => {
    expect(plage(doc(5), "p4", "p2")).toEqual(["p2", "p3", "p4"]);
    expect(plage(doc(5), "inconnue", "p2")).toEqual(["p2"]);
  });
});

describe("rectos et versos intercalés", () => {
  const texte = (pages: ReturnType<typeof intercaler>) => pages.map((p) => `${p.cote[0]}${p.page + 1}`).join(" ");

  it("remet les versos scannés à l'envers dans l'ordre", () => {
    expect(texte(intercaler(3, 3, true))).toBe("r1 v3 r2 v2 r3 v1");
    expect(texte(intercaler(3, 3, false))).toBe("r1 v1 r2 v2 r3 v3");
  });

  it("ajoute à la fin les pages en trop", () => {
    expect(texte(intercaler(3, 1, false))).toBe("r1 v1 r2 r3");
    expect(texte(intercaler(1, 2, false))).toBe("r1 v1 v2");
  });
});
