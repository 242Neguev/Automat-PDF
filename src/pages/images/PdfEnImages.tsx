import { useState } from "react";
import { Button, Group, Modal, Paper, SegmentedControl, Select, Stack, Text } from "@mantine/core";
import { IconLayoutGrid, IconPhoto } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { pagesPdf, pdfEnImages } from "../../coeur";
import { dossierDe, nomDe } from "../../donnees/chemins";
import { ecrirePlages } from "../../donnees/plages";
import { textes } from "../../textes";
import Action from "../commun/Action";
import BarreAction from "../commun/BarreAction";
import ChoixPages from "../commun/ChoixPages";
import { useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import Vignette from "../commun/Vignette";
import { ZoneFichier } from "../commun/ZoneFichier";

const t = textes.enImages;

/** PDF en images : une image PNG ou JPG par page, à la qualité choisie. */
export default function PdfEnImages() {
  const [source, setSource] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [choix, setChoix] = useState<"toutes" | "certaines">("toutes");
  // Pages choisies en les voyant (« Certaines pages ») ; la première, au départ.
  const [choisies, setChoisies] = useState<number[]>([0]);
  const [vue, setVue] = useState(0);
  const [choixOuvert, setChoixOuvert] = useState(false);
  const [format, setFormat] = useState<"png" | "jpeg">("png");
  const [resolution, setResolution] = useState("150");
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
  const nombre = !source ? 0 : choix === "toutes" ? source.pages : choisies.length;

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      setSource({ chemin, nom: nomDe(chemin), pages: (await pagesPdf(chemin)).length });
      setChoisies([0]);
      setVue(0);
      setCree(null);
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("enImages", (fichiers) => void ouvrir(fichiers[0]));

  async function creer() {
    if (!source || nombre === 0) return;
    const dossier = await open({ directory: true, multiple: false, title: t.choisirDossier, defaultPath: dossierDe(source.chemin) || undefined });
    if (typeof dossier !== "string") return;
    setEnCours(true);
    setCree(null);
    try {
      const cible = await pdfEnImages(source.chemin, choix === "toutes" ? [] : choisies, format, Number(resolution), dossier);
      setCree({ titre: t.cree(nombre), chemin: cible, dossier: true });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={760}>
      <ZoneFichier
        fichier={source ? { nom: source.nom, details: textes.pdf.pages(source.pages) } : null}
        invite={textes.depot.pdf}
        bouton={t.choisir}
        changer={t.autre}
        onChoisir={() => void choisir()}
      />

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
                <Group gap="sm" align="center">
                  <Text size="sm" data-pages-choisies>
                    {choisies.length > 0 ? t.choisies(ecrirePlages(choisies)) : t.aucune}
                  </Text>
                  <Button size="xs" variant="light" leftSection={<IconLayoutGrid size={14} />} onClick={() => setChoixOuvert(true)}>
                    {t.voirPages}
                  </Button>
                </Group>
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
                  w={240}
                />
              </Group>
              <Text size="sm" fw={500}>
                {t.resume(nombre)}
              </Text>
              <BarreAction>
                <Action icone={IconPhoto} enCours={enCours} raison={nombre === 0 ? t.raison : null} onClick={() => void creer()}>
                  {t.creer}
                </Action>
              </BarreAction>
            </Stack>
          </Group>
        </Paper>
      )}
      <Resultat cree={cree} onFermer={() => setCree(null)} />
      <Modal opened={choixOuvert && source !== null} onClose={() => setChoixOuvert(false)} size="90%" title={t.plages}>
        {source && <ChoixPages chemin={source.chemin} total={source.pages} choisies={choisies} onChoisies={setChoisies} vue={vue} onVue={setVue} />}
        <Group justify="flex-end" mt="md">
          <Button onClick={() => setChoixOuvert(false)}>{t.valider}</Button>
        </Group>
      </Modal>
    </Stack>
  );
}
