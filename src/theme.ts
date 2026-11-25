import type { CSSProperties } from "react";
import { createTheme, type CSSVariablesResolver, type MantineColorScheme, type MantineColorSchemeManager, type MantineColorsTuple } from "@mantine/core";

/** Palette de l'éditeur, Eau sèche : Encre, Eau (couleur d'accent) et Sec. */
export const COULEURS = {
  encre: "#22252A",
  eau: "#1E6E72",
  sec: "#EFEDE8",
};

// Nuances du bleu-vert « Eau », du plus clair au plus foncé (la nuance 6 est la couleur de référence).
const eau: MantineColorsTuple = [
  "#eff5f5",
  "#dbe8e8",
  "#bcd4d5",
  "#9abec0",
  "#74a5a8",
  "#4b8b8e",
  "#1e6e72",
  "#1a5e61",
  "#154d50",
  "#113d3f",
];

// Thème sombre : les gris de l'Encre, du plus clair (le texte) au plus foncé. La nuance 7, le fond,
// est l'Encre elle-même ; la 2, celle des textes secondaires, garde un contraste de 5,8 sur ce fond.
// Le texte (nuance 0) est presque blanc : les titres ressortent sur les panneaux de verre.
const encre: MantineColorsTuple = [
  "#e6e8eb",
  "#c3c7cd",
  "#9aa0a8",
  "#6e747d",
  "#464b54",
  "#3a3f47",
  "#2d3137",
  "#22252a",
  "#1b1e22",
  "#131518",
];

/**
 * Teintes des groupes d'outils : cartes de l'accueil, pastilles du menu et du lecteur. Des pastels
 * vifs, les mêmes en clair et en sombre, avec un texte foncé de leur couleur (contraste de 7 au
 * moins, vérifié par theme.test.ts) ; `tuile` est le fond de l'icône posée sur la carte.
 */
export const TEINTES = {
  jaune: { fond: "#ffe27a", tuile: "#f4c93e", texte: "#2b2200" },
  orange: { fond: "#ffae6b", tuile: "#f58f42", texte: "#2e1403" },
  lavande: { fond: "#cdb4ff", tuile: "#ae8df6", texte: "#1f1140" },
  citron: { fond: "#c8f27a", tuile: "#a5da45", texte: "#1a2a04" },
  ciel: { fond: "#9ad9ff", tuile: "#6dc2f4", texte: "#05253a" },
  rose: { fond: "#ffb8d2", tuile: "#f591b8", texte: "#3a0a1e" },
  menthe: { fond: "#8eedd2", tuile: "#55d5b0", texte: "#03291f" },
} as const;
export type Teinte = keyof typeof TEINTES;

/** « r, v, b » d'une couleur « #rrvvbb », pour l'écrire avec une transparence : rgba(var(--teinte-rvb), 0.2). */
const rvb = (couleur: string) => [1, 3, 5].map((i) => parseInt(couleur.slice(i, i + 2), 16)).join(", ");

/**
 * Variables CSS d'une teinte, à poser sur l'élément qui la porte : la carte d'un groupe, la pastille
 * d'un outil, la page d'un outil (son halo, les numéros de ses étapes).
 */
export const varsTeinte = (teinte: Teinte) =>
  ({
    "--teinte-fond": TEINTES[teinte].fond,
    "--teinte-tuile": TEINTES[teinte].tuile,
    "--teinte-texte": TEINTES[teinte].texte,
    "--teinte-rvb": rvb(TEINTES[teinte].fond),
  }) as CSSProperties;

/**
 * Halos de couleur du fond de la fenêtre (lavande en haut à gauche, bleu ciel à droite, or en bas, une
 * pointe de menthe), comme une photo floue derrière du verre ; plus vifs en sombre qu'en clair.
 */
const HALOS_SOMBRES =
  "radial-gradient(1000px 620px at 6% -10%, rgba(174, 141, 246, 0.42), transparent 60%), radial-gradient(900px 700px at 105% 0%, rgba(109, 194, 244, 0.32), transparent 55%), radial-gradient(1100px 700px at 60% 120%, rgba(244, 201, 62, 0.22), transparent 60%), radial-gradient(700px 500px at 30% 70%, rgba(85, 213, 176, 0.14), transparent 60%)";
