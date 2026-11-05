import { useState } from "react";
import { ActionIcon, Button, Group, Paper, Stack, Text, Tooltip } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconArrowDown, IconArrowUp, IconCheck, IconFilePlus, IconFiles, IconTrash } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { fusionnerPdf, infosPdf } from "../../coeur";
import { textes } from "../../textes";
import { useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";

const t = textes.pdf;

interface Source {
  chemin: string;
  nom: string;
  pages: number;
}

/** Fusionner plusieurs PDF, dans l'ordre choisi. */
export default function FusionPdf() {
  const [sources, setSources] = useState<Source[]>([]);
  const [enCours, setEnCours] = useState(false);
  const total = sources.reduce((somme, source) => somme + source.pages, 0);

  async function ajouter() {
    const choisis = await open({ multiple: true, directory: false, filters: [{ name: t.filtre, extensions: ["pdf"] }] });
    if (choisis) await ajouterChemins(Array.isArray(choisis) ? choisis : [choisis]);
  }

  async function ajouterChemins(chemins: string[]) {
    for (const chemin of chemins) {
      try {
        const pages = await infosPdf(chemin);
        setSources((actuelles) => [...actuelles, { chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin, pages }]);
      } catch (e) {
        signalerErreur(e);
      }
    }
  }

  useFichiersRecus("fusionner", (fichiers) => void ajouterChemins(fichiers));

  const deplacer = (index: number, sens: -1 | 1) =>
    setSources((actuelles) => {
      const liste = [...actuelles];
      [liste[index], liste[index + sens]] = [liste[index + sens], liste[index]];
      return liste;
    });

  async function fusionner() {
    const sortie = await save({ defaultPath: t.fusion.nomFichier, filters: [{ name: t.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    try {
      await fusionnerPdf(
        sources.map((source) => source.chemin),
        sortie,
      );
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.fusion.cree, message: sortie });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={720}>
      <Text size="sm" c="dimmed">
        {t.fusion.aide}
      </Text>
      <Paper withBorder p="sm">
        <Stack gap={4}>
          {sources.length === 0 && (
            <Text size="sm" c="dimmed" p="xs">
              {t.fusion.vide}
            </Text>
          )}
          {sources.map((source, index) => (
            <Group key={`${source.chemin}-${index}`} gap="xs" wrap="nowrap" data-source-pdf>
              <Text size="sm" w={28} c="dimmed" ta="right">
                {index + 1}.
              </Text>
              <Text size="sm" truncate style={{ flex: 1, minWidth: 0 }}>
                {source.nom}
              </Text>
              <Text size="xs" c="dimmed" w={70}>
                {t.pages(source.pages)}
              </Text>
              <Tooltip label={t.monter}>
                <ActionIcon variant="subtle" color="gray" aria-label={t.monter} disabled={index === 0} onClick={() => deplacer(index, -1)}>
                  <IconArrowUp size={16} />
                </ActionIcon>
              </Tooltip>
              <Tooltip label={t.descendre}>
                <ActionIcon variant="subtle" color="gray" aria-label={t.descendre} disabled={index === sources.length - 1} onClick={() => deplacer(index, 1)}>
                  <IconArrowDown size={16} />
                </ActionIcon>
              </Tooltip>
              <Tooltip label={t.retirer}>
                <ActionIcon variant="subtle" color="red" aria-label={t.retirer} onClick={() => setSources((actuelles) => actuelles.filter((_, i) => i !== index))}>
                  <IconTrash size={16} />
                </ActionIcon>
              </Tooltip>
            </Group>
          ))}
        </Stack>
      </Paper>
      <Group justify="space-between">
        <Button variant="light" leftSection={<IconFilePlus size={18} />} onClick={ajouter}>
          {t.fusion.ajouter}
        </Button>
        {sources.length > 0 && (
          <Text size="sm" c="dimmed">
            {t.fusion.total(sources.length, total)}
          </Text>
        )}
      </Group>
      <Button leftSection={<IconFiles size={18} />} disabled={sources.length < 2} loading={enCours} onClick={fusionner} w="fit-content">
        {t.fusion.fusionner}
      </Button>
    </Stack>
  );
}
