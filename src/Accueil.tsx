import { useMemo, useState } from "react";
import { CloseButton, Grid, Group, Paper, SimpleGrid, Stack, Text, TextInput, Title, UnstyledButton } from "@mantine/core";
import { IconSearch, IconShieldCheck, IconUpload } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { nomDe } from "./donnees/chemins";
import { ilYa, type DocumentRecent } from "./donnees/recents";
import { chercher } from "./donnees/recherche";
import { GROUPE_DE, texteDe, type DefinitionOutil, type Groupe, type Outil } from "./outils";
import { TOUTES_EXTENSIONS } from "./pages/commun/reception";
import Vignette from "./pages/commun/Vignette";
import { ZoneDepot } from "./pages/commun/ZoneFichier";
import { textes } from "./textes";
import { varsTeinte } from "./theme";

const t = textes.accueil;

/** Une tâche : sa pastille (la teinte de son groupe), son nom et ce qu'elle fait, en un seul bouton. */
export function TuileOutil({ definition, onClick }: { definition: DefinitionOutil; onClick: () => void }) {
  const Icone = definition.icone;
  const { titre, description } = textes.outils[definition.cle];
  return (
    <UnstyledButton className="tuile-outil" onClick={onClick} data-tuile={definition.cle}>
      <Paper withBorder p="sm">
        <Group gap="sm" wrap="nowrap" align="center">
          <span className="pastille pastille-moyenne" style={varsTeinte(GROUPE_DE[definition.cle].teinte)}>
            <Icone size={22} stroke={1.7} />
          </span>
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

/** Un groupe d'outils : une carte de sa couleur, son nom en capitales, ses outils en lignes. */
function CarteGroupe({ groupe, onOutil }: { groupe: Groupe; onOutil: (outil: Outil) => void }) {
  const Icone = groupe.icone;
  return (
    <section className="carte-groupe" style={varsTeinte(groupe.teinte)} aria-label={textes.groupes[groupe.cle]} data-groupe={groupe.cle}>
      <div className="entete">
        <span className="etiquette">{textes.groupes[groupe.cle]}</span>
        <span className="tuile-icone" aria-hidden>
          <Icone size={20} stroke={1.8} />
        </span>
      </div>
      {groupe.outils.map((definition) => {
        const IconeOutil = definition.icone;
        const { titre, description } = textes.outils[definition.cle];
        return (
          <UnstyledButton key={definition.cle} className="ligne-outil" onClick={() => onOutil(definition.cle)} data-tuile={definition.cle}>
            <IconeOutil size={24} stroke={1.7} style={{ flexShrink: 0 }} />
            <div style={{ minWidth: 0 }}>
              <div className="titre">{titre}</div>
              <div className="description">{description}</div>
            </div>
          </UnstyledButton>
        );
      })}
    </section>
  );
}

interface Props {
  groupes: Groupe[];
  /** Les derniers PDF ouverts dans le lecteur, du plus récent au plus ancien. */
  recents: DocumentRecent[];
  onLire: (chemin: string) => void;
  onOutil: (outil: Outil) => void;
  /** Fichiers choisis depuis l'accueil : l'application propose quoi en faire. */
  onFichiers: (fichiers: string[]) => void;
}

/** « Reprendre » : les derniers PDF, avec leur première page et depuis quand ils ont été ouverts. */
function PanneauReprendre({ recents, onLire }: { recents: DocumentRecent[]; onLire: (chemin: string) => void }) {
  const maintenant = Date.now();
  return (
    <Paper withBorder p="md" h="100%" data-reprendre>
      <Text fw={750} size="lg" lh={1.2}>
        {textes.barre.reprendre}
      </Text>
      <Text size="xs" c="dimmed" mb="xs">
        {textes.barre.reprendreAide}
      </Text>
      <Stack gap={4}>
        {recents.slice(0, 4).map(({ chemin, ouvert }) => (
          <UnstyledButton key={chemin} className="ligne-reprendre" onClick={() => onLire(chemin)} title={chemin}>
            <Vignette chemin={chemin} page={0} taille={40} largeur={112} alt="" />
            <div style={{ minWidth: 0, flex: 1 }}>
              <Text fw={600} size="sm" truncate="end">
                {nomDe(chemin)}
              </Text>
              <Text size="xs" c="dimmed">
                {ilYa(ouvert, maintenant)}
              </Text>
            </div>
          </UnstyledButton>
        ))}
      </Stack>
    </Paper>
  );
}

/**
 * Accueil : « Que voulez-vous faire ? » et la recherche, avec un bandeau pour choisir ou glisser des
 * fichiers ; à côté, les derniers PDF à reprendre (ou, la première fois, la grande zone de dépôt) ;
 * puis les tâches, une carte de couleur par groupe, dans l'ordre de lecture.
 */
export default function Accueil({ groupes, recents, onLire, onOutil, onFichiers }: Props) {
  const [recherche, setRecherche] = useState("");
  const tous = useMemo(() => groupes.flatMap((groupe) => groupe.outils), [groupes]);
  const trouves = useMemo(() => chercher(recherche, tous, texteDe), [recherche, tous]);

  async function choisir() {
    const choisis = await open({ multiple: true, directory: false, filters: [{ name: t.filtre, extensions: TOUTES_EXTENSIONS }] });
    if (choisis) onFichiers(Array.isArray(choisis) ? choisis : [choisis]);
  }

  return (
    <Stack gap="xl" maw={1240} data-accueil>
      <Grid gap="lg" align="stretch">
        <Grid.Col span={{ base: 12, lg: 7 }}>
          <Stack gap="md" justify="center" h="100%">
            <Title order={1}>{t.question}</Title>
            <TextInput
              size="lg"
              radius="xl"
              aria-label={t.rechercher}
              placeholder={t.exemple}
              leftSection={<IconSearch size={20} />}
              rightSection={recherche ? <CloseButton aria-label={t.effacer} onClick={() => setRecherche("")} /> : null}
              value={recherche}
              onChange={(e) => setRecherche(e.currentTarget.value)}
              data-recherche
            />
            {/* Avec des documents récents, la grande zone de dépôt laisse sa place à « Reprendre » : un bandeau suffit. */}
            {recents.length > 0 && (
              <UnstyledButton className="bandeau-depot" onClick={() => void choisir()} data-bandeau-depot>
                <IconUpload size={20} stroke={1.7} style={{ flexShrink: 0 }} />
                <div style={{ minWidth: 0, flex: 1 }}>
                  <Text fw={600} size="sm">
                    {t.bandeau}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {t.deposerDetail}
                  </Text>
                </div>
                <Text component="span" className="bandeau-bouton" fw={650} size="sm">
                  {t.choisir}
                </Text>
              </UnstyledButton>
            )}
            <Group gap={6} wrap="nowrap">
              <IconShieldCheck size={18} color="var(--automat-confiance)" style={{ flexShrink: 0 }} />
              <Text size="sm" c="dimmed">
                {t.rassurer}
              </Text>
            </Group>
          </Stack>
        </Grid.Col>
        <Grid.Col span={{ base: 12, lg: 5 }}>
          {recents.length > 0 ? (
            <PanneauReprendre recents={recents} onLire={onLire} />
          ) : (
            <ZoneDepot invite={t.deposer} detail={t.deposerDetail} bouton={t.choisir} onChoisir={() => void choisir()} />
          )}
        </Grid.Col>
      </Grid>

      {trouves === null ? (
        <div className="grille-groupes">
          {groupes.map((groupe) => (
            <CarteGroupe key={groupe.cle} groupe={groupe} onOutil={onOutil} />
          ))}
        </div>
      ) : trouves.length === 0 ? (
        <Text c="dimmed" data-aucun-resultat>
          {t.aucun(recherche.trim())}
        </Text>
      ) : (
        <SimpleGrid cols={{ base: 2, lg: 3 }} spacing="sm" verticalSpacing="sm">
          {trouves.map((definition) => (
            <TuileOutil key={definition.cle} definition={definition} onClick={() => onOutil(definition.cle)} />
          ))}
        </SimpleGrid>
      )}
    </Stack>
  );
}
