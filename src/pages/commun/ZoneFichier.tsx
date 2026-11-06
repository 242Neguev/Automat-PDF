import { Button, Group, Paper, Stack, Text, ThemeIcon } from "@mantine/core";
import { IconFileSearch, IconFileTypePdf, IconUpload } from "@tabler/icons-react";
import { textes } from "../../textes";

type Icone = typeof IconUpload;

interface PropsDepot {
  /** Ce que l'on attend : « Glissez votre PDF ici ». */
  invite: string;
  /** Précision sous l'invite (ce qui se passera, les formats acceptés…). */
  detail?: string;
  /** Bouton pour choisir les fichiers : « Choisir le PDF… ». */
  bouton: string;
  icone?: Icone;
  enCours?: boolean;
  onChoisir: () => void;
}

/**
 * Grande zone en pointillés, là où l'on n'a encore rien choisi : on y glisse ses fichiers (la
 * fenêtre entière les accepte), ou l'on clique pour les choisir.
 */
export function ZoneDepot({ invite, detail, bouton, icone: Icone = IconUpload, enCours, onChoisir }: PropsDepot) {
  return (
    <Paper withBorder p="xl" radius="md" className="zone-depot" onClick={() => !enCours && onChoisir()} data-zone-depot>
      <Stack align="center" gap={6}>
        <ThemeIcon variant="light" size={52} radius="xl">
          <Icone size={28} stroke={1.6} />
        </ThemeIcon>
        <Text fw={600} size="lg" ta="center">
          {invite}
        </Text>
        {detail && (
          <Text size="sm" c="dimmed" ta="center" maw={520}>
            {detail}
          </Text>
        )}
        <Text size="sm" c="dimmed">
          {textes.depot.ou}
        </Text>
        <Button
          leftSection={<IconFileSearch size={18} />}
          loading={enCours}
          onClick={(evenement) => {
            evenement.stopPropagation();
            onChoisir();
          }}
        >
          {bouton}
        </Button>
      </Stack>
    </Paper>
  );
}

interface PropsFichier extends PropsDepot {
  /** Le fichier choisi : son nom, et ce qu'on en sait (nombre de pages, taille…). */
  fichier: { nom: string; details?: string } | null;
  /** Bouton pour en choisir un autre : « Choisir un autre PDF… ». */
  changer: string;
  iconeFichier?: Icone;
}

/** Le fichier sur lequel travailler : la zone de dépôt tant qu'il n'est pas choisi, puis son nom. */
export function ZoneFichier({ fichier, changer, iconeFichier: IconeFichier = IconFileTypePdf, ...depot }: PropsFichier) {
  if (!fichier) return <ZoneDepot {...depot} />;
  return (
    <Paper withBorder p="sm" radius="md" data-fichier-choisi>
      <Group justify="space-between" wrap="nowrap" gap="sm">
        <Group gap="sm" wrap="nowrap" style={{ minWidth: 0 }}>
          <ThemeIcon variant="light" size={38} radius="md">
            <IconeFichier size={22} stroke={1.6} />
          </ThemeIcon>
          <div style={{ minWidth: 0 }}>
            <Text fw={600} truncate>
              {fichier.nom}
            </Text>
            {fichier.details && (
              <Text size="sm" c="dimmed">
                {fichier.details}
              </Text>
            )}
          </div>
        </Group>
        <Button variant="light" leftSection={<IconFileSearch size={18} />} loading={depot.enCours} onClick={depot.onChoisir} style={{ flexShrink: 0 }}>
          {changer}
        </Button>
      </Group>
    </Paper>
  );
}
