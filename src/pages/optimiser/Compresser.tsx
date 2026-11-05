import { useState } from "react";
import { Button, Group, Paper, SegmentedControl, Stack, Text, Title } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconArrowsMinimize, IconCheck, IconFileSearch } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { compresserPdf, tailleFichier } from "../../coeur";
import { formaterTaille } from "../../donnees/tailles";
import { textes } from "../../textes";
import { useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";

const t = textes.compression;
type Niveau = "legere" | "moyenne" | "forte";

/** Compresser : un PDF plus léger (images réduites, doublons retirés), à la qualité choisie. */
export default function Compresser() {
  const [source, setSource] = useState<{ chemin: string; nom: string; taille: number } | null>(null);
  const [niveau, setNiveau] = useState<Niveau>("moyenne");
  const [enCours, setEnCours] = useState(false);

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      setSource({ chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin, taille: await tailleFichier(chemin) });
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("compresser", (fichiers) => void ouvrir(fichiers[0]));

  async function compresser() {
    if (!source) return;
    const sortie = await save({ defaultPath: t.nomFichier(source.nom.replace(/\.pdf$/i, "")), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    try {
      const bilan = await compresserPdf(source.chemin, niveau, sortie);
      const gain = Math.round((1 - bilan.apres / Math.max(1, bilan.avant)) * 100);
      notifications.show({
        color: "teal",
        icon: <IconCheck size={18} />,
        title: t.cree,
        message: bilan.apres < bilan.avant ? t.resultat(formaterTaille(bilan.avant), formaterTaille(bilan.apres), gain, bilan.images) : t.dejaCompact,
        autoClose: 10000,
      });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={640}>
      <Group>
        <Button variant={source ? "light" : "filled"} leftSection={<IconFileSearch size={18} />} onClick={choisir}>
          {source ? t.autre : t.choisir}
        </Button>
        {source && (
          <Text size="sm">
            {source.nom} · {formaterTaille(source.taille)}
          </Text>
        )}
      </Group>
      {source && (
        <Paper withBorder p="md">
          <Stack gap="sm">
            <Title order={5}>{t.niveau}</Title>
            <SegmentedControl value={niveau} onChange={(v) => setNiveau(v as Niveau)} data={(["legere", "moyenne", "forte"] as const).map((value) => ({ value, label: t.niveaux[value] }))} />
            <Text size="sm" c="dimmed">
              {t.niveauxAide[niveau]}
            </Text>
            <Button leftSection={<IconArrowsMinimize size={18} />} loading={enCours} onClick={compresser} w="fit-content">
              {t.compresser}
            </Button>
          </Stack>
        </Paper>
      )}
    </Stack>
  );
}
