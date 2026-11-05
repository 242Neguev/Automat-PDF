import { useEffect, useRef, useState, type PointerEvent } from "react";
import { ActionIcon, Badge, Box, Button, Group, Loader, Paper, SegmentedControl, Stack, Text, Title, Tooltip } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconAlertTriangle, IconCheck, IconChevronLeft, IconChevronRight, IconEraser, IconEyeOff, IconFileSearch, IconX } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { caviarderPdf, pagesPdf, type Zone } from "../../coeur";
import { textes } from "../../textes";
import { useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";
import { imagePage } from "../commun/vignettes";

const t = textes.caviardage;
const LARGEUR = 560;
const borner = (valeur: number) => Math.min(1, Math.max(0, valeur));

/** La page, ses zones noires, et le rectangle en cours de tracé. */
function PageZones({ chemin, page, zones, onZones }: { chemin: string; page: number; zones: Zone[]; onZones: (zones: Zone[]) => void }) {
  const cadre = useRef<HTMLDivElement>(null);
  const depart = useRef<{ x: number; y: number } | null>(null);
  const [fond, setFond] = useState<string | null>(null);
  const [hauteur, setHauteur] = useState<number | null>(null);
  const [trace, setTrace] = useState<Zone | null>(null);

  useEffect(() => {
    let actuel = true;
    setFond(null);
    imagePage(chemin, page, LARGEUR * 2).then((adresse) => actuel && setFond(adresse), signalerErreur);
    return () => {
      actuel = false;
    };
  }, [chemin, page]);

  const position = (evenement: PointerEvent) => {
    const rect = cadre.current!.getBoundingClientRect();
    return { x: borner((evenement.clientX - rect.left) / rect.width), y: borner((evenement.clientY - rect.top) / rect.height) };
  };

  function debut(evenement: PointerEvent) {
    if (!hauteur) return;
    evenement.currentTarget.setPointerCapture(evenement.pointerId);
    depart.current = position(evenement);
    setTrace({ ...depart.current, largeur: 0, hauteur: 0 });
  }

  function mouvement(evenement: PointerEvent) {
    if (!depart.current) return;
    const ici = position(evenement);
    setTrace({
      x: Math.min(depart.current.x, ici.x),
      y: Math.min(depart.current.y, ici.y),
      largeur: Math.abs(ici.x - depart.current.x),
      hauteur: Math.abs(ici.y - depart.current.y),
    });
  }

  function fin() {
    // Un simple clic (rectangle minuscule) n'ajoute rien.
    if (trace && trace.largeur > 0.005 && trace.hauteur > 0.003) onZones([...zones, trace]);
    depart.current = null;
    setTrace(null);
  }

  const style = (zone: Zone) => ({ left: zone.x * LARGEUR, top: zone.y * (hauteur ?? 0), width: zone.largeur * LARGEUR, height: zone.hauteur * (hauteur ?? 0) });

  return (
    <Box
      ref={cadre}
      pos="relative"
      w={LARGEUR}
      h={hauteur ?? LARGEUR * 1.414}
      onPointerDown={debut}
      onPointerMove={mouvement}
      onPointerUp={fin}
      data-page-caviardage
      style={{ boxShadow: "0 1px 4px rgba(34, 37, 42, 0.25)", background: "white", cursor: "crosshair", touchAction: "none", userSelect: "none", flexShrink: 0 }}
    >
      {fond ? (
        <img src={fond} width={LARGEUR} alt="" draggable={false} onLoad={(e) => setHauteur((LARGEUR * e.currentTarget.naturalHeight) / e.currentTarget.naturalWidth)} style={{ display: "block" }} />
      ) : (
        <Loader size="sm" color="gray" pos="absolute" top="45%" left="47%" />
      )}
      {hauteur &&
        zones.map((zone, i) => (
          <Box key={i} data-zone pos="absolute" style={{ ...style(zone), background: "rgba(0, 0, 0, 0.82)" }}>
            <Tooltip label={t.retirer} withArrow>
              <ActionIcon
                size="xs"
                color="red"
                variant="filled"
                aria-label={t.retirer}
                pos="absolute"
                top={-8}
                right={-8}
                onPointerDown={(e) => e.stopPropagation()}
                onClick={() => onZones(zones.filter((_, autre) => autre !== i))}
              >
                <IconX size={12} />
              </ActionIcon>
            </Tooltip>
          </Box>
        ))}
      {trace && <Box pos="absolute" style={{ ...style(trace), outline: "1.5px dashed var(--mantine-color-red-6)", background: "rgba(0, 0, 0, 0.35)" }} />}
    </Box>
  );
}

/** Caviarder : des zones noircies, et ce qui est dessous vraiment supprimé du fichier. */
export default function Caviarder() {
  const [source, setSource] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [page, setPage] = useState(0);
  const [zones, setZones] = useState<Map<number, Zone[]>>(new Map());
  const [mode, setMode] = useState<"texte" | "image">("texte");
  const [enCours, setEnCours] = useState(false);
  const total = [...zones.values()].reduce((somme, liste) => somme + liste.length, 0);
  const zonesPage = zones.get(page) ?? [];

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      setSource({ chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin, pages: (await pagesPdf(chemin)).length });
      setPage(0);
      setZones(new Map());
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("caviarder", (fichiers) => void ouvrir(fichiers[0]));

  const changerZones = (liste: Zone[]) =>
    setZones((actuelles) => {
      const nouvelles = new Map(actuelles);
      if (liste.length > 0) nouvelles.set(page, liste);
      else nouvelles.delete(page);
      return nouvelles;
    });

  async function creer() {
    if (!source || total === 0) return;
    const sortie = await save({ defaultPath: t.nomFichier(source.nom.replace(/\.pdf$/i, "")), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    try {
      const pages = [...zones.entries()].map(([numero, liste]) => ({ page: numero, zones: liste }));
      const { pagesEnImage } = await caviarderPdf(source.chemin, pages, mode, sortie);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.cree, message: sortie });
      if (mode === "texte" && pagesEnImage.length > 0) {
        notifications.show({ color: "orange", icon: <IconAlertTriangle size={18} />, title: t.cree, message: t.pagesEnImage(pagesEnImage), autoClose: 12000 });
      }
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md">
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
        <Group align="flex-start" gap="lg" wrap="nowrap">
          <Stack gap="md" maw={400} style={{ flex: 1 }}>
            <Paper withBorder p="md">
              <Stack gap="xs">
                <Text size="sm">{t.aide}</Text>
                <Badge variant="light" size="lg" style={{ textTransform: "none" }}>
                  {t.zones(zonesPage.length, total)}
                </Badge>
                <Button variant="subtle" color="gray" size="xs" leftSection={<IconEraser size={16} />} disabled={zonesPage.length === 0} onClick={() => changerZones([])} w="fit-content">
                  {t.viderPage}
                </Button>
              </Stack>
            </Paper>
            <Paper withBorder p="md">
              <Stack gap="xs">
                <Title order={5}>{t.mode}</Title>
                <SegmentedControl
                  orientation="vertical"
                  fullWidth
                  value={mode}
                  onChange={(v) => setMode(v as typeof mode)}
                  data={(["texte", "image"] as const).map((value) => ({ value, label: t.modes[value] }))}
                />
                <Text size="xs" c="dimmed">
                  {t.modesAide[mode]}
                </Text>
                <Text size="xs" c="dimmed">
                  {t.garanties}
                </Text>
              </Stack>
            </Paper>
            <Button color="dark" leftSection={<IconEyeOff size={18} />} loading={enCours} disabled={total === 0} onClick={creer} w="fit-content">
              {t.creer}
            </Button>
          </Stack>

          <Stack gap="xs" align="center">
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
            <PageZones chemin={source.chemin} page={page} zones={zonesPage} onZones={changerZones} />
          </Stack>
        </Group>
      )}
    </Stack>
  );
}
