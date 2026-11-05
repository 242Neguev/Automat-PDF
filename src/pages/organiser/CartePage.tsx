import type { MouseEvent, PointerEvent } from "react";
import { ActionIcon, Badge, Group, Paper, Text, Tooltip } from "@mantine/core";
import { IconMaximize, IconRotate, IconRotateClockwise, IconTrash } from "@tabler/icons-react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { textes } from "../../textes";
import Vignette from "../commun/Vignette";

const t = textes.organisation;

interface Props {
  /** Élément déplacé : une page de PDF, ou une image. */
  element: { id: string; rotation: number };
  /** Position dans le document (à partir de 1). */
  numero: number;
  chemin: string;
  /** Page du PDF (à partir de 0) ; absente pour un fichier image. */
  page?: number;
  /** Texte court du badge (fichier d'origine). */
  legende: string;
  /** Description complète (bulle, lecteurs d'écran). */
  description: string;
  couleur: string;
  cote: number;
  largeur: number;
  choisie: boolean;
  onClic: (evenement: MouseEvent) => void;
  /** Aperçu en grand, s'il y en a un. */
  onAgrandir?: () => void;
  onPivoter: (sens: 1 | -1) => void;
  onSupprimer: () => void;
}

/** Une page (ou une image) à glisser pour la déplacer, avec ses boutons au survol. */
export default function CartePage({ element, numero, chemin, page, legende, description, couleur, cote, largeur, choisie, onClic, onAgrandir, onPivoter, onSupprimer }: Props) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: element.id });
  // Un bouton ne doit ni changer la sélection ni commencer un déplacement.
  const bouton = (action: () => void) => ({
    onClick: (evenement: MouseEvent) => {
      evenement.stopPropagation();
      action();
    },
    onPointerDown: (evenement: PointerEvent) => evenement.stopPropagation(),
  });
  return (
    <Paper
      ref={setNodeRef}
      className="carte-page"
      withBorder
      p={6}
      radius="md"
      data-page-organiser={numero}
      data-choisie={choisie || undefined}
      aria-label={`${numero}. ${description}`}
      onClick={onClic}
      onDoubleClick={onAgrandir}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
        opacity: isDragging ? 0.3 : 1,
        position: "relative",
        cursor: "grab",
        outline: choisie ? "2px solid var(--mantine-color-eau-6)" : undefined,
        background: choisie ? "var(--mantine-color-eau-0)" : undefined,
        touchAction: "none",
      }}
      {...attributes}
      {...listeners}
    >
      <Vignette chemin={chemin} page={page} rotation={element.rotation} taille={cote} largeur={largeur} alt={description} />
      <Group justify="space-between" gap={4} mt={6} wrap="nowrap">
        <Text size="xs" fw={600}>
          {numero}
        </Text>
        <Tooltip label={description} withArrow>
          <Badge size="xs" variant="light" color={couleur} maw={cote - 24} style={{ textTransform: "none" }}>
            {legende}
          </Badge>
        </Tooltip>
      </Group>
      <Group className="actions-page" gap={2} pos="absolute" top={4} right={4}>
        <Tooltip label={t.pivoterGauche} withArrow>
          <ActionIcon size="sm" variant="default" aria-label={t.pivoterGauche} {...bouton(() => onPivoter(-1))}>
            <IconRotate size={14} />
          </ActionIcon>
        </Tooltip>
        <Tooltip label={t.pivoterDroite} withArrow>
          <ActionIcon size="sm" variant="default" aria-label={t.pivoterDroite} {...bouton(() => onPivoter(1))}>
            <IconRotateClockwise size={14} />
          </ActionIcon>
        </Tooltip>
        {onAgrandir && (
          <Tooltip label={t.agrandir} withArrow>
            <ActionIcon size="sm" variant="default" aria-label={t.agrandir} {...bouton(onAgrandir)}>
              <IconMaximize size={14} />
            </ActionIcon>
          </Tooltip>
        )}
        <Tooltip label={t.supprimer} withArrow>
          <ActionIcon size="sm" variant="default" color="red" aria-label={t.supprimer} {...bouton(onSupprimer)}>
            <IconTrash size={14} />
          </ActionIcon>
        </Tooltip>
      </Group>
    </Paper>
  );
}