const HALOS_CLAIRS =
  "radial-gradient(1000px 620px at 6% -10%, rgba(174, 141, 246, 0.4), transparent 60%), radial-gradient(900px 700px at 105% 0%, rgba(109, 194, 244, 0.38), transparent 55%), radial-gradient(1100px 700px at 60% 120%, rgba(244, 201, 62, 0.32), transparent 60%)";

/** Polices du système : SF Pro sur Mac, Segoe UI sous Windows ; rien à embarquer. */
const POLICES = '-apple-system, BlinkMacSystemFont, "Segoe UI Variable Text", "Segoe UI", Inter, Roboto, "Helvetica Neue", Arial, sans-serif';

export const theme = createTheme({
  colors: { eau, dark: encre },
  primaryColor: "eau",
  primaryShade: 6,
  black: COULEURS.encre,
  defaultRadius: "md",
  // De grands arrondis, comme des cartes posées sur du verre.
  radius: { xs: "6px", sm: "8px", md: "12px", lg: "20px", xl: "28px" },
  fontFamily: POLICES,
  headings: {
    fontFamily: POLICES.replace("Segoe UI Variable Text", "Segoe UI Variable Display"),
    fontWeight: "700",
    sizes: {
      h1: { fontSize: "2.25rem", lineHeight: "1.15" },
      h2: { fontSize: "1.9rem", lineHeight: "1.2" },
      h3: { fontSize: "1.3rem", lineHeight: "1.25" },
      h4: { fontSize: "1.05rem", lineHeight: "1.3" },
    },
  },
  components: {
    Paper: { defaultProps: { radius: "lg" } },
    Modal: { defaultProps: { radius: "lg", overlayProps: { backgroundOpacity: 0.5, blur: 6 } } },
  },
});

/**
 * Le thème suit celui de l'ordinateur, clair ou sombre, et change avec lui : rien n'est gardé, aucun
 * choix ne vient le figer.
 */
export const suivreOrdinateur: MantineColorSchemeManager = {
  get: (): MantineColorScheme => "auto",
  set: () => {},
  subscribe: () => {},
  unsubscribe: () => {},
  clear: () => {},
};

/**
 * Couleurs de l'application qui changent avec le thème. Textes secondaires (aides, explications) plus
 * foncés que le gris de Mantine en clair : lisibles par tous, contraste de 5,8 contre 3,3 sur fond
 * blanc ; en sombre, le gris clair de l'Encre (nuance 2 ci-dessus), 5,8 aussi.
 */
