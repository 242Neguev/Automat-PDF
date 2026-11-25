import { describe, expect, it } from "vitest";
import { destination } from "./destination";

// Quelques outils, avec ce qu'ils prennent, comme dans src/outils.tsx.
const OUTILS = [
  { cle: "organiser", accepte: ["pdf"] },
  { cle: "images", accepte: ["image"] },
  { cle: "signature", accepte: ["pdf", "image"] },
  { cle: "compresser", accepte: ["pdf"] },
  { cle: "officeEnPdf", accepte: ["office"] },
];
const sorte = (fichier: string) =>
  fichier.endsWith(".pdf") ? "pdf" : fichier.endsWith(".jpg") ? "image" : fichier.endsWith(".docx") ? "office" : "autre";
const prend = (outil: (typeof OUTILS)[number], fichier: string) => outil.accepte.includes(sorte(fichier));
const vers = (fichiers: string[], demande: string | null, affichee: string) => destination(fichiers, demande, affichee, OUTILS, prend);
const cles = (d: ReturnType<typeof vers>) => (d.sorte === "choix" ? d.outils.map((o) => o.cle) : d.sorte === "outil" ? d.outil.cle : d);

describe("destination des fichiers reçus", () => {
  it("ouvre un seul PDF dans le lecteur, depuis l'accueil comme depuis le lecteur", () => {
    expect(vers(["C:\\Paie\\octobre.pdf"], null, "accueil")).toEqual({ sorte: "lecteur", chemin: "C:\\Paie\\octobre.pdf" });
    expect(vers(["/Users/rh/Paie.PDF"], null, "lecteur")).toEqual({ sorte: "lecteur", chemin: "/Users/rh/Paie.PDF" });
  });

  it("n'interrompt pas l'outil affiché ni l'outil demandé par le clic droit", () => {
    expect(cles(vers(["a.pdf"], null, "compresser"))).toBe("compresser");
    expect(cles(vers(["a.pdf"], "organiser", "accueil"))).toBe("organiser");
    expect(cles(vers(["a.pdf"], "organiser", "lecteur"))).toBe("organiser");
  });

  it("ouvre dans le lecteur un PDF que l'outil affiché ne prend pas", () => {
    expect(vers(["a.pdf"], null, "images")).toEqual({ sorte: "lecteur", chemin: "a.pdf" });
  });

  it("pose la question pour plusieurs PDF, ou pour un fichier que plusieurs outils prennent", () => {
    expect(cles(vers(["a.pdf", "b.pdf"], null, "accueil"))).toEqual(["organiser", "signature", "compresser"]);
    expect(cles(vers(["photo.jpg"], null, "accueil"))).toEqual(["images", "signature"]);
    expect(cles(vers(["photo.jpg"], null, "compresser"))).toEqual(["images", "signature"]);
  });

  it("choisit d'office le seul outil possible depuis l'accueil", () => {
    expect(cles(vers(["lettre.docx"], null, "accueil"))).toBe("officeEnPdf");
    expect(cles(vers(["lettre.docx"], null, "lecteur"))).toEqual(["officeEnPdf"]);
  });

  it("signale les fichiers qu'aucun outil ne prend", () => {
    expect(vers(["notes.zip"], null, "accueil")).toEqual({ sorte: "aucun" });
  });
});
