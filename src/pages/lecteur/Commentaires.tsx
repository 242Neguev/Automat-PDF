// Liste des commentaires du document ouvert, à la place des outils pendant l'annotation (comme le
// volet « Commentaires » d'Acrobat) : ceux déjà présents dans le PDF reçu (auteur, date, texte) et
// ceux ajoutés ici, page après page dans l'ordre de lecture. Un clic mène au commentaire ; on peut
// changer le texte d'un commentaire du PDF, ou le supprimer (le PDF annoté enregistré le gardera).
import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import { ActionIcon, Button, Group, Stack, Text, Textarea, Tooltip, UnstyledButton } from "@mantine/core";
import {
  IconCircle,
  IconCursorText,
  IconHighlight,
  IconLine,
  IconMessageCircle,
  IconPaperclip,
  IconPencil,
  IconRectangle,
  IconRubberStamp,
  IconScribble,
  IconShape,
  IconStrikethrough,
  IconTrash,
  IconTypography,
  IconUnderline,
} from "@tabler/icons-react";
import type { CommentairePdf } from "../../coeur";
import { estRetouche, TAMPONS, zoneDe, type Annotation, type Zone } from "../../donnees/annotations";
import { textes } from "../../textes";
import { couleur, iconeDe } from "./Annotations";

const t = textes.lecteur.commentaires;
const ta = textes.lecteur.annotation;

/** L'icône d'un commentaire du PDF, d'après sa sorte. */
const ICONES_PDF: Record<string, typeof IconMessageCircle> = {
  Text: IconMessageCircle,
  Highlight: IconHighlight,
  Underline: IconUnderline,
  Squiggly: IconUnderline,
  StrikeOut: IconStrikethrough,
  FreeText: IconTypography,
  Square: IconRectangle,
  Circle: IconCircle,
  Line: IconLine,
  Polygon: IconShape,
  PolyLine: IconShape,
  Ink: IconScribble,
  Stamp: IconRubberStamp,
  Caret: IconCursorText,
  FileAttachment: IconPaperclip,
};

/** Ce que la liste dit d'une annotation ajoutée ici : sa sorte, et son texte s'il en a un. */
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
    case "image":
    case "blanc":
    case "dessin":
      return { titre: t.sortes[annotation.sorte], detail: null };
    case "suppression":
    case "retouche":
      return { titre: "", detail: null };
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

type Entree =
  | { genre: "ici"; cle: string; page: number; zone: Zone; annotation: Annotation }
  | { genre: "pdf"; cle: string; page: number; zone: Zone; commentaire: CommentairePdf; texte: string };

/** Dans l'ordre de lecture : page après page, de haut en bas, puis de gauche à droite. */
function avant(a: Entree, b: Entree): number {
  return a.page - b.page || (Math.abs(a.zone.y - b.zone.y) > 4 ? a.zone.y - b.zone.y : a.zone.x - b.zone.x);
}

interface Props {
  /** Les annotations ajoutées ici (retouches des commentaires du PDF comprises, non listées). */
  annotations: Annotation[];
  /** Les commentaires déjà présents dans le PDF, d'après son contenu d'origine. */
  existants: CommentairePdf[];
  /** Retouches en cours des commentaires du PDF : supprimés, et nouveau texte. */
  supprimes: string[];
  textesChanges: [string, string][];
  selection: string | null;
  /** Le commentaire du PDF choisi dans la liste. */
  choisiPdf: string | null;
  onChoisir: (annotation: Annotation) => void;
  onSupprimer: (id: string) => void;
  onAllerPdf: (commentaire: CommentairePdf) => void;
  onSupprimerPdf: (commentaire: CommentairePdf) => void;
  onModifierPdf: (commentaire: CommentairePdf, texte: string) => void;
}

