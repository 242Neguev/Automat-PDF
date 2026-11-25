// Éditeur de PDF, première étape : la barre d'annotation du lecteur, et sur chaque page la couche
// qui montre les annotations et reçoit les gestes de l'outil choisi. Positions en points de la page
// affichée (comme les mots), converties en pixels par l'échelle du lecteur.
import { useEffect, useRef, useState, type PointerEvent as EvenementPointeur, type ReactNode } from "react";
import { ActionIcon, Box, Button, ColorSwatch, Group, NumberInput, Popover, Text, Textarea, Tooltip } from "@mantine/core";
import { IconArrowBackUp, IconArrowForwardUp, IconCheck, IconDeviceFloppy, IconHighlight, IconMessageCircle, IconPhoto, IconPointer, IconTrash, IconTypography } from "@tabler/icons-react";
import {
  INTERLIGNE,
  nouvelId,
  RVB,
  TEINTES_SURLIGNEUR,
  TEINTES_TEXTE,
  zonesSurlignees,
  type Annotation,
  type MotPlace,
  type Point,
  type Teinte,
  type Zone,
} from "../../donnees/annotations";
import { textes } from "../../textes";

const t = textes.lecteur.annotation;

export type OutilAnnotation = "selection" | "surligner" | "note" | "texte" | "image";

const ICONES: Record<OutilAnnotation, typeof IconPointer> = {
  selection: IconPointer,
  surligner: IconHighlight,
  note: IconMessageCircle,
  texte: IconTypography,
  image: IconPhoto,
};
const ORDRE: OutilAnnotation[] = ["selection", "surligner", "note", "texte", "image"];

export const couleur = (teinte: Teinte, opacite = 1) => `rgba(${RVB[teinte].join(", ")}, ${opacite})`;

/** Image choisie pour être posée : son adresse dans la page, et sa largeur sur sa hauteur. */
export interface ImageChargee {
  adresse: string;
  proportion: number;
}

interface PropsBarre {
  outil: OutilAnnotation;
  onOutil: (outil: OutilAnnotation) => void;
  teinteSurligneur: Teinte;
  onTeinteSurligneur: (teinte: Teinte) => void;
  teinteTexte: Teinte;
  onTeinteTexte: (teinte: Teinte) => void;
  taille: number;
  onTaille: (taille: number) => void;
  nombre: number;
  peutAnnuler: boolean;
  peutRetablir: boolean;
  onAnnuler: () => void;
  onRetablir: () => void;
  selection: boolean;
  onSupprimer: () => void;
  enregistrement: boolean;
  onEnregistrer: () => void;
  onTerminer: () => void;
}

