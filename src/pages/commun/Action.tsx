import type { ReactNode } from "react";
import { Button, Group, Text } from "@mantine/core";
import type { IconCheck } from "@tabler/icons-react";

interface Props {
  icone: typeof IconCheck;
  children: ReactNode;
  onClick: () => void;
  enCours?: boolean;
  /** Pourquoi le bouton est grisé (« Ajoutez au moins deux PDF. ») ; absente : il est actif. */
  raison?: string | null;
  /** Grisé sans explication, le temps d'un autre travail (un envoi en cours…). */
  desactive?: boolean;
  couleur?: string;
  variante?: "filled" | "light";
  taille?: "sm" | "md";
}

/** Le bouton qui lance le travail ; grisé, il dit ce qu'il manque. */
export default function Action({ icone: Icone, children, onClick, enCours, raison, desactive, couleur, variante = "filled", taille = "md" }: Props) {
  return (
    <Group gap="sm" wrap="nowrap" align="center">
      <Button
        size={taille}
        variant={variante}
        color={couleur}
        leftSection={<Icone size={18} />}
        loading={enCours}
        disabled={Boolean(raison) || desactive}
        onClick={onClick}
        style={{ flexShrink: 0 }}
      >
        {children}
      </Button>
      {raison && (
        <Text size="sm" c="dimmed" data-raison>
          {raison}
        </Text>
      )}
    </Group>
  );
}
