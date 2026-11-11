import { useState } from "react";
import { ActionIcon, Button, Group, Paper, Stack, Text, Tooltip } from "@mantine/core";
import { IconArrowDown, IconArrowUp, IconFilePlus, IconFiles, IconTrash } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { fusionnerPdf, infosPdf } from "../../coeur";
import { aCote, nomDe } from "../../donnees/chemins";
import { textes } from "../../textes";
import Action from "../commun/Action";
import BarreAction from "../commun/BarreAction";
import { useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { ZoneDepot } from "../commun/ZoneFichier";

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
  const [cree, setCree] = useState<Cree | null>(null);
  const total = sources.reduce((somme, source) => somme + source.pages, 0);
  const raison = sources.length === 0 ? t.fusion.raisons.aucun : sources.length === 1 ? t.fusion.raisons.un : null;

  async function ajouter() {
    const choisis = await open({ multiple: true, directory: false, filters: [{ name: t.filtre, extensions: ["pdf"] }] });
    if (choisis) await ajouterChemins(Array.isArray(choisis) ? choisis : [choisis]);
  }

  async function ajouterChemins(chemins: string[]) {
    for (const chemin of chemins) {
      try {
        const pages = await infosPdf(chemin);
        setSources((actuelles) => [...actuelles, { chemin, nom: nomDe(chemin), pages }]);
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
    const sortie = await save({ defaultPath: aCote(sources[0]?.chemin, t.fusion.nomFichier), filters: [{ name: t.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    setCree(null);
    try {
      await fusionnerPdf(
        sources.map((source) => source.chemin),
        sortie,
      );
      setCree({ titre: t.fusion.cree, chemin: sortie, details: [t.fusion.total(sources.length, total)] });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={760}>
      {sources.length === 0 ? (
        <ZoneDepot invite={textes.depot.pdfs} detail={t.fusion.aide} bouton={t.fusion.ajouter} onChoisir={() => void ajouter()} />
      ) : (
        <>
          <Text size="sm" c="dimmed">
            {t.fusion.aide}
          </Text>
          <Paper withBorder p="sm">
            <Stack gap={4}>
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
            <Button variant="light" leftSection={<IconFilePlus size={18} />} onClick={() => void ajouter()}>
              {t.fusion.ajouter}
            </Button>
            <Text size="sm" c="dimmed">
              {t.fusion.total(sources.length, total)}
            </Text>
          </Group>
        </>
      )}
      <BarreAction>
        <Action icone={IconFiles} enCours={enCours} raison={raison} onClick={() => void fusionner()}>
          {t.fusion.fusionner}
        </Action>
      </BarreAction>
      <Resultat cree={cree} onFermer={() => setCree(null)} />
    </Stack>
  );
}