/** La barre d'annotation : les outils, la couleur et la taille, annuler, enregistrer le PDF annoté. */
export function BarreAnnotations(props: PropsBarre) {
  const { outil, onOutil, nombre } = props;
  const teintes = outil === "texte" ? TEINTES_TEXTE : outil === "surligner" ? TEINTES_SURLIGNEUR : [];
  const teinte = outil === "texte" ? props.teinteTexte : props.teinteSurligneur;
  const choisirTeinte = outil === "texte" ? props.onTeinteTexte : props.onTeinteSurligneur;
  return (
    <Box data-barre-annotations>
      <Group justify="space-between" wrap="wrap" gap="xs" className="commandes-lecteur barre-annotations">
        <Group gap={6} wrap="wrap">
          <Group gap={2} wrap="nowrap" className="choix-outil" role="radiogroup" aria-label={t.annoter}>
            {ORDRE.map((cle) => {
              const Icone = ICONES[cle];
              return (
                <Tooltip key={cle} label={t.outils[cle].titre}>
                  <ActionIcon
                    variant={outil === cle ? "filled" : "subtle"}
                    color={outil === cle ? "eau" : "gray"}
                    size={32}
                    radius="xl"
                    role="radio"
                    aria-checked={outil === cle}
                    aria-label={t.outils[cle].titre}
                    onClick={() => onOutil(cle)}
                    data-outil-annotation={cle}
                  >
                    <Icone size={18} stroke={1.8} />
                  </ActionIcon>
                </Tooltip>
              );
            })}
          </Group>
          {teintes.length > 0 && (
            <Group gap={6} wrap="nowrap" pl={6} className="choix-teinte">
              {teintes.map((cle) => (
                <Tooltip key={cle} label={t.teintes[cle]}>
                  <ColorSwatch
                    component="button"
                    type="button"
                    size={22}
                    color={couleur(cle)}
                    onClick={() => choisirTeinte(cle)}
                    aria-label={t.teintes[cle]}
                    aria-pressed={teinte === cle}
                    style={{ cursor: "pointer", color: cle === "jaune" || cle === "vert" ? "#22252a" : "white" }}
                  >
                    {teinte === cle && <IconCheck size={13} stroke={3} />}
                  </ColorSwatch>
                </Tooltip>
              ))}
            </Group>
          )}
          {outil === "texte" && (
            <NumberInput size="xs" w={78} min={6} max={72} value={props.taille} onChange={(valeur) => typeof valeur === "number" && props.onTaille(valeur)} aria-label={t.taille} title={t.taille} suffix=" pt" />
          )}
          <Group gap={2} wrap="nowrap" pl={4}>
            <Tooltip label={t.annuler}>
              <ActionIcon variant="subtle" color="gray" onClick={props.onAnnuler} disabled={!props.peutAnnuler} aria-label={t.annuler}>
                <IconArrowBackUp size={19} stroke={1.7} />
              </ActionIcon>
            </Tooltip>
            <Tooltip label={t.retablir}>
              <ActionIcon variant="subtle" color="gray" onClick={props.onRetablir} disabled={!props.peutRetablir} aria-label={t.retablir}>
                <IconArrowForwardUp size={19} stroke={1.7} />
              </ActionIcon>
            </Tooltip>
            {props.selection && (
              <Tooltip label={t.supprimer}>
                <ActionIcon variant="subtle" color="red" onClick={props.onSupprimer} aria-label={t.supprimer}>
                  <IconTrash size={18} stroke={1.7} />
                </ActionIcon>
              </Tooltip>
            )}
          </Group>
        </Group>
        <Group gap={8} wrap="nowrap">
          <Text size="xs" c="dimmed" style={{ whiteSpace: "nowrap" }} data-nombre-annotations>
            {t.nombre(nombre)}
          </Text>
          <Tooltip label={t.enregistrerAide} multiline w={260}>
            <Button size="xs" radius="xl" color="eau" leftSection={<IconDeviceFloppy size={16} />} onClick={props.onEnregistrer} disabled={nombre === 0} loading={props.enregistrement}>
              {t.enregistrer}
            </Button>
          </Tooltip>
          <Button size="xs" radius="xl" variant="default" onClick={props.onTerminer}>
            {t.terminer}
          </Button>
        </Group>
      </Group>
      <Text size="xs" c="dimmed" mt={4} px={10} data-aide-annotation>
        {t.outils[outil].aide}
      </Text>
    </Box>
  );
}

interface PropsCouche {
  page: number;
  echelle: number;
  annotations: Annotation[];
  mots: MotPlace[] | null;
  /** Mode annotation : les gestes créent, choisissent, déplacent ; sinon les annotations se montrent seulement. */
  actif: boolean;
  outil: OutilAnnotation;
  teinteSurligneur: Teinte;
  teinteTexte: Teinte;
  taille: number;
  images: Record<string, ImageChargee>;
  selection: string | null;
  onSelection: (id: string | null) => void;
  onAjouter: (annotation: Annotation) => void;
  onModifier: (annotation: Annotation) => void;
  onSupprimer: (id: string) => void;
}

/** Geste en cours sur la couche : surlignage tracé, ou annotation déplacée ou agrandie. */
type Geste =
  | { sorte: "surligner"; debut: Point; courant: Point }
  | { sorte: "deplacer"; annotation: Annotation; debut: Point; courant: Point }
  | { sorte: "taille"; annotation: Annotation & { sorte: "image" }; debut: Point; courant: Point };

