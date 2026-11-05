import { createTheme, type MantineColorsTuple } from "@mantine/core";

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

export const theme = createTheme({
  colors: { eau },
  primaryColor: "eau",
  primaryShade: 6,
  black: COULEURS.encre,
  defaultRadius: "md",
});
