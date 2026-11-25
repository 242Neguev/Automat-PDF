import type { ReactNode } from "react";
import { Group, Paper, Stack, Text, Title } from "@mantine/core";

interface Props {
  numero: number;
  titre: string;
  aide?: string;
  children: ReactNode;
}

/**
 * Une étape numérotée d'un outil qui en compte plusieurs : « 1. Le PDF à séparer ». Son numéro prend la
 * teinte du groupe de l'outil (variables --teinte-* posées par la page de l'outil, voir App.tsx).
 */
export default function Etape({ numero, titre, aide, children }: Props) {
  return (
    <Paper withBorder p="md" data-etape={numero}>
      <Stack gap="sm">
        <Group gap="sm" wrap="nowrap" align="flex-start">
          <span className="numero-etape">{numero}</span>
          <div>
            <Title order={4}>{titre}</Title>
            {aide && (
              <Text size="sm" c="dimmed">
                {aide}
              </Text>
            )}
          </div>
        </Group>
        {children}
      </Stack>
    </Paper>
  );
}
