import { useMemo, useState } from "react";
import { Group, Paper, SegmentedControl, Select, Stack, Text, TextInput } from "@mantine/core";
import { IconPhoto } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { pagesPdf, pdfEnImages } from "../../coeur";
import { dossierDe, nomDe } from "../../donnees/chemins";
import { lirePlages } from "../../donnees/plages";
import { textes } from "../../textes";
import Action from "../commun/Action";
import BarreAction from "../commun/BarreAction";
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
  const [plages, setPlages] = useState("1");
  const [format, setFormat] = useState<"png" | "jpeg">("png");
  const [resolution, setResolution] = useState("150");
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
  const lecture = useMemo(() => (source && choix === "certaines" ? lirePlages(plages, source.pages) : null), [source, choix, plages]);
  const nombre = !source ? 0 : !lecture ? source.pages : "pages" in lecture ? lecture.pages.length : 0;

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      setSource({ chemin, nom: nomDe(chemin), pages: (await pagesPdf(chemin)).length });
      setCree(null);
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("enImages", (fichiers) => void ouvrir(fichiers[0]));

  async function creer() {
    if (!source || (lecture && "erreur" in lecture)) return;
    const dossier = await open({ directory: true, multiple: false, title: t.choisirDossier, defaultPath: dossierDe(source.chemin) || undefined });
    if (typeof dossier !== "string") return;
    setEnCours(true);
    setCree(null);
    try {
      const cible = await pdfEnImages(source.chemin, lecture?.pages ?? [], format, Number(resolution), dossier);
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
    </Stack>
  );
}
