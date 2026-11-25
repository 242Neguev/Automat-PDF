import { describe, expect, it } from "vitest";
import { aDesSousSignets, ouvertsAuDepart, signetCourant, signetsVisibles, type SignetPlace } from "./signets";

const s = (niveau: number, page: number | null, ouvert = false): SignetPlace => ({ niveau, ouvert, destination: page === null ? null : { page } });
// 0 Introduction (p. 1) ; 1 Contexte (p. 2) ; 2 Détail (p. 3) ; 1 Enjeux (p. 4) ; 0 Annexe (p. 9) ; 0 Site (pas de page)
const SOMMAIRE = [s(0, 0, true), s(1, 1), s(2, 2), s(1, 3), s(0, 8), s(0, null)];

describe("signets", () => {
  it("sait lesquels ont des sous-signets, et lesquels sont dépliés au départ", () => {
    expect(SOMMAIRE.map((_, rang) => aDesSousSignets(SOMMAIRE, rang))).toEqual([true, true, false, false, false, false]);
    expect([...ouvertsAuDepart(SOMMAIRE)]).toEqual([0]);
  });

  it("ne montre que les signets dont les parents sont dépliés", () => {
    expect(signetsVisibles(SOMMAIRE, new Set())).toEqual([0, 4, 5]);
    expect(signetsVisibles(SOMMAIRE, new Set([0]))).toEqual([0, 1, 3, 4, 5]);
    expect(signetsVisibles(SOMMAIRE, new Set([0, 1]))).toEqual([0, 1, 2, 3, 4, 5]);
    expect(signetsVisibles(SOMMAIRE, new Set([1])), "Parent replié : ses petits-enfants restent cachés.").toEqual([0, 4, 5]);
  });

  it("trouve le signet de la page lue", () => {
    expect(signetCourant(SOMMAIRE, 0)).toBe(0);
    expect(signetCourant(SOMMAIRE, 2)).toBe(2);
    expect(signetCourant(SOMMAIRE, 6)).toBe(3);
    expect(signetCourant(SOMMAIRE, 20)).toBe(4);
    expect(signetCourant([s(0, 5)], 1), "Avant le premier signet : aucun.").toBeNull();
  });
});
