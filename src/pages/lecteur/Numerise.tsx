import { Alert, Button, CloseButton, Group, Progress, Stack, Text } from "@mantine/core";
import { IconCircleCheck, IconDeviceFloppy, IconTextScan2 } from "@tabler/icons-react";
import type { BilanReconnaissance } from "../../coeur";
import type { AvancementReconnaissance } from "../commun/reconnaissance";
import { textes } from "../../textes";

const t = textes.lecteur.numerise;

interface Props {
  /** Reconnaissance en cours (pages lues, pages à lire ; null : elle démarre). */
  enCours: boolean;
  avancement: AvancementReconnaissance | null;
  /** Ce qu'a donné la reconnaissance, une fois faite. */
  bilan: BilanReconnaissance | null;
  enregistrement: boolean;
  onReconnaitre: () => void;
  onEnregistrer: () => void;
  onFermer: () => void;
}

/**
 * Bandeau d'un PDF numérisé (aucun texte sur ses premières pages) : on ne peut ni y chercher ni en
 * copier le texte. « Reconnaître le texte » le reconnaît en mémoire, page par page ; ensuite, la
 * recherche et la copie marchent, et le PDF peut être enregistré avec son texte.
 */
export function BandeauNumerise({ enCours, avancement, bilan, enregistrement, onReconnaitre, onEnregistrer, onFermer }: Props) {
  if (bilan) {
    return (
      <Alert color="green" variant="light" icon={<IconCircleCheck size={20} />} py={8} data-bandeau-numerise="fait">
        <Group justify="space-between" gap="sm" wrap="nowrap">
          <Text size="sm" style={{ minWidth: 0 }}>
            {t.fait(bilan.pagesReconnues)}
          </Text>
          <Group gap="xs" wrap="nowrap" style={{ flexShrink: 0 }}>
            <Button size="xs" radius="xl" leftSection={<IconDeviceFloppy size={15} />} loading={enregistrement} onClick={onEnregistrer} data-enregistrer-texte>
              {t.enregistrer}
            </Button>
            <CloseButton size="sm" aria-label={t.fermer} onClick={onFermer} />
          </Group>
        </Group>
      </Alert>
    );
  }
  return (
    <Alert color="blue" variant="light" icon={<IconTextScan2 size={20} />} py={8} data-bandeau-numerise={enCours ? "en-cours" : "propose"}>
      {enCours ? (
        <Stack gap={6}>
          <Text size="sm">{avancement ? t.enCours(avancement[0], avancement[1]) : t.demarrage}</Text>
          <Progress value={avancement ? (100 * avancement[0]) / Math.max(1, avancement[1]) : 0} animated={!avancement} />
        </Stack>
      ) : (
        <Group justify="space-between" gap="sm" wrap="nowrap">
          <Text size="sm" style={{ minWidth: 0 }}>
            {t.bandeau}
          </Text>
          <Button size="xs" radius="xl" leftSection={<IconTextScan2 size={15} />} onClick={onReconnaitre} style={{ flexShrink: 0 }} data-reconnaitre-texte>
            {t.reconnaitre}
          </Button>
        </Group>
      )}
    </Alert>
  );
}
