import { useMemo, useState } from "react";
import { Button, Group, Paper, SegmentedControl, Select, Stack, Text, TextInput } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCheck, IconFileSearch, IconPhoto } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { pagesPdf, pdfEnImages } from "../../coeur";
import { lirePlages } from "../../donnees/plages";
import { textes } from "../../textes";
import { useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";
import Vignette from "../commun/Vignette";

const t = textes.enImages;

/** PDF en images : une image PNG ou JPG par page, à la résolution choisie. */
export default function PdfEnImages() {
  const [source, setSource] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [choix, setChoix] = useState<"toutes" | "certaines">("toutes");
  const [plages, setPlages] = useState("1");
  const [format, setFormat] = useState<"png" | "jpeg">("png");
  const [resolution, setResolution] = useState("150");
  const [enCours, setEnCours] = useState(false);
  const lecture = useMemo(() => (source && choix === "certaines" ? lirePlages(plages, source.pages) : null), [source, choix, plages]);
  const nombre = !source ? 0 : !lecture ? source.pages : "pages" in lecture ? lecture.pages.length : 0;

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      setSource({ chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin, pages: (await pagesPdf(chemin)).length });
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("enImages", (fichiers) => void ouvrir(fichiers[0]));

  async function creer() {
    if (!source || (lecture && "erreur" in lecture)) return;
    const dossier = await open({ directory: true, multiple: false, title: t.choisirDossier });
    if (typeof dossier !== "string") return;
    setEnCours(true);
    try {
      const cible = await pdfEnImages(source.chemin, lecture?.pages ?? [], format, Number(resolution), dossier);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.cree, message: cible, autoClose: 10000 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={760}>
      <Group>
        <Button variant={source ? "light" : "filled"} leftSection={<IconFileSearch size={18} />} onClick={choisir}>
          {source ? t.autre : t.choisir}
        </Button>
        {source && (
          <Text size="sm">
            {source.nom} · {textes.pdf.pages(source.pages)}
          </Text>
        )}
      </Group>

      {source && (
        <Paper withBorder p="md">
          <Group align="flex-start" gap="xl" wrap="nowrap">
            <Vignette chemin={source.chemin} page={0} taille={140} largeur={280} alt={source.nom} />
            <Stack gap="sm" style={{ flex: 1 }}>
              <Stack gap={4}>
                <Text size="sm" fw={500}>
                  {t.pages}
                </Text>
                <SegmentedControl value={choix} onChange={(valeur) => setChoix(valeur as typeof choix)} data={(["toutes", "certaines"] as const).map((value) => ({ value, label: t[value] }))} />
              </Stack>
              {choix === "certaines" && (
                <TextInput
                  label={t.plages}
                  description={t.plagesAide}
                  value={plages}
                  onChange={(e) => setPlages(e.currentTarget.value)}
                  error={lecture && "erreur" in lecture ? lecture.erreur : undefined}
                />
              )}
              <Group gap="xl" align="flex-start">
                <Stack gap={4}>
                  <Text size="sm" fw={500}>
                    {t.format}
                  </Text>
                  <SegmentedControl value={format} onChange={(valeur) => setFormat(valeur as typeof format)} data={(["png", "jpeg"] as const).map((value) => ({ value, label: t.formats[value] }))} />
                </Stack>
                <Select
                  label={t.resolution}
                  value={resolution}
                  allowDeselect={false}
                  onChange={(valeur) => valeur && setResolution(valeur)}
                  data={(["96", "150", "300"] as const).map((value) => ({ value, label: t.resolutions[value] }))}
                  w={220}
                />
              </Group>
              <Text size="sm" fw={500}>
                {t.resume(nombre)}
              </Text>
              <Button leftSection={<IconPhoto size={18} />} loading={enCours} disabled={nombre === 0} onClick={creer} w="fit-content">
                {t.creer}
              </Button>
            </Stack>
          </Group>
        </Paper>
      )}
    </Stack>
  );
}