export default function ListeCommentaires(props: Props) {
  const { annotations, existants, supprimes, textesChanges, selection, choisiPdf, onChoisir, onSupprimer } = props;
  /** Commentaire du PDF dont on change le texte. */
  const [edition, setEdition] = useState<{ id: string; texte: string } | null>(null);
  const entrees = useMemo(() => {
    const retires = new Set(supprimes);
    const changes = new Map(textesChanges);
    const liste: Entree[] = [
      ...existants
        .filter((commentaire) => !retires.has(commentaire.id))
        .map(
          (commentaire): Entree => ({
            genre: "pdf",
            cle: `pdf:${commentaire.id}`,
            page: commentaire.page,
            zone: commentaire,
            commentaire,
            texte: changes.get(commentaire.id) ?? commentaire.texte,
          }),
        ),
      ...annotations
        .filter((annotation) => !estRetouche(annotation))
        .map((annotation): Entree => ({ genre: "ici", cle: annotation.id, page: annotation.page, zone: zoneDe(annotation), annotation })),
    ];
    return liste.sort(avant);
  }, [annotations, existants, supprimes, textesChanges]);
  const elements = useRef<Record<string, HTMLDivElement | null>>({});
  // Le commentaire choisi reste visible dans la liste.
  useEffect(() => {
    const cle = selection ?? (choisiPdf ? `pdf:${choisiPdf}` : null);
    if (cle) elements.current[cle]?.scrollIntoView({ block: "nearest" });
  }, [selection, choisiPdf]);
  const nombrePdf = entrees.filter((entree) => entree.genre === "pdf").length;

  /** Garde le nouveau texte d'un commentaire du PDF, s'il a changé. */
  function valider(commentaire: CommentairePdf, actuel: string) {
    if (edition && edition.texte.trim() !== actuel.trim()) props.onModifierPdf(commentaire, edition.texte.trim());
    setEdition(null);
  }

  return (
    <Stack gap={6} w={272} style={{ overflow: "auto", flexShrink: 0 }} data-commentaires>
      <div>
        <Text fw={750} size="lg">
          {t.titre}
        </Text>
        <Text size="xs" c="dimmed">
          {entrees.length === 0 ? t.vide : t.aide}
        </Text>
        {nombrePdf > 0 && (
          <Text size="xs" c="dimmed" mt={2}>
            {t.dejaDansLePdf(nombrePdf)}
          </Text>
        )}
      </div>
      {entrees.map((entree, rang) => {
        const nouvellePage = rang === 0 || entrees[rang - 1].page !== entree.page;
        const enTete = nouvellePage && (
          <Text size="xs" c="dimmed" className="liste-commentaires__page">
            {t.page(entree.page + 1)}
          </Text>
        );
        const garder = (element: HTMLDivElement | null) => {
          elements.current[entree.cle] = element;
        };
        if (entree.genre === "ici") {
          const { annotation } = entree;
          const Icone = iconeDe(annotation);
          const { titre, detail } = resume(annotation);
          return (
            <Fragment key={entree.cle}>
              {enTete}
              <div ref={garder} className="commentaire" data-choisi={selection === annotation.id || undefined} data-commentaire={annotation.sorte}>
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
        }
        const { commentaire, texte } = entree;
        const Icone = ICONES_PDF[commentaire.sorte] ?? IconMessageCircle;
        const titre = t.sortesPdf[commentaire.sorte] ?? t.autre;
        const auteurEtDate = [commentaire.auteur, commentaire.date].filter(Boolean).join(" · ");
        const enEdition = edition?.id === commentaire.id;
        return (
          <Fragment key={entree.cle}>
            {enTete}
            <div ref={garder} className="commentaire" data-choisi={choisiPdf === commentaire.id || undefined} data-commentaire-pdf={commentaire.sorte}>
              <UnstyledButton className="commentaire__corps" onClick={() => props.onAllerPdf(commentaire)} aria-current={choisiPdf === commentaire.id || undefined}>
                <Icone size={17} stroke={1.8} className="commentaire__icone" style={{ color: commentaire.couleur ? `rgb(${commentaire.couleur.join(", ")})` : undefined }} />
                <div style={{ minWidth: 0 }}>
                  <Text size="sm" fw={600} lh={1.3}>
                    {commentaire.reponse ? `${titre} · ${t.reponse}` : titre}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {auteurEtDate || t.dansLePdf}
                  </Text>
                  {texte && !enEdition && (
                    <Text size="xs" lineClamp={4} style={{ overflowWrap: "anywhere", whiteSpace: "pre-line" }} data-texte-commentaire>
                      {texte}
                    </Text>
                  )}
                </div>
              </UnstyledButton>
              <Stack gap={0} mt={4}>
                {commentaire.modifiable && (
                  <Tooltip label={t.modifier} withArrow openDelay={300}>
                    <ActionIcon variant="subtle" color="gray" size={28} onClick={() => setEdition({ id: commentaire.id, texte })} aria-label={t.modifier} data-modifier-commentaire>
                      <IconPencil size={15} stroke={1.7} />
                    </ActionIcon>
                  </Tooltip>
                )}
                <Tooltip label={t.supprimerPdf} withArrow openDelay={300}>
                  <ActionIcon variant="subtle" color="gray" size={28} onClick={() => props.onSupprimerPdf(commentaire)} aria-label={t.supprimerPdf} data-supprimer-commentaire>
                    <IconTrash size={15} stroke={1.7} />
                  </ActionIcon>
                </Tooltip>
              </Stack>
            </div>
            {enEdition && (
              <div className="commentaire__edition">
                <Textarea
                  autoFocus
                  autosize
                  minRows={2}
                  maxRows={8}
                  size="xs"
                  value={edition.texte}
                  onChange={(evenement) => setEdition({ id: commentaire.id, texte: evenement.currentTarget.value })}
                  onKeyDown={(evenement) => {
                    if (evenement.key === "Escape") setEdition(null);
                    else if (evenement.key === "Enter" && (evenement.metaKey || evenement.ctrlKey)) valider(commentaire, texte);
                  }}
                  aria-label={t.modifier}
                  data-edition-commentaire
                />
                <Group justify="flex-end" gap={6} mt={6}>
                  <Button size="compact-xs" variant="default" onClick={() => setEdition(null)}>
                    {t.annulerEdition}
                  </Button>
                  <Button size="compact-xs" color="eau" onClick={() => valider(commentaire, texte)}>
                    {t.validerEdition}
                  </Button>
                </Group>
              </div>
            )}
          </Fragment>
        );
      })}
    </Stack>
  );
}
