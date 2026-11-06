import { useEffect, useMemo, useState } from "react";
import {
  ActionIcon,
  Badge,
  Box,
  Group,
  Loader,
  NumberInput,
  Paper,
  SegmentedControl,
  Select,
  SimpleGrid,
  Slider,
  Stack,
  Switch,
  Text,
  TextInput,
  Title,
  Tooltip,
  UnstyledButton,
} from "@mantine/core";
import { IconChevronLeft, IconChevronRight, IconStack2 } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { apercuMarques, marquerPdf, pagesPdf, type OptionsMarques, type OptionsNumeros, type OptionsTampon, type Position } from "../../coeur";
import { aCote, nomDe } from "../../donnees/chemins";
import { pagesSelon, type ChoixPages } from "../../donnees/plages";
import { textes } from "../../textes";
import Action from "../commun/Action";
import { useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { ZoneFichier } from "../commun/ZoneFichier";

const t = textes.marques;
const POSITIONS: Position[] = ["haut-gauche", "haut-centre", "haut-droite", "bas-gauche", "bas-centre", "bas-droite"];
const LARGEUR_APERCU = 420;

/** Choix de la position du numéro : une page schématique avec six emplacements. */
function ChoixPosition({ valeur, onValeur }: { valeur: Position; onValeur: (position: Position) => void }) {
  return (
    <Box w={96} h={124} p={6} style={{ border: "1px solid var(--mantine-color-gray-4)", borderRadius: 3, display: "flex", flexDirection: "column", justifyContent: "space-between" }}>
      {[POSITIONS.slice(0, 3), POSITIONS.slice(3)].map((rangee, i) => (
        <SimpleGrid key={i} cols={3} spacing={4}>
          {rangee.map((position) => (
            <Tooltip key={position} label={t.positions[position]} withArrow>
              <UnstyledButton
                aria-label={t.positions[position]}
                aria-pressed={position === valeur}
                onClick={() => onValeur(position)}
                h={18}
                style={{
                  borderRadius: 2,
                  background: position === valeur ? "var(--mantine-color-eau-6)" : "var(--mantine-color-gray-2)",
                }}
              />
            </Tooltip>
          ))}
        </SimpleGrid>
      ))}
    </Box>
  );
}

/** Numéros et tampon : numéros de page, tampon ou filigrane, avec un aperçu fidèle de la page. */
export default function NumerosTampon() {
  const [source, setSource] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [avecNumeros, setAvecNumeros] = useState(true);
  const [numeros, setNumeros] = useState<OptionsNumeros>({ format: "page-n-sur-total", position: "bas-centre", taille: 10, premier: 1, margeMm: 10 });
  const [avecTampon, setAvecTampon] = useState(false);
  const [tampon, setTampon] = useState<OptionsTampon>({ texte: "COPIE", style: "filigrane", couleur: "gris", opacite: 0.25 });
  const [choix, setChoix] = useState<ChoixPages>("toutes");
  const [plages, setPlages] = useState("2-");
  const [page, setPage] = useState(0);
  const [apercu, setApercu] = useState<string | null>(null);
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
  const selection = source ? pagesSelon(choix, plages, source.pages, page) : null;
  const options: OptionsMarques | null = useMemo(
    () =>
      (avecNumeros || avecTampon) && selection && "pages" in selection
        ? { numeros: avecNumeros ? numeros : null, tampon: avecTampon ? tampon : null, pages: selection.pages }
        : null,
    [avecNumeros, avecTampon, numeros, tampon, selection && "pages" in selection ? selection.pages.join(",") : null],
  );

  // Aperçu fidèle de la page affichée, recalculé peu après chaque changement.
  useEffect(() => {
    if (!source || !options) return;
    let actuel = true;
    const minuteur = setTimeout(() => {
      apercuMarques(source.chemin, page, options, LARGEUR_APERCU * 2).then(
        (octets) =>
          actuel &&
          setApercu((precedent) => {
            if (precedent) URL.revokeObjectURL(precedent);
            return URL.createObjectURL(new Blob([octets], { type: "image/jpeg" }));
          }),
        (e) => actuel && signalerErreur(e),
      );
    }, 250);
    return () => {
      actuel = false;
      clearTimeout(minuteur);
    };
  }, [source, page, options]);

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      setSource({ chemin, nom: nomDe(chemin), pages: (await pagesPdf(chemin)).length });
      setPage(0);
      setApercu(null);
      setCree(null);
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("marques", (fichiers) => void ouvrir(fichiers[0]));

  async function creer() {
    if (!source || !options) return;
    const sortie = await save({ defaultPath: aCote(source.chemin, t.nomFichier(source.nom.replace(/\.pdf$/i, ""))), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    setCree(null);
    try {
      await marquerPdf(source.chemin, options, sortie);
      setCree({ titre: t.cree, chemin: sortie });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  const changerTampon = (modification: Partial<OptionsTampon>) => setTampon((actuel) => ({ ...actuel, ...modification }));
  const raison = !source ? t.raisons.pdf : !avecNumeros && !avecTampon ? t.rien : !options ? t.raisons.pages : null;

  return (
    <Stack gap="md">
      <Box maw={720}>
        <ZoneFichier
          fichier={source ? { nom: source.nom, details: textes.pdf.pages(source.pages) } : null}
          invite={textes.depot.pdf}
          bouton={t.choisir}
          changer={t.autre}
          onChoisir={() => void choisir()}
        />
      </Box>

      <Group align="flex-start" gap="lg" wrap="nowrap">
        <Stack gap="md" style={{ flex: 1, minWidth: 0 }} maw={620}>
          <Paper withBorder p="md">
            <Stack gap="sm">
              <Switch label={<Title order={5}>{t.numeros}</Title>} checked={avecNumeros} onChange={(e) => setAvecNumeros(e.currentTarget.checked)} />
              {avecNumeros && (
                <Group align="flex-start" gap="lg">
                  <Stack gap={4}>
                    <Text size="sm" fw={500}>
                      {t.position}
                    </Text>
                    <ChoixPosition valeur={numeros.position} onValeur={(position) => setNumeros((n) => ({ ...n, position }))} />
                  </Stack>
                  <Stack gap="xs" style={{ flex: 1 }}>
                    <Select
                      label={t.format}
                      value={numeros.format}
                      allowDeselect={false}
                      onChange={(format) => format && setNumeros((n) => ({ ...n, format: format as OptionsNumeros["format"] }))}
                      data={(Object.keys(t.formats) as OptionsNumeros["format"][]).map((value) => ({ value, label: t.formats[value] }))}
                    />
                    <Group grow>
                      <NumberInput label={t.taille} min={6} max={36} value={numeros.taille} onChange={(v) => typeof v === "number" && setNumeros((n) => ({ ...n, taille: v }))} />
                      <NumberInput label={t.premier} min={0} value={numeros.premier} onChange={(v) => typeof v === "number" && setNumeros((n) => ({ ...n, premier: v }))} />
                    </Group>
                  </Stack>
                </Group>
              )}
            </Stack>
          </Paper>

          <Paper withBorder p="md">
            <Stack gap="sm">
              <Switch label={<Title order={5}>{t.tampon}</Title>} checked={avecTampon} onChange={(e) => setAvecTampon(e.currentTarget.checked)} />
              {avecTampon && (
                <>
                  <TextInput label={t.texte} value={tampon.texte} onChange={(e) => changerTampon({ texte: e.currentTarget.value })} />
                  <Group gap={6}>
                    {t.suggestions.map((suggestion) => (
                      <Badge key={suggestion} variant={suggestion === tampon.texte ? "filled" : "light"} style={{ cursor: "pointer" }} onClick={() => changerTampon({ texte: suggestion })}>
                        {suggestion}
                      </Badge>
                    ))}
                  </Group>
                  <SegmentedControl
                    value={tampon.style}
                    onChange={(style) => changerTampon({ style: style as OptionsTampon["style"], opacite: style === "filigrane" ? 0.25 : 0.9 })}
                    data={(["filigrane", "tampon"] as const).map((value) => ({ value, label: t.styles[value] }))}
                  />
                  <Group align="flex-end" gap="lg">
                    <Stack gap={4}>
                      <Text size="sm" fw={500}>
                        {t.couleur}
                      </Text>
                      <SegmentedControl
                        size="xs"
                        value={tampon.couleur}
                        onChange={(couleur) => changerTampon({ couleur: couleur as OptionsTampon["couleur"] })}
                        data={(["gris", "rouge", "bleu", "vert"] as const).map((value) => ({ value, label: t.couleurs[value] }))}
                      />
                    </Stack>
                    <Stack gap={4} style={{ flex: 1 }} miw={160}>
                      <Text size="sm" fw={500}>
                        {t.opacite}
                      </Text>
                      <Slider min={10} max={100} step={5} value={Math.round(tampon.opacite * 100)} onChange={(v) => changerTampon({ opacite: v / 100 })} label={(v) => `${v} %`} />
                    </Stack>
                  </Group>
                </>
              )}
            </Stack>
          </Paper>

          <Paper withBorder p="md">
            <Stack gap="xs">
              <Title order={5}>{t.pages}</Title>
              <SegmentedControl value={choix} onChange={(v) => setChoix(v as ChoixPages)} data={(["toutes", "sauf-premiere", "certaines"] as const).map((value) => ({ value, label: t.choixPages[value] }))} />
              {choix === "certaines" && (
                <TextInput label={t.plages} description={t.plagesAide} value={plages} onChange={(e) => setPlages(e.currentTarget.value)} error={selection && "erreur" in selection ? selection.erreur : undefined} />
              )}
              {choix === "sauf-premiere" && selection && "erreur" in selection && (
                <Text size="sm" c="red">
                  {selection.erreur}
                </Text>
              )}
            </Stack>
          </Paper>

          <Action icone={IconStack2} enCours={enCours} raison={raison} onClick={() => void creer()}>
            {t.creer}
          </Action>
          <Resultat cree={cree} onFermer={() => setCree(null)} />
        </Stack>

        {source && (
          <Paper withBorder p="md" data-apercu-marques>
            <Stack gap="xs" align="center">
              <Group justify="space-between" w="100%">
                <Title order={5}>{t.apercu}</Title>
                <Group gap={4}>
                  <Tooltip label={t.precedente}>
                    <ActionIcon variant="default" aria-label={t.precedente} disabled={page === 0} onClick={() => setPage((p) => p - 1)}>
                      <IconChevronLeft size={16} />
                    </ActionIcon>
                  </Tooltip>
                  <Text size="sm" w={110} ta="center">
                    {t.page(page + 1, source.pages)}
                  </Text>
                  <Tooltip label={t.suivante}>
                    <ActionIcon variant="default" aria-label={t.suivante} disabled={page >= source.pages - 1} onClick={() => setPage((p) => p + 1)}>
                      <IconChevronRight size={16} />
                    </ActionIcon>
                  </Tooltip>
                </Group>
              </Group>
              <Box w={LARGEUR_APERCU} mih={LARGEUR_APERCU} style={{ display: "flex", alignItems: "center", justifyContent: "center" }}>
                {apercu && options ? (
                  <img src={apercu} alt={t.page(page + 1, source.pages)} width={LARGEUR_APERCU} style={{ boxShadow: "0 1px 4px rgba(34, 37, 42, 0.25)" }} />
                ) : options ? (
                  <Loader size="sm" color="gray" />
                ) : (
                  <Text size="sm" c="dimmed">
                    {t.rien}
                  </Text>
                )}
              </Box>
            </Stack>
          </Paper>
        )}
      </Group>
    </Stack>
  );
}