/** L'annotation décalée de (dx, dy) points. */
function decaler(annotation: Annotation, dx: number, dy: number): Annotation {
  switch (annotation.sorte) {
    case "surlignage":
      return { ...annotation, zones: annotation.zones.map((z) => ({ ...z, x: z.x + dx, y: z.y + dy })) };
    case "image":
      return { ...annotation, zone: { ...annotation.zone, x: annotation.zone.x + dx, y: annotation.zone.y + dy } };
    default:
      return { ...annotation, x: annotation.x + dx, y: annotation.y + dy };
  }
}

/** L'image agrandie de `dx` points en largeur, sans déformation. */
function agrandir(annotation: Annotation & { sorte: "image" }, dx: number, proportion: number): Annotation {
  const largeur = Math.max(12, annotation.zone.largeur + dx);
  return { ...annotation, zone: { ...annotation.zone, largeur, hauteur: largeur / proportion } };
}

/**
 * La couche d'annotations d'une page. Ce qui se crée (texte, note) reste un brouillon tant qu'il
 * est vide ; un déplacement ne compte qu'une fois relâché : une seule étape à annuler.
 */
export function CoucheAnnotations(props: PropsCouche) {
  const { page, echelle, annotations, actif, outil, selection, onSelection } = props;
  const couche = useRef<HTMLDivElement>(null);
  const [geste, setGeste] = useState<Geste | null>(null);
  /** Texte ou note en cours d'écriture (nouveau, ou existant que l'on modifie). */
  const [brouillon, setBrouillon] = useState<Annotation | null>(null);
  // Le dernier brouillon validé : un clic ailleurs puis la perte du focus ne le valident qu'une fois.
  const valide = useRef<Annotation | null>(null);

  // Hors du mode annotation, rien ne reste en cours.
  useEffect(() => {
    if (!actif) {
      setGeste(null);
      setBrouillon(null);
    }
  }, [actif]);

  const point = (evenement: { clientX: number; clientY: number }): Point => {
    const rect = couche.current!.getBoundingClientRect();
    return { x: (evenement.clientX - rect.left) / echelle, y: (evenement.clientY - rect.top) / echelle };
  };

  function valider(annotation: Annotation | null) {
    if (!annotation || valide.current === annotation) return;
    valide.current = annotation;
    setBrouillon(null);
    const vide = (annotation.sorte === "texte" || annotation.sorte === "note") && annotation.texte.trim() === "";
    const existante = annotations.some((a) => a.id === annotation.id);
    if (vide) {
      if (existante) props.onSupprimer(annotation.id);
      return;
    }
    if (existante) props.onModifier(annotation);
    else props.onAjouter(annotation);
    onSelection(annotation.id);
  }

  // La couche elle-même : surligner, poser une note ou un texte ; en sélection, un clic dans le vide
  // retire la sélection.
  function pointeurBas(evenement: EvenementPointeur<HTMLDivElement>) {
    if (!actif || evenement.button !== 0) return;
    const ici = point(evenement);
    if (brouillon) {
      valider(brouillon);
      return;
    }
    if (outil === "surligner") {
      evenement.preventDefault();
      evenement.currentTarget.setPointerCapture(evenement.pointerId);
      setGeste({ sorte: "surligner", debut: ici, courant: ici });
    } else if (outil === "note") {
      evenement.preventDefault();
      setBrouillon({ id: nouvelId(), sorte: "note", page, x: ici.x, y: ici.y, texte: "" });
    } else if (outil === "texte") {
      evenement.preventDefault();
      // Le point cliqué est le milieu de la première ligne.
      setBrouillon({ id: nouvelId(), sorte: "texte", page, x: ici.x, y: ici.y - (INTERLIGNE * props.taille) / 2, texte: "", taille: props.taille, teinte: props.teinteTexte });
    } else if (outil === "selection") {
      onSelection(null);
    }
  }

  function pointeurMouvement(evenement: EvenementPointeur<HTMLElement>) {
    if (!geste) return;
    setGeste({ ...geste, courant: point(evenement) });
  }

  function pointeurHaut() {
    if (!geste) return;
    const { debut, courant } = geste;
    if (geste.sorte === "surligner") {
      const zones = zonesSurlignees(props.mots ?? [], debut, courant);
      if (zones.length > 0) props.onAjouter({ id: nouvelId(), sorte: "surlignage", page, teinte: props.teinteSurligneur, zones });
    } else if (Math.hypot(courant.x - debut.x, courant.y - debut.y) > 0.5) {
      const finale =
        geste.sorte === "deplacer"
          ? decaler(geste.annotation, courant.x - debut.x, courant.y - debut.y)
          : agrandir(geste.annotation, courant.x - debut.x, props.images[geste.annotation.chemin]?.proportion ?? 1);
      props.onModifier(finale);
    }
    setGeste(null);
  }

  /** Début d'un déplacement (ou d'un changement de taille) d'une annotation, en sélection. */
  function saisir(annotation: Annotation, sorte: "deplacer" | "taille", evenement: EvenementPointeur<HTMLElement>) {
    if (!actif || outil !== "selection" || evenement.button !== 0) return;
    evenement.preventDefault();
    evenement.stopPropagation();
    onSelection(annotation.id);
    // L'annotation saisie suit le pointeur jusqu'à ce qu'on la lâche (la couche, en sélection, laisse
    // passer les clics vers le texte de la page).
    evenement.currentTarget.setPointerCapture(evenement.pointerId);
    const ici = point(evenement);
    if (sorte === "taille" && annotation.sorte === "image") setGeste({ sorte: "taille", annotation, debut: ici, courant: ici });
    else setGeste({ sorte: "deplacer", annotation, debut: ici, courant: ici });
  }

  /** L'annotation telle qu'elle s'affiche : pendant un geste, à sa place provisoire. */
  function affichee(annotation: Annotation): Annotation {
    if (!geste || geste.sorte === "surligner" || geste.annotation.id !== annotation.id) return annotation;
    const { debut, courant } = geste;
    return geste.sorte === "deplacer"
      ? decaler(annotation, courant.x - debut.x, courant.y - debut.y)
      : agrandir(geste.annotation, courant.x - debut.x, props.images[geste.annotation.chemin]?.proportion ?? 1);
  }

  const px = (points: number) => points * echelle;
  const enSelection = actif && outil === "selection";
  const visibles = annotations.filter((a) => a.id !== brouillon?.id);
  const apercu = geste?.sorte === "surligner" ? zonesSurlignees(props.mots ?? [], geste.debut, geste.courant) : [];

  function rendu(brute: Annotation): ReactNode {
    const annotation = affichee(brute);
    const choisie = selection === annotation.id || undefined;
    const prise = (sorte: "deplacer" | "taille" = "deplacer") => (evenement: EvenementPointeur<HTMLElement>) => saisir(brute, sorte, evenement);
    switch (annotation.sorte) {
      case "surlignage":
        return annotation.zones.map((zone, rang) => (
          <div
            key={`${annotation.id}-${rang}`}
            className="annotation-surlignage"
            data-selectionnee={choisie}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            style={{ left: px(zone.x), top: px(zone.y), width: px(zone.largeur), height: px(zone.hauteur), background: couleur(annotation.teinte, 0.5), pointerEvents: enSelection ? "auto" : "none" }}
          />
        ));
      case "note":
        return (
          <button
            key={annotation.id}
            type="button"
            className="annotation-note"
            data-selectionnee={choisie}
            title={annotation.texte}
            aria-label={annotation.texte || t.note}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            onDoubleClick={() => actif && setBrouillon(annotation)}
            style={{ left: px(annotation.x), top: px(annotation.y), pointerEvents: actif ? "auto" : "none" }}
          >
            <IconMessageCircle size={15} stroke={2} />
          </button>
        );
      case "texte":
        return (
          <div
            key={annotation.id}
            className="annotation-texte"
            data-selectionnee={choisie}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            onDoubleClick={() => actif && setBrouillon(annotation)}
            style={{
              left: px(annotation.x),
              top: px(annotation.y),
              fontSize: px(annotation.taille),
              color: couleur(annotation.teinte),
              pointerEvents: enSelection ? "auto" : "none",
            }}
          >
            {annotation.texte}
          </div>
        );
      case "image": {
        const image = props.images[annotation.chemin];
        return (
          <div
            key={annotation.id}
            className="annotation-image"
            data-selectionnee={choisie}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            style={{ left: px(annotation.zone.x), top: px(annotation.zone.y), width: px(annotation.zone.largeur), height: px(annotation.zone.hauteur), pointerEvents: enSelection ? "auto" : "none" }}
          >
            {image && <img src={image.adresse} alt="" draggable={false} />}
            {choisie && enSelection && <span className="poignee-annotation" aria-label={t.redimensionner} onPointerDown={prise("taille")} onPointerMove={pointeurMouvement} onPointerUp={pointeurHaut} />}
          </div>
        );
      }
    }
  }

  return (
    <div
      ref={couche}
      className="couche-annotations"
      data-outil={actif ? outil : undefined}
      data-annotations-page={page + 1}
      onPointerDown={pointeurBas}
      onPointerMove={pointeurMouvement}
      onPointerUp={pointeurHaut}
      onPointerCancel={() => setGeste(null)}
      style={{ pointerEvents: actif && outil !== "selection" ? "auto" : "none" }}
    >
      {visibles.map(rendu)}
      {apercu.map((zone: Zone, rang) => (
        <div key={`apercu-${rang}`} className="annotation-surlignage" style={{ left: px(zone.x), top: px(zone.y), width: px(zone.largeur), height: px(zone.hauteur), background: couleur(props.teinteSurligneur, 0.5) }} />
      ))}
      {brouillon?.sorte === "texte" && (
        <Textarea
          autoFocus
          autosize
          minRows={1}
          variant="unstyled"
          className="annotation-texte annotation-en-ecriture"
          placeholder={t.texte}
          value={brouillon.texte}
          onChange={(evenement) => setBrouillon({ ...brouillon, texte: evenement.currentTarget.value })}
          onBlur={() => valider(brouillon)}
          onKeyDown={(evenement) => evenement.key === "Escape" && valider(brouillon)}
          onPointerDown={(evenement) => evenement.stopPropagation()}
          styles={{ input: { fontSize: px(brouillon.taille), lineHeight: INTERLIGNE, color: couleur(brouillon.teinte), padding: 0, minHeight: 0, fontFamily: "Helvetica, Arial, sans-serif" } }}
          style={{ left: px(brouillon.x), top: px(brouillon.y), minWidth: px(brouillon.taille * 8), pointerEvents: "auto" }}
          data-texte-en-ecriture
        />
      )}
      {brouillon?.sorte === "note" && (
        <Popover opened onClose={() => valider(brouillon)} position="bottom-start" withArrow shadow="md" trapFocus>
          <Popover.Target>
            <span className="annotation-note" style={{ left: px(brouillon.x), top: px(brouillon.y) }}>
              <IconMessageCircle size={15} stroke={2} />
            </span>
          </Popover.Target>
          <Popover.Dropdown onPointerDown={(evenement) => evenement.stopPropagation()}>
            <Textarea
              autoFocus
              autosize
              minRows={3}
              w={240}
              placeholder={t.note}
              value={brouillon.texte}
              onChange={(evenement) => setBrouillon({ ...brouillon, texte: evenement.currentTarget.value })}
              onKeyDown={(evenement) => evenement.key === "Escape" && valider(brouillon)}
              data-note-en-ecriture
            />
            <Group justify="flex-end" mt={8}>
              <Button size="xs" color="eau" onClick={() => valider(brouillon)}>
                OK
              </Button>
            </Group>
          </Popover.Dropdown>
        </Popover>
      )}
    </div>
  );
}
