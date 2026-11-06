import type { ReactNode } from "react";
import { Group, Paper, Stack, Text, ThemeIcon, Title } from "@mantine/core";

interface Props {
  numero: number;
  titre: string;
  aide?: string;
  children: ReactNode;
}

/** Une étape numérotée d'un outil qui en compte plusieurs : « 1. Le PDF à séparer ». */
export default function Etape({ numero, titre, aide, children }: Props) {
  return (
    <Paper withBorder p="md" data-etape={numero}>
      <Stack gap="sm">
        <Group gap="sm" wrap="nowrap" align="flex-start">
          <ThemeIcon radius="xl" size={30} style={{ flexShrink: 0 }}>
            <Text fw={700} size="sm" c="white">
              {numero}
            </Text>
          </ThemeIcon>
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
