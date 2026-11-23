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
const encre: MantineColorsTuple = [
  "#c9ccd1",
  "#b5b9bf",
  "#9aa0a8",
  "#6e747d",
  "#464b54",
  "#3a3f47",
  "#2d3137",
  "#22252a",
  "#1b1e22",
  "#131518",
];

export const theme = createTheme({
  colors: { eau, dark: encre },
  primaryColor: "eau",
  primaryShade: 6,
  black: COULEURS.encre,
  defaultRadius: "md",
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
    "--automat-encart": "var(--mantine-color-gray-0)",
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
    "--automat-encart": "var(--mantine-color-dark-6)",
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
  },
});
