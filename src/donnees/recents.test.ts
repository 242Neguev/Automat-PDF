import { describe, expect, it } from "vitest";
import { ajouterRecent, ecrireRecents, ilYa, lireRecents, NOMBRE_RECENTS, retirerRecent, type DocumentRecent } from "./recents";

const memoire = () => {
  const valeurs = new Map<string, string>();
  return { getItem: (cle: string) => valeurs.get(cle) ?? null, setItem: (cle: string, valeur: string) => void valeurs.set(cle, valeur) };
};

describe("documents récents", () => {
  it("met le dernier ouvert en tête, sans doublon", () => {
    let liste: DocumentRecent[] = [];
    liste = ajouterRecent(liste, "/a.pdf", 1);
    liste = ajouterRecent(liste, "/b.pdf", 2);
    liste = ajouterRecent(liste, "/a.pdf", 3);
    expect(liste).toEqual([
      { chemin: "/a.pdf", ouvert: 3 },
      { chemin: "/b.pdf", ouvert: 2 },
    ]);
    expect(retirerRecent(liste, "/a.pdf")).toEqual([{ chemin: "/b.pdf", ouvert: 2 }]);
  });

  it(`n'en garde que ${NOMBRE_RECENTS}`, () => {
    let liste: DocumentRecent[] = [];
    for (let i = 0; i < NOMBRE_RECENTS + 3; i++) liste = ajouterRecent(liste, `/doc${i}.pdf`, i);
    expect(liste).toHaveLength(NOMBRE_RECENTS);
    expect(liste[0].chemin).toBe(`/doc${NOMBRE_RECENTS + 2}.pdf`);
  });

  it("garde la liste, et la relit telle quelle", () => {
    const stockage = memoire();
    const liste = ajouterRecent([], "C:\\Paie\\octobre.pdf", 42);
    ecrireRecents(liste, stockage);
    expect(lireRecents(stockage)).toEqual(liste);
  });

  it("dit depuis quand un document a été ouvert", () => {
    const maintenant = new Date(2026, 10, 25, 15, 30).getTime();
    const il = (date: Date) => ilYa(date.getTime(), maintenant);
    expect(il(new Date(2026, 10, 25, 15, 29, 40))).toBe("à l'instant");
    expect(il(new Date(2026, 10, 25, 15, 18))).toBe("il y a 12 min");
    expect(il(new Date(2026, 10, 25, 9, 10))).toBe("il y a 6 h");
    expect(il(new Date(2026, 10, 24, 23, 50))).toBe("hier");
    expect(il(new Date(2026, 10, 21, 10, 0))).toBe("il y a 4 jours");
    expect(il(new Date(2026, 10, 3, 10, 0))).toBe("le 03/11");
  });

  it("ignore une liste abîmée, ou un stockage indisponible", () => {
    const stockage = memoire();
    stockage.setItem("automat-pdf.documents-recents", "{pas du JSON");
    expect(lireRecents(stockage)).toEqual([]);
    stockage.setItem("automat-pdf.documents-recents", JSON.stringify([{ chemin: "/ok.pdf", ouvert: 1 }, { chemin: 3 }, null, "texte"]));
    expect(lireRecents(stockage)).toEqual([{ chemin: "/ok.pdf", ouvert: 1 }]);
    expect(lireRecents(null)).toEqual([]);
    expect(() =>
      ecrireRecents([], {
        setItem: () => {
          throw new Error("plein");
        },
      }),
    ).not.toThrow();
  });
});
