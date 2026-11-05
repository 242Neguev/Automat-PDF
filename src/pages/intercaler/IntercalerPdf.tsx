import { useMemo, useState } from "react";
import { Alert, Button, Group, Paper, SimpleGrid, Stack, Switch, Text, Title } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCheck, IconFileSearch, IconLayersIntersect } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { enregistrerPages, pagesPdf } from "../../coeur";
import { textes } from "../../textes";
import { useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";
import Vignette from "../commun/Vignette";
import { oublier } from "../commun/vignettes";
import { intercaler } from "./ordre";

const t = textes.intercalage;
const PAGES_APERCU = 6;
const COTE = 130;

interface Fichier {
  chemin: string;
  nom: string;
  pages: number;
}

/** Intercaler : rectos et versos scannés séparément, réunis page par page. */
export default function IntercalerPdf() {
  const [rectos, setRectos] = useState<Fichier | null>(null);
  const [versos, setVersos] = useState<Fichier | null>(null);
  const [inverses, setInverses] = useState(true);
  const [enCours, setEnCours] = useState(false);
  const ordre = useMemo(() => (rectos && versos ? intercaler(rectos.pages, versos.pages, inverses) : []), [rectos, versos, inverses]);
  const largeur = Math.round(COTE * Math.min(2, window.devicePixelRatio || 1));

  async function choisir(choix: (fichier: Fichier) => void) {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin, choix);
  }

  // Deux PDF : les rectos puis les versos ; un seul : ce qui manque encore.
  useFichiersRecus("intercaler", (fichiers) => {
    const [premier, second] = fichiers;
    if (second) {
      void ouvrir(premier, setRectos);
      void ouvrir(second, setVersos);
    } else void ouvrir(premier, rectos ? setVersos : setRectos);
  });

  async function ouvrir(chemin: string, choix: (fichier: Fichier) => void) {
    try {
      choix({ chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin, pages: (await pagesPdf(chemin)).length });
    } catch (e) {
      signalerErreur(e);
    }
  }

  async function enregistrer() {
    if (!rectos || !versos) return;
    const sortie = await save({ defaultPath: t.nomFichier, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    try {
      await enregistrerPages(
        ordre.map(({ cote, page }) => ({ chemin: (cote === "recto" ? rectos : versos).chemin, page, rotation: 0 })),
        sortie,
      );
      oublier(sortie);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.cree, message: sortie });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  const choixFichier = (titre: string, fichier: Fichier | null, choix: (fichier: Fichier) => void) => (
    <Paper withBorder p="md">
      <Stack gap="xs">
        <Title order={5}>{titre}</Title>
        <Text size="sm" c={fichier ? undefined : "dimmed"}>
          {fichier ? `${fichier.nom} · ${textes.pdf.pages(fichier.pages)}` : t.aucun}
        </Text>
        <Button variant={fichier ? "light" : "filled"} leftSection={<IconFileSearch size={18} />} onClick={() => choisir(choix)} w="fit-content" aria-label={`${titre} : ${fichier ? t.changer : t.choisir}`}>
          {fichier ? t.changer : t.choisir}
        </Button>
      </Stack>
    </Paper>
  );

  return (
    <Stack gap="md" maw={900}>
      <SimpleGrid cols={2}>
        {choixFichier(t.rectos, rectos, setRectos)}
        {choixFichier(t.versos, versos, setVersos)}
      </SimpleGrid>
      <Switch label={t.inverses} description={t.inversesAide} checked={inverses} onChange={(e) => setInverses(e.currentTarget.checked)} />

      {rectos && versos && (
        <Paper withBorder p="md">
          <Stack gap="sm">
            {rectos.pages !== versos.pages && <Alert color="orange">{t.ecart(rectos.pages, versos.pages)}</Alert>}
            <Group justify="space-between">
              <Title order={5}>{t.apercu}</Title>
              <Text size="sm" c="dimmed">
                {t.total(ordre.length)}
              </Text>
            </Group>
            <Group gap="sm" data-apercu-intercale>
              {ordre.slice(0, PAGES_APERCU).map(({ cote, page }, i) => (
                <Stack key={`${cote}-${page}`} gap={4} align="center">
                  <Vignette chemin={(cote === "recto" ? rectos : versos).chemin} page={page} taille={COTE} largeur={largeur} alt={t.legende(cote, page + 1)} />
                  <Text size="xs" c="dimmed">
                    {i + 1}. {t.legende(cote, page + 1)}
                  </Text>
                </Stack>
              ))}
            </Group>
            <Button leftSection={<IconLayersIntersect size={18} />} loading={enCours} onClick={enregistrer} w="fit-content">
              {t.intercaler}
            </Button>
          </Stack>
        </Paper>
      )}
    </Stack>
  );
}
