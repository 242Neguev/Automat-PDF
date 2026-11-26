import { Alert, Button, Group, Text } from "@mantine/core";
import { IconTextScan2 } from "@tabler/icons-react";
import { textes } from "../../textes";

const t = textes.lecteur.numerise;

/**
 * Bandeau d'un PDF qui semble numérisé (aucun texte sur ses premières pages) : on ne peut ni y
 * chercher ni en copier le texte. « Reconnaître le texte » ouvre l'outil avec ce PDF.
 */
export function BandeauNumerise({ onReconnaitre }: { onReconnaitre: (() => void) | null }) {
  return (
    <Alert color="blue" variant="light" icon={<IconTextScan2 size={20} />} py={8} data-bandeau-numerise>
      <Group justify="space-between" gap="sm" wrap="nowrap">
        <Text size="sm" style={{ minWidth: 0 }}>
          {t.bandeau}
        </Text>
        {onReconnaitre && (
          <Button size="xs" radius="xl" leftSection={<IconTextScan2 size={15} />} onClick={onReconnaitre} style={{ flexShrink: 0 }} data-reconnaitre-texte>
            {t.reconnaitre}
          </Button>
        )}
      </Group>
    </Alert>
  );
}
