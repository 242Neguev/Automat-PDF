import { useMemo, useState } from "react";
import { CloseButton, Grid, Group, Paper, SimpleGrid, Stack, Text, TextInput, ThemeIcon, Title, UnstyledButton } from "@mantine/core";
import { IconSearch, IconShieldCheck } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { chercher } from "./donnees/recherche";
import { texteDe, type DefinitionOutil, type Groupe, type Outil } from "./outils";
import { TOUTES_EXTENSIONS } from "./pages/commun/reception";
import { ZoneDepot } from "./pages/commun/ZoneFichier";
import { textes } from "./textes";

const t = textes.accueil;

/** Une tâche : son icône, son nom et ce qu'elle fait, en un seul bouton. */
export function TuileOutil({ definition, onClick }: { definition: DefinitionOutil; onClick: () => void }) {
  const Icone = definition.icone;
  const { titre, description } = textes.outils[definition.cle];
  return (
    <UnstyledButton className="tuile-outil" onClick={onClick} data-tuile={definition.cle}>
      <Paper withBorder p="sm">
        <Group gap="sm" wrap="nowrap" align="center">
          <ThemeIcon variant="light" size={42} radius="md" style={{ flexShrink: 0 }}>
            <Icone size={24} stroke={1.6} />
          </ThemeIcon>
          <div style={{ minWidth: 0 }}>
            <Text fw={600} lh={1.3}>
              {titre}
            </Text>
            <Text size="sm" c="dimmed" lh={1.3}>
              {description}
            </Text>
          </div>
        </Group>
      </Paper>
    </UnstyledButton>
  );
}

/** Rangées de tuiles d'un groupe (deux tuiles par rangée). */
const rangees = (groupe: Groupe) => Math.ceil(groupe.outils.length / 2);

/** Les groupes répartis en deux colonnes de hauteurs voisines, sans changer leur ordre. */
function colonnes(groupes: Groupe[]): [Groupe[], Groupe[]] {
  const total = groupes.reduce((somme, groupe) => somme + rangees(groupe) + 1, 0);
  let meilleure = 0;
  let ecart = Infinity;
  let cumul = 0;
  groupes.forEach((groupe, i) => {
    cumul += rangees(groupe) + 1;
    if (Math.abs(total - 2 * cumul) < ecart) {
      ecart = Math.abs(total - 2 * cumul);
      meilleure = i + 1;
    }
  });
  return [groupes.slice(0, meilleure), groupes.slice(meilleure)];
}

interface Props {
  groupes: Groupe[];
  onOutil: (outil: Outil) => void;
  /** Fichiers choisis depuis l'accueil : l'application propose quoi en faire. */
  onFichiers: (fichiers: string[]) => void;
}

/** Accueil : « Que voulez-vous faire ? », les tâches en grand, une recherche et une zone de dépôt. */
export default function Accueil({ groupes, onOutil, onFichiers }: Props) {
  const [recherche, setRecherche] = useState("");
  const tous = useMemo(() => groupes.flatMap((groupe) => groupe.outils), [groupes]);
  const trouves = useMemo(() => chercher(recherche, tous, texteDe), [recherche, tous]);
  const [gauche, droite] = useMemo(() => colonnes(groupes), [groupes]);

  async function choisir() {
    const choisis = await open({ multiple: true, directory: false, filters: [{ name: t.filtre, extensions: TOUTES_EXTENSIONS }] });
    if (choisis) onFichiers(Array.isArray(choisis) ? choisis : [choisis]);
  }

  const tuiles = (outils: DefinitionOutil[], colonnesTuiles: number | Record<string, number>) => (
    <SimpleGrid cols={colonnesTuiles} spacing="sm" verticalSpacing="sm">
      {outils.map((definition) => (
        <TuileOutil key={definition.cle} definition={definition} onClick={() => onOutil(definition.cle)} />
      ))}
    </SimpleGrid>
  );

  const groupe = ({ cle, outils }: Groupe) => (
    <div key={cle}>
      <Text size="xs" fw={700} c="dimmed" tt="uppercase" mb={6}>
        {textes.groupes[cle]}
      </Text>
      {tuiles(outils, 2)}
    </div>
  );

  return (
    <Stack gap="lg" maw={1180} data-accueil>
      <Grid gap="lg" align="stretch">
        <Grid.Col span={{ base: 12, lg: 7 }}>
          <Stack gap="sm" justify="center" h="100%">
            <Title order={2}>{t.question}</Title>
            <Text>{t.intro}</Text>
            <TextInput
              size="md"
              aria-label={t.rechercher}
              placeholder={t.exemple}
              leftSection={<IconSearch size={18} />}
              rightSection={recherche ? <CloseButton aria-label={t.effacer} onClick={() => setRecherche("")} /> : null}
              value={recherche}
              onChange={(e) => setRecherche(e.currentTarget.value)}
              data-recherche
            />
            <Group gap={6} wrap="nowrap">
              <IconShieldCheck size={18} color="var(--mantine-color-teal-7)" style={{ flexShrink: 0 }} />
              <Text size="sm" c="dimmed">
                {t.rassurer}
              </Text>
            </Group>
          </Stack>
        </Grid.Col>
        <Grid.Col span={{ base: 12, lg: 5 }}>
          <ZoneDepot invite={t.deposer} detail={t.deposerDetail} bouton={t.choisir} onChoisir={() => void choisir()} />
        </Grid.Col>
      </Grid>

      {trouves === null ? (
        <SimpleGrid cols={{ base: 1, lg: 2 }} spacing="xl" verticalSpacing="md">
          <Stack gap="md">{gauche.map(groupe)}</Stack>
          <Stack gap="md">{droite.map(groupe)}</Stack>
        </SimpleGrid>
      ) : trouves.length === 0 ? (
        <Text c="dimmed" data-aucun-resultat>
          {t.aucun(recherche.trim())}
        </Text>
      ) : (
        tuiles(trouves, { base: 2, lg: 3 })
      )}
    </Stack>
  );
}
