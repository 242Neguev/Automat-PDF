// Choisir des pages d'un PDF en les voyant : les vignettes à cocher, la page en grand à côté (pour
// être sûr que c'est la bonne), et la liste des numéros, que l'on peut aussi taper (« 2, 5-7 »).
import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import { ActionIcon, Badge, Box, Button, Group, Modal, Paper, SegmentedControl, Stack, Text, TextInput, Tooltip, UnstyledButton } from "@mantine/core";
import { IconCheck, IconChevronLeft, IconChevronRight, IconCircleCheckFilled, IconX } from "@tabler/icons-react";
import { ecrirePlages, lirePlages } from "../../donnees/plages";
import { textes } from "../../textes";
import { useTailleFenetre } from "./useTailleFenetre";
import Vignette from "./Vignette";

const t = textes.choixPages;
const TAILLES = { petite: 96, moyenne: 136, grande: 196 } as const;
type Taille = keyof typeof TAILLES;

interface Props {
  chemin: string;
  /** Nombre de pages du PDF. */
  total: number;
  /** Pages choisies, à partir de 0, dans l'ordre du document. */
  choisies: number[];
  onChoisies: (pages: number[]) => void;
  /** Page montrée en grand (à partir de 0). */
  vue: number;
  onVue: (page: number) => void;
}

/** Côté du carré de la page en grand, à côté des vignettes : assez pour la lire, sans dépasser la fenêtre. */
export function coteEnGrand(largeur: number, hauteur: number): number {
  return Math.round(Math.max(220, Math.min(560, hauteur - 250, largeur * 0.3)));
}

/** Côté du carré de la page agrandie (double-clic) : la fenêtre d'aperçu tient entière. */
export function cotePleinEcran(largeur: number, hauteur: number): number {
  return Math.round(Math.max(240, Math.min(1000, hauteur * 0.9 - 150, largeur * 0.9 - 140)));
}

