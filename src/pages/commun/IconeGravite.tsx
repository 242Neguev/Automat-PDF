import { ThemeIcon } from "@mantine/core";
import { IconAlertCircle, IconAlertTriangle, IconCheck, IconInfoSmall } from "@tabler/icons-react";

/** Gravité d'une anomalie : à corriger, à vérifier, ou simple information. */
export type Gravite = "erreur" | "attention" | "info";

const STYLES: Record<Gravite | "ok", { couleur: string; icone: typeof IconCheck }> = {
  erreur: { couleur: "red", icone: IconAlertTriangle },
  attention: { couleur: "orange", icone: IconAlertCircle },
  info: { couleur: "blue", icone: IconInfoSmall },
  ok: { couleur: "teal", icone: IconCheck },
};

/** Pastille de gravité d'un problème (coche verte : rien à signaler). */
export default function IconeGravite({ gravite, taille = 18 }: { gravite: Gravite | null; taille?: number }) {
  const { couleur, icone: Icone } = STYLES[gravite ?? "ok"];
  return (
    <ThemeIcon color={couleur} variant="light" radius="xl" size={taille}>
      <Icone size={Math.round(taille * 0.7)} />
    </ThemeIcon>
  );
}
