import { useEffect } from "react";
import { ActionIcon, Group, Modal, Stack, Text, Tooltip } from "@mantine/core";
import { IconChevronLeft, IconChevronRight } from "@tabler/icons-react";
import { textes } from "../../textes";
import Vignette from "../commun/Vignette";
import type { PageTravail } from "./pages";

const t = textes.organisation;
const COTE = 720;

interface Props {
  pages: PageTravail[];
  /** Page affichée (position dans le document), ou null : aperçu fermé. */
  index: number | null;
  source: (page: PageTravail) => { chemin: string; nom: string };
  onIndex: (index: number | null) => void;
}

/** Une page en grand, avec les flèches (boutons ou clavier) pour passer aux voisines. */
export default function ApercuPage({ pages, index, source, onIndex }: Props) {
  const page = index === null ? null : pages[index];
  const aller = (sens: number) => index !== null && onIndex(Math.min(pages.length - 1, Math.max(0, index + sens)));

  useEffect(() => {
    if (index === null) return;
    const touche = (evenement: KeyboardEvent) => {
      if (evenement.key === "ArrowLeft") aller(-1);
      if (evenement.key === "ArrowRight") aller(1);
    };
    window.addEventListener("keydown", touche);
    return () => window.removeEventListener("keydown", touche);
  });

  return (
    <Modal opened={page !== null && page !== undefined} onClose={() => onIndex(null)} size="auto" centered title={index !== null ? t.position(index + 1, pages.length) : ""}>
      {page && (
        <Stack align="center" gap="sm">
          <Group gap="md" wrap="nowrap">
            <Tooltip label={t.precedente}>
              <ActionIcon variant="light" size="lg" aria-label={t.precedente} disabled={index === 0} onClick={() => aller(-1)}>
                <IconChevronLeft size={20} />
              </ActionIcon>
            </Tooltip>
            <Vignette key={page.id} chemin={source(page).chemin} page={page.page} rotation={page.rotation} taille={COTE} largeur={Math.round(COTE * Math.min(2, window.devicePixelRatio || 1))} alt={t.origine(source(page).nom, page.page + 1)} />
            <Tooltip label={t.suivante}>
              <ActionIcon variant="light" size="lg" aria-label={t.suivante} disabled={index === pages.length - 1} onClick={() => aller(1)}>
                <IconChevronRight size={20} />
              </ActionIcon>
            </Tooltip>
          </Group>
          <Text size="sm" c="dimmed">
            {t.origine(source(page).nom, page.page + 1)}
          </Text>
        </Stack>
      )}
    </Modal>
  );
}