export const variables: CSSVariablesResolver = () => ({
  variables: {},
  light: {
    "--mantine-color-dimmed": "#5f666d",
    // Choix en cours, survol : fond teinté d'Eau et bordure Eau.
    "--automat-accent-fond": "var(--mantine-color-eau-0)",
    "--automat-accent-bord": "var(--mantine-color-eau-6)",
    // Pages affichées (aperçus) : blanches dans les deux thèmes, détachées du fond.
    "--automat-ombre-page": "0 1px 4px rgba(34, 37, 42, 0.25)",
    "--automat-ombre-barre": "0 -10px 12px -10px rgba(34, 37, 42, 0.4)",
    "--automat-voile": "rgba(255, 255, 255, 0.85)",
    // Encart creusé dans le verre (pages du lecteur, enchaînements) : une ombre légère du fond.
    "--automat-encart": "rgba(34, 37, 42, 0.045)",
    "--automat-neutre": "var(--mantine-color-gray-2)",
    // Feuilles schématiques de « Pages par feuille ».
    "--automat-feuille": "var(--mantine-color-white)",
    "--automat-feuille-page": "var(--mantine-color-eau-1)",
    "--automat-feuille-page-bord": "var(--mantine-color-eau-4)",
    "--automat-feuille-page-texte": "var(--mantine-color-eau-8)",
    "--automat-feuille-vide": "var(--mantine-color-gray-3)",
    // Logo « PDF » de l'en-tête, logos d'Eau sèche (encre sur fond clair).
    "--automat-logo-fond": COULEURS.encre,
    "--automat-logo-texte": COULEURS.sec,
    "--automat-filtre-logo": "none",
    "--automat-confiance": "var(--mantine-color-teal-7)",
    // Fond de la fenêtre : un gris très clair traversé de halos de couleur, comme une photo floue
    // derrière du verre ; les panneaux de verre posés dessus sont presque blancs.
    "--automat-fond": `${HALOS_CLAIRS}, #e8ebf0`,
    // Mac : la fenêtre est transparente, macOS floute le bureau derrière ; les halos y passent sur un voile léger.
    "--automat-fond-verre": `${HALOS_CLAIRS}, rgba(240, 242, 246, 0.12)`,
    // Verre dépoli (choix du 25/11/2026) : panneaux nacrés, bord et liseré de lumière en haut.
    "--automat-verre": "rgba(255, 255, 255, 0.5)",
    "--automat-verre-fort": "rgba(255, 255, 255, 0.6)",
    "--automat-verre-bord": "rgba(255, 255, 255, 0.85)",
    "--automat-reflet": "rgba(255, 255, 255, 0.9)",
    // Panneaux d'un outil, voilés de la teinte de son groupe (choix du 25/11/2026).
    "--automat-teinte-panneau": "0.22",
    "--automat-teinte-bord": "0.75",
    "--automat-ombre-carte": "0 10px 28px rgba(34, 37, 42, 0.08), 0 1px 3px rgba(34, 37, 42, 0.06)",
    "--automat-actif-fond": "rgba(34, 37, 42, 0.07)",
    "--automat-survol-fond": "rgba(34, 37, 42, 0.05)",
    // Halo de la teinte d'un outil derrière son en-tête : plus marqué sur le fond clair.
    "--automat-halo-force": "0.55",
  },
  dark: {
    // Éléments teintés d'Eau (outil choisi dans le menu, icônes des tâches, boutons secondaires,
    // encadrés) : une Eau éclaircie, plutôt que le bleu-vert presque noir de Mantine.
    "--mantine-color-eau-light": "rgba(116, 165, 168, 0.15)",
    "--mantine-color-eau-light-hover": "rgba(116, 165, 168, 0.22)",
    "--mantine-color-eau-light-color": "var(--mantine-color-eau-2)",
    "--automat-accent-fond": "rgba(116, 165, 168, 0.12)",
    "--automat-accent-bord": "var(--mantine-color-eau-4)",
    "--automat-ombre-page": "0 0 0 1px rgba(255, 255, 255, 0.1), 0 2px 8px rgba(0, 0, 0, 0.5)",
    "--automat-ombre-barre": "0 -10px 12px -10px rgba(0, 0, 0, 0.8)",
    "--automat-voile": "rgba(34, 37, 42, 0.88)",
    "--automat-encart": "rgba(0, 0, 0, 0.22)",
    "--automat-neutre": "var(--mantine-color-dark-4)",
    "--automat-feuille": "var(--mantine-color-dark-6)",
    "--automat-feuille-page": "rgba(116, 165, 168, 0.2)",
    "--automat-feuille-page-bord": "var(--mantine-color-eau-5)",
    "--automat-feuille-page-texte": "var(--mantine-color-eau-2)",
    "--automat-feuille-vide": "var(--mantine-color-dark-4)",
    // En sombre, le logo s'inverse (Sec sur Encre) ; les logos d'Eau sèche passent en clair, et leur
    // Eau s'éclaircit sans changer de teinte.
    "--automat-logo-fond": COULEURS.sec,
    "--automat-logo-texte": COULEURS.encre,
    "--automat-filtre-logo": "invert(1) hue-rotate(180deg)",
    "--automat-confiance": "var(--mantine-color-teal-4)",
    // Fond de la fenêtre : l'Encre, plus profonde, traversée de halos de couleur ; les panneaux de
    // verre l'éclaircissent à peine.
    "--automat-fond": `${HALOS_SOMBRES}, #121418`,
    "--automat-fond-verre": `${HALOS_SOMBRES}, rgba(18, 20, 24, 0.16)`,
    "--automat-verre": "rgba(255, 255, 255, 0.075)",
    "--automat-verre-fort": "rgba(30, 33, 39, 0.55)",
    "--automat-verre-bord": "rgba(255, 255, 255, 0.14)",
    "--automat-reflet": "rgba(255, 255, 255, 0.14)",
    "--automat-teinte-panneau": "0.075",
    "--automat-teinte-bord": "0.3",
    "--automat-ombre-carte": "0 12px 32px rgba(0, 0, 0, 0.35), 0 1px 2px rgba(0, 0, 0, 0.3)",
    "--automat-actif-fond": "rgba(255, 255, 255, 0.1)",
    "--automat-survol-fond": "rgba(255, 255, 255, 0.06)",
    "--automat-halo-force": "0.2",
  },
});
