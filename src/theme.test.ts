import { DEFAULT_THEME, mergeMantineTheme } from "@mantine/core";
import { describe, expect, it } from "vitest";
import { suivreOrdinateur, TEINTES, theme, variables } from "./theme";

const complet = mergeMantineTheme(DEFAULT_THEME, theme);
const { light: clair, dark: sombre } = variables(complet);
const sources = import.meta.glob<string>(["./**/*.{ts,tsx,css}", "!./**/*.test.ts"], { query: "?raw", import: "default", eager: true });

/** Contraste entre deux couleurs « #rrggbb », de 1 à 21 (WCAG : 4,5 au moins pour un texte). */
function contraste(a: string, b: string): number {
  const luminance = (couleur: string) => {
    const [r, v, b] = [1, 3, 5].map((i) => parseInt(couleur.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
    return 0.2126 * r + 0.7152 * v + 0.0722 * b;
  };
  const [claire, foncee] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (claire + 0.05) / (foncee + 0.05);
}

describe("thème clair et sombre", () => {
  it("définit dans les deux thèmes chaque couleur utilisée par l'application", () => {
    const utilisees = new Set(Object.values(sources).flatMap((texte) => texte.match(/--automat-[a-z-]+/g) ?? []));
    expect(utilisees.size).toBeGreaterThan(10);
    for (const nom of utilisees) {
      expect(clair, nom).toHaveProperty([nom]);
      expect(sombre, nom).toHaveProperty([nom]);
    }
    const propres = (couleurs: Record<string, string>) => Object.keys(couleurs).filter((nom) => nom.startsWith("--automat-")).sort();
    expect(propres(sombre)).toEqual(propres(clair));
  });

  it("garde les textes lisibles en sombre comme en clair", () => {
    const gris = complet.colors.dark;
    expect(contraste(gris[0], gris[7])).toBeGreaterThanOrEqual(7); // texte sur le fond
    expect(contraste(gris[2], gris[7])).toBeGreaterThanOrEqual(5.5); // textes secondaires
    expect(contraste(gris[2], gris[6])).toBeGreaterThanOrEqual(4.5); // … dans un champ
    expect(contraste(complet.colors.eau[4], gris[7])).toBeGreaterThanOrEqual(4.5); // liens et textes Eau
    expect(contraste(clair["--mantine-color-dimmed"], "#ffffff")).toBeGreaterThanOrEqual(5.5);
  });

  it("garde lisible le texte des cartes de couleur et de leurs icônes", () => {
    for (const [nom, { fond, tuile, texte }] of Object.entries(TEINTES)) {
      expect(contraste(texte, fond), nom).toBeGreaterThanOrEqual(7);
      expect(contraste(texte, tuile), nom).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("suit toujours le thème de l'ordinateur", () => {
    suivreOrdinateur.set("dark");
    expect(suivreOrdinateur.get("light")).toBe("auto");
  });
});
