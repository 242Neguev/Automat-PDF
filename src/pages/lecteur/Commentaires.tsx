// Liste des commentaires du document ouvert, à la place des outils pendant l'annotation (comme le
// volet « Commentaires » d'Acrobat) : chaque annotation, page après page dans l'ordre de lecture, avec
// ce qu'elle dit. Un clic mène à elle et la choisit sur sa page ; la corbeille la retire.
import { Fragment, useEffect, useMemo, useRef } from "react";
import { ActionIcon, Stack, Text, Tooltip, UnstyledButton } from "@mantine/core";
import { IconTrash } from "@tabler/icons-react";
import { ordreDeLecture, TAMPONS, type Annotation } from "../../donnees/annotations";
import { textes } from "../../textes";
import { couleur, iconeDe } from "./Annotations";

const t = textes.lecteur.commentaires;
const ta = textes.lecteur.annotation;

/** Ce que la liste dit d'une annotation : sa sorte, et son texte s'il en a un. */
function resume(annotation: Annotation): { titre: string; detail: string | null } {
  switch (annotation.sorte) {
    case "surlignage":
    case "soulignage":
    case "barre":
      return { titre: t.sortes[annotation.sorte], detail: annotation.extrait ? `« ${annotation.extrait} »` : null };
    case "note":
    case "texte":
      return { titre: t.sortes[annotation.sorte], detail: annotation.texte.trim() || null };
    case "correction":
      return { titre: t.sortes.correction, detail: `« ${annotation.ancien} » → « ${annotation.texte} »` };
    case "forme":
      return { titre: ta.formes[annotation.forme], detail: null };
    case "tampon":
      return { titre: `${t.sortes.tampon} ${TAMPONS[annotation.mot].texte}`, detail: annotation.date };
    default:
      return { titre: t.sortes[annotation.sorte], detail: null };
  }
}

/** La couleur de l'icône : celle de l'annotation sur la page. */
function teinteDe(annotation: Annotation): string | undefined {
  switch (annotation.sorte) {
    case "surlignage":
    case "soulignage":
    case "barre":
    case "texte":
    case "forme":
    case "dessin":
      return couleur(annotation.teinte);
    case "tampon":
      return `rgb(${TAMPONS[annotation.mot].rvb.join(", ")})`;
    case "note":
      return "#b8860b";
    default:
      return undefined;
  }
}

interface Props {
  annotations: Annotation[];
  selection: string | null;
  onChoisir: (annotation: Annotation) => void;
  onSupprimer: (id: string) => void;
}

export default function ListeCommentaires({ annotations, selection, onChoisir, onSupprimer }: Props) {
  const triees = useMemo(() => ordreDeLecture(annotations), [annotations]);
  const elements = useRef<Record<string, HTMLDivElement | null>>({});
  // Le commentaire choisi sur la page reste visible dans la liste.
  useEffect(() => {
    if (selection) elements.current[selection]?.scrollIntoView({ block: "nearest" });
  }, [selection]);

  return (
    <Stack gap={6} w={272} style={{ overflow: "auto", flexShrink: 0 }} data-commentaires>
      <div>
        <Text fw={750} size="lg">
          {t.titre}
        </Text>
        <Text size="xs" c="dimmed">
          {triees.length === 0 ? t.vide : t.aide}
        </Text>
      </div>
      {triees.map((annotation, rang) => {
        const Icone = iconeDe(annotation);
        const { titre, detail } = resume(annotation);
        const nouvellePage = rang === 0 || triees[rang - 1].page !== annotation.page;
        return (
          <Fragment key={annotation.id}>
            {nouvellePage && (
              <Text size="xs" c="dimmed" className="liste-commentaires__page">
                {t.page(annotation.page + 1)}
              </Text>
            )}
            <div
              ref={(element) => {
                elements.current[annotation.id] = element;
              }}
              className="commentaire"
              data-choisi={selection === annotation.id || undefined}
              data-commentaire={annotation.sorte}
            >
              <UnstyledButton className="commentaire__corps" onClick={() => onChoisir(annotation)} aria-current={selection === annotation.id || undefined}>
                <Icone size={17} stroke={1.8} className="commentaire__icone" style={{ color: teinteDe(annotation) }} />
                <div style={{ minWidth: 0 }}>
                  <Text size="sm" fw={600} lh={1.3}>
                    {titre}
                  </Text>
                  {detail && (
                    <Text size="xs" c="dimmed" lineClamp={3} style={{ overflowWrap: "anywhere", whiteSpace: "pre-line" }}>
                      {detail}
                    </Text>
                  )}
                </div>
              </UnstyledButton>
              <Tooltip label={t.supprimer} withArrow openDelay={300}>
                <ActionIcon variant="subtle" color="gray" size={28} mt={4} onClick={() => onSupprimer(annotation.id)} aria-label={t.supprimer}>
                  <IconTrash size={15} stroke={1.7} />
                </ActionIcon>
              </Tooltip>
            </div>
          </Fragment>
        );
      })}
    </Stack>
  );
}