/** Les vignettes à cocher et la page en grand ; les pages choisies restent dans l'ordre du document. */
export default function ChoixPages({ chemin, total, choisies, onChoisies, vue, onVue }: Props) {
  const [taille, setTaille] = useState<Taille>("moyenne");
  const [saisie, setSaisie] = useState(() => ecrirePlages(choisies));
  const [erreur, setErreur] = useState<string | null>(null);
  const [ancre, setAncre] = useState<number | null>(null);
  const [agrandie, setAgrandie] = useState(false);
  const fenetre = useTailleFenetre();
  const vignettes = useRef<(HTMLButtonElement | null)[]>([]);
  const ensemble = new Set(choisies);
  const choisieVue = ensemble.has(vue);
  const cote = TAILLES[taille];
  const finesse = Math.min(2, window.devicePixelRatio || 1);
  const enGrand = coteEnGrand(fenetre.largeur, fenetre.hauteur);
  const pleinEcran = cotePleinEcran(fenetre.largeur, fenetre.hauteur);

  // La liste tapée suit les clics ; pendant la frappe, elle reste telle qu'on l'écrit.
  const depuisSaisie = useRef(false);
  useEffect(() => {
    if (depuisSaisie.current) {
      depuisSaisie.current = false;
      return;
    }
    setSaisie(ecrirePlages(choisies));
    setErreur(null);
  }, [choisies]);

  // La vignette de la page montrée en grand reste à l'écran (flèches, clavier).
  useEffect(() => {
    vignettes.current[vue]?.scrollIntoView({ block: "nearest" });
  }, [vue]);

  function changer(pages: number[]) {
    onChoisies([...new Set(pages)].filter((page) => page >= 0 && page < total).sort((a, b) => a - b));
  }

  function basculer(page: number) {
    changer(ensemble.has(page) ? choisies.filter((choisie) => choisie !== page) : [...choisies, page]);
    setAncre(page);
  }

  function cliquer(page: number, evenement: MouseEvent) {
    if (evenement.shiftKey && ancre !== null) {
      const [de, a] = ancre < page ? [ancre, page] : [page, ancre];
      changer([...choisies, ...Array.from({ length: a - de + 1 }, (_, i) => de + i)]);
      setAncre(page);
    } else {
      basculer(page);
    }
    onVue(page);
  }

  function taper(valeur: string) {
    setSaisie(valeur);
    depuisSaisie.current = true;
    if (!valeur.trim()) {
      setErreur(null);
      changer([]);
      return;
    }
    const lecture = lirePlages(valeur, total);
    if ("erreur" in lecture) {
      depuisSaisie.current = false;
      setErreur(lecture.erreur);
      return;
    }
    setErreur(null);
    changer(lecture.pages);
    onVue(lecture.pages[lecture.pages.length - 1]);
  }

  function aller(sens: -1 | 1) {
    onVue(Math.min(total - 1, Math.max(0, vue + sens)));
  }

  // Flèches gauche et droite : la page voisine en grand (pas pendant la frappe d'une liste).
  function clavier(evenement: KeyboardEvent) {
    if (evenement.target instanceof HTMLInputElement || evenement.target instanceof HTMLTextAreaElement) return;
    if (evenement.key === "ArrowLeft") aller(-1);
    else if (evenement.key === "ArrowRight") aller(1);
    else return;
    evenement.preventDefault();
  }

  const fleches = (
    <>
      <Tooltip label={t.precedente} withArrow>
        <ActionIcon variant="light" size="lg" aria-label={t.precedente} disabled={vue === 0} onClick={() => aller(-1)}>
          <IconChevronLeft size={20} />
        </ActionIcon>
      </Tooltip>
      <Button
        leftSection={choisieVue ? <IconX size={16} /> : <IconCheck size={16} />}
        variant={choisieVue ? "default" : "filled"}
        onClick={() => basculer(vue)}
        data-basculer-page
      >
        {choisieVue ? t.retirer : t.choisir}
      </Button>
      <Tooltip label={t.suivante} withArrow>
        <ActionIcon variant="light" size="lg" aria-label={t.suivante} disabled={vue >= total - 1} onClick={() => aller(1)}>
          <IconChevronRight size={20} />
        </ActionIcon>
      </Tooltip>
    </>
  );

  return (
    <Stack gap="sm" onKeyDown={clavier} data-choix-pages>
      <Group justify="space-between" align="flex-end" gap="sm">
        <Group align="flex-end" gap="xs">
          <TextInput w={240} label={t.pages} placeholder={t.exemple} value={saisie} onChange={(e) => taper(e.currentTarget.value)} error={erreur} data-saisie-pages />
          <Button variant="default" onClick={() => changer(Array.from({ length: total }, (_, i) => i))}>
            {t.toutes}
          </Button>
          <Button variant="subtle" color="gray" disabled={choisies.length === 0} onClick={() => changer([])}>
            {t.aucune}
          </Button>
        </Group>
        <SegmentedControl
          size="xs"
          aria-label={t.taille}
          value={taille}
          onChange={(valeur) => setTaille(valeur as Taille)}
          data={(Object.keys(TAILLES) as Taille[]).map((value) => ({ value, label: t.tailles[value] }))}
        />
      </Group>
      <Text size="sm" c="dimmed">
        {t.aide}
      </Text>

      <Group align="flex-start" gap="md" wrap="nowrap">
        <Box style={{ flex: 1, minWidth: 0, display: "grid", gridTemplateColumns: `repeat(auto-fill, ${cote + 16}px)`, gap: 12 }}>
          {Array.from({ length: total }, (_, page) => {
            const choisie = ensemble.has(page);
            return (
              <UnstyledButton
                key={page}
                ref={(element: HTMLButtonElement | null) => {
                  vignettes.current[page] = element;
                }}
                className="vignette-choix"
                data-page-choix={page + 1}
                data-choisie={choisie || undefined}
                data-vue={page === vue || undefined}
                aria-pressed={choisie}
                aria-label={t.alt(page + 1)}
                onClick={(evenement: MouseEvent) => cliquer(page, evenement)}
                onDoubleClick={() => {
                  onVue(page);
                  setAgrandie(true);
                }}
              >
                <Vignette chemin={chemin} page={page} taille={cote} largeur={Math.round(cote * finesse)} alt="" />
                <Group justify="center" gap={4} wrap="nowrap">
                  <Text size="xs" fw={600}>
                    {page + 1}
                  </Text>
                  {choisie && <IconCircleCheckFilled className="coche" size={16} aria-hidden />}
                </Group>
              </UnstyledButton>
            );
          })}
        </Box>

        <Paper withBorder p="sm" radius="md" w={enGrand + 26} style={{ position: "sticky", top: 12, flexShrink: 0 }} data-page-en-grand={vue + 1}>
          <Stack gap="xs" align="center">
            <Group justify="space-between" w="100%" wrap="nowrap">
              <Text fw={650} size="sm">
                {t.position(vue + 1, total)}
              </Text>
              <Badge variant={choisieVue ? "filled" : "light"} color={choisieVue ? undefined : "gray"}>
                {choisieVue ? t.choisie : t.pasChoisie}
              </Badge>
            </Group>
            <Tooltip label={t.agrandir} withArrow openDelay={400}>
              <UnstyledButton onClick={() => setAgrandie(true)} aria-label={t.agrandir} style={{ cursor: "zoom-in" }}>
                <Vignette key={`${chemin}\u0000${vue}`} chemin={chemin} page={vue} taille={enGrand} largeur={Math.round(enGrand * finesse)} alt={t.alt(vue + 1)} />
              </UnstyledButton>
            </Tooltip>
            <Group justify="space-between" w="100%" wrap="nowrap">
              {fleches}
            </Group>
          </Stack>
        </Paper>
      </Group>

      <Modal opened={agrandie} onClose={() => setAgrandie(false)} size="auto" centered title={t.position(vue + 1, total)}>
        <Stack align="center" gap="sm">
          <Vignette key={`${chemin}\u0000${vue}\u0000grand`} chemin={chemin} page={vue} taille={pleinEcran} largeur={Math.round(pleinEcran * finesse)} alt={t.alt(vue + 1)} />
          <Group gap="md" wrap="nowrap">
            {fleches}
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}
