// Éditeur de PDF : la barre d'annotation du lecteur, et sur chaque page la couche qui montre les
// annotations et reçoit les gestes de l'outil choisi. Positions en points de la page affichée (comme
// les mots), converties en pixels par l'échelle du lecteur.
import { useEffect, useRef, useState, type PointerEvent as EvenementPointeur, type ReactNode, type SVGAttributes } from "react";
import { ActionIcon, Box, Button, Checkbox, ColorSwatch, Group, NumberInput, Popover, SegmentedControl, Select, Text, Textarea, Tooltip } from "@mantine/core";
import {
  IconArrowBackUp,
  IconArrowForwardUp,
  IconArrowNarrowRight,
  IconCheck,
  IconCircle,
  IconCursorText,
  IconDeviceFloppy,
  IconEraser,
  IconHighlight,
  IconLine,
  IconMessageCircle,
  IconPhoto,
  IconPointer,
  IconRectangle,
  IconRubberStamp,
  IconScribble,
  IconShape,
  IconSquare,
  IconStrikethrough,
  IconTrash,
  IconTypography,
  IconUnderline,
} from "@tabler/icons-react";
import {
  branche,
  correctionDe,
  dateDuJour,
  extraitSurligne,
  INTERLIGNE,
  motsDeLaLigne,
  nouvelId,
  policeCss,
  prolonger,
  RVB,
  TAMPONS,
  TEINTES_SURLIGNEUR,
  TEINTES_TEXTE,
  TEINTES_TRAIT,
  zoneDe,
  zoneEntre,
  zonesSurlignees,
  zoneTampon,
  type Annotation,
  type Forme,
  type MotComplet,
  type MotTampon,
  type Point,
  type Teinte,
  type Zone,
} from "../../donnees/annotations";
import { versLaPage, type Rotation } from "../../donnees/affichage";
import { textes } from "../../textes";

const t = textes.lecteur.annotation;

export type OutilAnnotation =
  | "selection"
  | "surligner"
  | "souligner"
  | "barrer"
  | "note"
  | "texte"
  | "forme"
  | "dessin"
  | "tampon"
  | "image"
  | "corriger"
  | "correcteur"
  | "gomme";

const ICONES: Record<OutilAnnotation, typeof IconPointer> = {
  selection: IconPointer,
  surligner: IconHighlight,
  souligner: IconUnderline,
  barrer: IconStrikethrough,
  note: IconMessageCircle,
  texte: IconTypography,
  forme: IconShape,
  dessin: IconScribble,
  tampon: IconRubberStamp,
  image: IconPhoto,
  corriger: IconCursorText,
  correcteur: IconSquare,
  gomme: IconEraser,
};
const ORDRE: OutilAnnotation[] = ["selection", "surligner", "souligner", "barrer", "note", "texte", "forme", "dessin", "tampon", "image", "corriger", "correcteur", "gomme"];

const ICONES_FORMES: Record<Forme, typeof IconPointer> = { rectangle: IconRectangle, ellipse: IconCircle, ligne: IconLine, fleche: IconArrowNarrowRight };
const FORMES: Forme[] = ["rectangle", "ellipse", "ligne", "fleche"];

/** L'outil qui crée chaque sorte d'annotation. */
const OUTIL_DE: Record<Annotation["sorte"], OutilAnnotation> = {
  surlignage: "surligner",
  soulignage: "souligner",
  barre: "barrer",
  note: "note",
  texte: "texte",
  forme: "forme",
  dessin: "dessin",
  tampon: "tampon",
  image: "image",
  correction: "corriger",
  blanc: "correcteur",
};

/** L'icône d'une annotation (celle de son outil, ou de sa forme), pour la liste des commentaires. */
export const iconeDe = (annotation: Annotation) => (annotation.sorte === "forme" ? ICONES_FORMES[annotation.forme] : ICONES[OUTIL_DE[annotation.sorte]]);

/** Outils qui tracent un trait : soulignement, rature, formes, dessin. */
const OUTILS_TRAIT: OutilAnnotation[] = ["souligner", "barrer", "forme", "dessin"];

export const couleur = (teinte: Teinte, opacite = 1) => `rgba(${RVB[teinte].join(", ")}, ${opacite})`;

/** Image choisie pour être posée : son adresse dans la page, et sa largeur sur sa hauteur. */
export interface ImageChargee {
  adresse: string;
  proportion: number;
}

/** Réglages des outils de commentaire : couleur des traits, forme, épaisseur, tampon et sa date. */
export interface ReglagesCommentaires {
  teinteTrait: Teinte;
  forme: Forme;
  epaisseur: number;
  tampon: MotTampon;
  avecDate: boolean;
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
  reglages: ReglagesCommentaires;
  onReglages: (reglages: ReglagesCommentaires) => void;
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

/** La barre d'annotation : les outils, leurs réglages, annuler, enregistrer le PDF annoté. */
export function BarreAnnotations(props: PropsBarre) {
  const { outil, onOutil, nombre, reglages, onReglages } = props;
  const trait = OUTILS_TRAIT.includes(outil);
  const teintes = outil === "texte" ? TEINTES_TEXTE : outil === "surligner" ? TEINTES_SURLIGNEUR : trait ? TEINTES_TRAIT : [];
  const teinte = outil === "texte" ? props.teinteTexte : trait ? reglages.teinteTrait : props.teinteSurligneur;
  const choisirTeinte = (cle: Teinte) => (outil === "texte" ? props.onTeinteTexte(cle) : trait ? onReglages({ ...reglages, teinteTrait: cle }) : props.onTeinteSurligneur(cle));
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
          {outil === "forme" && (
            <Group gap={2} wrap="nowrap" pl={4} role="radiogroup" aria-label={t.formes.titre}>
              {FORMES.map((cle) => {
                const Icone = ICONES_FORMES[cle];
                return (
                  <Tooltip key={cle} label={t.formes[cle]}>
                    <ActionIcon
                      variant={reglages.forme === cle ? "light" : "subtle"}
                      color={reglages.forme === cle ? "eau" : "gray"}
                      size={28}
                      radius="md"
                      role="radio"
                      aria-checked={reglages.forme === cle}
                      aria-label={t.formes[cle]}
                      onClick={() => onReglages({ ...reglages, forme: cle })}
                      data-forme={cle}
                    >
                      <Icone size={17} stroke={1.8} />
                    </ActionIcon>
                  </Tooltip>
                );
              })}
            </Group>
          )}
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
          {(outil === "forme" || outil === "dessin") && (
            <SegmentedControl
              size="xs"
              value={String(reglages.epaisseur)}
              onChange={(valeur) => onReglages({ ...reglages, epaisseur: Number(valeur) })}
              aria-label={t.epaisseur}
              data={[
                { value: "1", label: t.epaisseurs.fin },
                { value: "2", label: t.epaisseurs.moyen },
                { value: "4", label: t.epaisseurs.epais },
              ]}
            />
          )}
          {outil === "tampon" && (
            <Group gap={8} wrap="nowrap" pl={4}>
              <Select
                size="xs"
                w={150}
                value={reglages.tampon}
                onChange={(valeur) => valeur && onReglages({ ...reglages, tampon: valeur as MotTampon })}
                allowDeselect={false}
                aria-label={t.tampon}
                data={(Object.keys(TAMPONS) as MotTampon[]).map((cle) => ({ value: cle, label: t.tampons[cle] }))}
                data-choix-tampon
              />
              <Checkbox size="xs" label={t.avecDate} checked={reglages.avecDate} onChange={(evenement) => onReglages({ ...reglages, avecDate: evenement.currentTarget.checked })} />
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
  /** Rotation de l'affichage : la couche est tournée avec sa page, le pointeur ramené à la page droite. */
  rotation?: Rotation;
  annotations: Annotation[];
  mots: MotComplet[] | null;
  /** Mode annotation : les gestes créent, choisissent, déplacent ; sinon les annotations se montrent seulement. */
  actif: boolean;
  outil: OutilAnnotation;
  teinteSurligneur: Teinte;
  teinteTexte: Teinte;
  taille: number;
  reglages: ReglagesCommentaires;
  images: Record<string, ImageChargee>;
  selection: string | null;
  onSelection: (id: string | null) => void;
  onAjouter: (annotation: Annotation) => void;
  onModifier: (annotation: Annotation) => void;
  onSupprimer: (id: string) => void;
  /** Annotations effacées d'un geste de gomme : une seule étape à annuler. */
  onEffacer: (ids: string[]) => void;
}

/** Passage surligné, souligné ou barré : la même façon de choisir les mots. */
type StyleTexte = "surlignage" | "soulignage" | "barre";

/** Geste en cours sur la couche : tracé, gomme passée, ou annotation déplacée ou agrandie. */
type Geste =
  | { sorte: "surligner"; style: StyleTexte; debut: Point; courant: Point }
  | { sorte: "gommer"; effacees: string[] }
  | { sorte: "corriger"; debut: Point; courant: Point }
  | { sorte: "correcteur"; debut: Point; courant: Point }
  | { sorte: "forme"; debut: Point; courant: Point }
  | { sorte: "dessin"; points: Point[] }
  | { sorte: "deplacer"; annotation: Annotation; debut: Point; courant: Point }
  | { sorte: "taille"; annotation: Annotation & { sorte: "image" | "tampon" }; debut: Point; courant: Point };

/** L'annotation décalée de (dx, dy) points. */
function decaler(annotation: Annotation, dx: number, dy: number): Annotation {
  const point = (p: Point) => ({ x: p.x + dx, y: p.y + dy });
  switch (annotation.sorte) {
    case "surlignage":
    case "soulignage":
    case "barre":
      return { ...annotation, zones: annotation.zones.map((z) => ({ ...z, x: z.x + dx, y: z.y + dy })) };
    case "image":
    case "blanc":
    case "tampon":
      return { ...annotation, zone: { ...annotation.zone, x: annotation.zone.x + dx, y: annotation.zone.y + dy } };
    case "forme":
      return { ...annotation, debut: point(annotation.debut), fin: point(annotation.fin) };
    case "dessin":
      return { ...annotation, points: annotation.points.map(point) };
    // Une correction reste à la place du texte qu'elle remplace.
    case "correction":
      return annotation;
    default:
      return { ...annotation, x: annotation.x + dx, y: annotation.y + dy };
  }
}

/** L'image ou le tampon agrandi de `dx` points en largeur, sans déformation. */
function agrandir(annotation: Annotation & { sorte: "image" | "tampon" }, dx: number, proportion: number): Annotation {
  const largeur = Math.max(annotation.sorte === "tampon" ? 40 : 12, annotation.zone.largeur + dx);
  return { ...annotation, zone: { ...annotation.zone, largeur, hauteur: largeur / proportion } };
}

/** Dessin SVG d'une forme ou d'un trait, en points de la page (le cadre SVG fait l'échelle). */
function trace(annotation: Annotation & { sorte: "forme" | "dessin" }, attributs: SVGAttributes<SVGElement>): ReactNode {
  if (annotation.sorte === "dessin") return <polyline points={annotation.points.map((p) => `${p.x},${p.y}`).join(" ")} {...attributs} />;
  const { debut, fin } = annotation;
  const zone = zoneEntre(debut, fin);
  switch (annotation.forme) {
    case "rectangle":
      return <rect x={zone.x} y={zone.y} width={zone.largeur} height={zone.hauteur} {...attributs} />;
    case "ellipse":
      return <ellipse cx={zone.x + zone.largeur / 2} cy={zone.y + zone.hauteur / 2} rx={zone.largeur / 2} ry={zone.hauteur / 2} {...attributs} />;
    case "ligne":
      return <line x1={debut.x} y1={debut.y} x2={fin.x} y2={fin.y} {...attributs} />;
    case "fleche": {
      // Deux branches à 30° de part et d'autre de la ligne, au bout (comme le cœur).
      const retour = Math.atan2(debut.y - fin.y, debut.x - fin.x);
      const longueur = branche(annotation.epaisseur);
      const bout = (ecart: number) => `${fin.x + longueur * Math.cos(retour + ecart)},${fin.y + longueur * Math.sin(retour + ecart)}`;
      return (
        <g {...attributs}>
          <line x1={debut.x} y1={debut.y} x2={fin.x} y2={fin.y} />
          <polyline points={`${bout(-Math.PI / 6)} ${fin.x},${fin.y} ${bout(Math.PI / 6)}`} />
        </g>
      );
    }
  }
}

/**
 * La couche d'annotations d'une page. Ce qui se crée (texte, note) reste un brouillon tant qu'il
 * est vide ; un déplacement ne compte qu'une fois relâché : une seule étape à annuler.
 */
export function CoucheAnnotations(props: PropsCouche) {
  const { page, echelle, annotations, actif, outil, selection, onSelection, reglages, rotation = 0 } = props;
  const couche = useRef<HTMLDivElement>(null);
  const [geste, setGeste] = useState<Geste | null>(null);
  /** Texte ou note en cours d'écriture (nouveau, ou existant que l'on modifie). */
  const [brouillon, setBrouillon] = useState<Annotation | null>(null);
  // Le dernier brouillon validé : un clic ailleurs puis la perte du focus ne le valident qu'une fois.
  const valide = useRef<Annotation | null>(null);
  /** Outil Corriger : les mots sous le pointeur, entourés avant le clic. */
  const [survol, setSurvol] = useState<number[]>([]);

  // Hors du mode annotation, rien ne reste en cours.
  useEffect(() => {
    if (!actif) {
      setGeste(null);
      setBrouillon(null);
    }
  }, [actif]);

  const point = (evenement: { clientX: number; clientY: number }): Point => {
    // La boîte affichée de la couche (tournée), et sa taille droite : offsetWidth ignore la rotation.
    const element = couche.current!;
    const rect = element.getBoundingClientRect();
    const ici = versLaPage(evenement.clientX - rect.left, evenement.clientY - rect.top, element.offsetWidth, element.offsetHeight, rotation);
    return { x: ici.x / echelle, y: ici.y / echelle };
  };

  /** Largeur sur hauteur d'une image ou d'un tampon, gardée quand on l'agrandit. */
  const proportion = (annotation: Annotation & { sorte: "image" | "tampon" }) =>
    (annotation.sorte === "image" ? props.images[annotation.chemin]?.proportion : undefined) ?? annotation.zone.largeur / Math.max(1, annotation.zone.hauteur);

  /** Les annotations de cette page sous le pointeur (là où passe la gomme). */
  function touchees(evenement: { clientX: number; clientY: number }): string[] {
    const ids = document
      .elementsFromPoint(evenement.clientX, evenement.clientY)
      .map((element) => element.closest("[data-annotation-id]")?.getAttribute("data-annotation-id"))
      .filter((id): id is string => !!id && annotations.some((a) => a.id === id));
    return [...new Set(ids)];
  }

  /** Abandonne le brouillon (Échap) : la perte du focus qui suit ne le valide pas. */
  function abandonner() {
    valide.current = brouillon;
    setBrouillon(null);
  }

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
    // Une correction qui ne change rien n'est pas gardée.
    if (annotation.sorte === "correction" && !existante && annotation.texte === annotation.ancien) return;
    if (existante) props.onModifier(annotation);
    else props.onAjouter(annotation);
    onSelection(annotation.id);
  }

  /** Commence un geste qui suit le pointeur jusqu'à ce qu'on le lâche. */
  function capturer(evenement: EvenementPointeur<HTMLDivElement>, suivant: Geste) {
    evenement.preventDefault();
    evenement.currentTarget.setPointerCapture(evenement.pointerId);
    setGeste(suivant);
  }

  // La couche elle-même : tracer, poser une note, un texte ou un tampon ; en sélection, un clic
  // dans le vide retire la sélection.
  function pointeurBas(evenement: EvenementPointeur<HTMLDivElement>) {
    if (!actif || evenement.button !== 0) return;
    const ici = point(evenement);
    if (brouillon) {
      valider(brouillon);
      return;
    }
    if (outil === "surligner" || outil === "souligner" || outil === "barrer") {
      const style: StyleTexte = outil === "souligner" ? "soulignage" : outil === "barrer" ? "barre" : "surlignage";
      capturer(evenement, { sorte: "surligner", style, debut: ici, courant: ici });
    } else if (outil === "note") {
      evenement.preventDefault();
      setBrouillon({ id: nouvelId(), sorte: "note", page, x: ici.x, y: ici.y, texte: "" });
    } else if (outil === "texte") {
      evenement.preventDefault();
      // Le point cliqué est le milieu de la première ligne.
      setBrouillon({ id: nouvelId(), sorte: "texte", page, x: ici.x, y: ici.y - (INTERLIGNE * props.taille) / 2, texte: "", taille: props.taille, teinte: props.teinteTexte });
    } else if (outil === "forme") {
      capturer(evenement, { sorte: "forme", debut: ici, courant: ici });
    } else if (outil === "dessin") {
      capturer(evenement, { sorte: "dessin", points: [ici] });
    } else if (outil === "tampon") {
      evenement.preventDefault();
      const tampon: Annotation = { id: nouvelId(), sorte: "tampon", page, zone: zoneTampon(reglages.tampon, ici), mot: reglages.tampon, date: reglages.avecDate ? dateDuJour() : null };
      props.onAjouter(tampon);
      onSelection(tampon.id);
    } else if (outil === "corriger" || outil === "correcteur") {
      capturer(evenement, { sorte: outil, debut: ici, courant: ici });
      setSurvol([]);
    } else if (outil === "gomme") {
      capturer(evenement, { sorte: "gommer", effacees: touchees(evenement) });
    } else if (outil === "selection") {
      onSelection(null);
    }
  }

  function pointeurMouvement(evenement: EvenementPointeur<HTMLElement | SVGElement>) {
    if (!geste) {
      // Corriger : on voit quel mot le clic prendra.
      if (actif && outil === "corriger" && props.mots) {
        const ici = point(evenement);
        const mots = motsDeLaLigne(props.mots, ici, ici);
        if (mots.join() !== survol.join()) setSurvol(mots);
      }
      return;
    }
    if (geste.sorte === "gommer") {
      const nouvelles = touchees(evenement).filter((id) => !geste.effacees.includes(id));
      if (nouvelles.length > 0) setGeste({ sorte: "gommer", effacees: [...geste.effacees, ...nouvelles] });
      return;
    }
    if (geste.sorte === "dessin") {
      const suite = prolonger(geste.points, point(evenement));
      if (suite !== geste.points) setGeste({ sorte: "dessin", points: suite });
      return;
    }
    setGeste({ ...geste, courant: point(evenement) });
  }

  function pointeurHaut() {
    if (!geste) return;
    if (geste.sorte === "gommer") {
      if (geste.effacees.length > 0) props.onEffacer(geste.effacees);
      setGeste(null);
      return;
    }
    if (geste.sorte === "dessin") {
      if (geste.points.length >= 2) props.onAjouter({ id: nouvelId(), sorte: "dessin", page, points: geste.points, teinte: reglages.teinteTrait, epaisseur: reglages.epaisseur });
      setGeste(null);
      return;
    }
    const { debut, courant } = geste;
    if (geste.sorte === "corriger") {
      const rangs = motsDeLaLigne(props.mots ?? [], debut, courant);
      if (rangs.length > 0) setBrouillon(correctionDe(props.mots!, rangs, page));
      setGeste(null);
      return;
    }
    if (geste.sorte === "correcteur") {
      const zone = zoneEntre(debut, courant);
      if (zone.largeur > 2 && zone.hauteur > 2) props.onAjouter({ id: nouvelId(), sorte: "blanc", page, zone });
      setGeste(null);
      return;
    }
    if (geste.sorte === "forme") {
      if (Math.hypot(courant.x - debut.x, courant.y - debut.y) >= 2)
        props.onAjouter({ id: nouvelId(), sorte: "forme", page, forme: reglages.forme, debut, fin: courant, teinte: reglages.teinteTrait, epaisseur: reglages.epaisseur });
      setGeste(null);
      return;
    }
    if (geste.sorte === "surligner") {
      const zones = zonesSurlignees(props.mots ?? [], debut, courant);
      if (zones.length > 0) {
        const id = nouvelId();
        // Le texte pris, gardé dans le commentaire (comme Acrobat) et montré dans la liste.
        const extrait = extraitSurligne(props.mots ?? [], debut, courant) || undefined;
        if (geste.style === "surlignage") props.onAjouter({ id, sorte: "surlignage", page, teinte: props.teinteSurligneur, zones, extrait });
        else if (geste.style === "soulignage") props.onAjouter({ id, sorte: "soulignage", page, teinte: reglages.teinteTrait, zones, extrait });
        else props.onAjouter({ id, sorte: "barre", page, teinte: reglages.teinteTrait, zones, extrait });
      }
    } else if (Math.hypot(courant.x - debut.x, courant.y - debut.y) > 0.5) {
      const finale = geste.sorte === "deplacer" ? decaler(geste.annotation, courant.x - debut.x, courant.y - debut.y) : agrandir(geste.annotation, courant.x - debut.x, proportion(geste.annotation));
      props.onModifier(finale);
    }
    setGeste(null);
  }

  /** Début d'un déplacement (ou d'un changement de taille) d'une annotation, en sélection. */
  function saisir(annotation: Annotation, sorte: "deplacer" | "taille", evenement: EvenementPointeur<HTMLElement | SVGElement>) {
    if (!actif || outil !== "selection" || evenement.button !== 0) return;
    evenement.preventDefault();
    evenement.stopPropagation();
    onSelection(annotation.id);
    // L'annotation saisie suit le pointeur jusqu'à ce qu'on la lâche (la couche, en sélection, laisse
    // passer les clics vers le texte de la page).
    evenement.currentTarget.setPointerCapture(evenement.pointerId);
    const ici = point(evenement);
    if (sorte === "taille" && (annotation.sorte === "image" || annotation.sorte === "tampon")) setGeste({ sorte: "taille", annotation, debut: ici, courant: ici });
    else setGeste({ sorte: "deplacer", annotation, debut: ici, courant: ici });
  }

  /** L'annotation telle qu'elle s'affiche : pendant un geste, à sa place provisoire. */
  function affichee(annotation: Annotation): Annotation {
    if (!geste || (geste.sorte !== "deplacer" && geste.sorte !== "taille") || geste.annotation.id !== annotation.id) return annotation;
    const { debut, courant } = geste;
    return geste.sorte === "deplacer" ? decaler(annotation, courant.x - debut.x, courant.y - debut.y) : agrandir(geste.annotation, courant.x - debut.x, proportion(geste.annotation));
  }

  const px = (points: number) => points * echelle;
  const enSelection = actif && outil === "selection";
  // En sélection, on saisit les annotations ; avec la gomme, on passe dessus : elles reçoivent le pointeur.
  const touchables = actif && (outil === "selection" || outil === "gomme");
  const effacees = geste?.sorte === "gommer" ? geste.effacees : [];
  const visibles = annotations.filter((a) => a.id !== brouillon?.id && !effacees.includes(a.id));
  const apercu = geste?.sorte === "surligner" ? zonesSurlignees(props.mots ?? [], geste.debut, geste.courant) : [];
  // Corriger : les mots que le geste prend ; correcteur : le cadre tracé.
  const motsVises = geste?.sorte === "corriger" ? motsDeLaLigne(props.mots ?? [], geste.debut, geste.courant) : actif && outil === "corriger" ? survol : [];
  const cadreBlanc = geste?.sorte === "correcteur" ? zoneEntre(geste.debut, geste.courant) : null;
  /** Haut d'une ligne de texte affichée en hauteur de ligne 1 (ligne de base à 0,84 de la taille). */
  const hautDeLigne = (ligne: number, taille: number) => ligne - 0.84 * taille;
  const rvb = (c: [number, number, number]) => `rgb(${c.join(", ")})`;

  /** Trait sous les mots, ou au travers : à la même hauteur que le cœur l'écrira. */
  function traitTexte(zone: Zone, style: "soulignage" | "barre", teinte: Teinte): ReactNode {
    const epaisseur = Math.max(1, px(Math.max(0.6, zone.hauteur * 0.07)));
    return <span className="trait-texte" style={{ top: px(zone.hauteur * (style === "barre" ? 0.55 : 0.9)) - epaisseur / 2, height: epaisseur, background: couleur(teinte) }} />;
  }

  /** Forme ou dessin dans un cadre SVG à l'échelle de la page ; `interactif` : saisissable et gommable. */
  function dessinSvg(annotation: Annotation & { sorte: "forme" | "dessin" }, interactif: boolean, choisie?: boolean): ReactNode {
    const marge = annotation.epaisseur + (annotation.sorte === "forme" && annotation.forme === "fleche" ? branche(annotation.epaisseur) : 0) + 6 / echelle;
    const zone = zoneDe(annotation);
    const cadre = { x: zone.x - marge, y: zone.y - marge, largeur: zone.largeur + 2 * marge, hauteur: zone.hauteur + 2 * marge };
    return (
      <svg
        key={annotation.id}
        className="annotation-dessin"
        viewBox={`${cadre.x} ${cadre.y} ${cadre.largeur} ${cadre.hauteur}`}
        style={{ left: px(cadre.x), top: px(cadre.y), width: px(cadre.largeur), height: px(cadre.hauteur) }}
      >
        {choisie && <rect className="cadre-selection" x={zone.x - 2} y={zone.y - 2} width={zone.largeur + 4} height={zone.hauteur + 4} />}
        <g
          data-annotation-id={interactif ? annotation.id : undefined}
          onPointerDown={interactif && enSelection ? (evenement) => saisir(annotation, "deplacer", evenement) : undefined}
          onPointerMove={interactif ? pointeurMouvement : undefined}
          onPointerUp={interactif ? pointeurHaut : undefined}
        >
          {trace(annotation, { stroke: couleur(annotation.teinte), strokeWidth: annotation.epaisseur, fill: "none", strokeLinecap: "round", strokeLinejoin: "round", pointerEvents: "none" })}
          {interactif && touchables && trace(annotation, { stroke: "transparent", strokeWidth: annotation.epaisseur + 8 / echelle, fill: "none", pointerEvents: "stroke" })}
        </g>
      </svg>
    );
  }

  function rendu(brute: Annotation): ReactNode {
    const annotation = affichee(brute);
    const choisie = selection === annotation.id || undefined;
    const prise = (sorte: "deplacer" | "taille" = "deplacer") => (evenement: EvenementPointeur<HTMLElement>) => saisir(brute, sorte, evenement);
    switch (annotation.sorte) {
      case "surlignage":
        return annotation.zones.map((zone, rang) => (
          <div
            key={`${annotation.id}-${rang}`}
            data-annotation-id={annotation.id}
            className="annotation-surlignage"
            data-selectionnee={choisie}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            style={{ left: px(zone.x), top: px(zone.y), width: px(zone.largeur), height: px(zone.hauteur), background: couleur(annotation.teinte, 0.5), pointerEvents: touchables ? "auto" : "none" }}
          />
        ));
      case "soulignage":
      case "barre":
        return annotation.zones.map((zone, rang) => (
          <div
            key={`${annotation.id}-${rang}`}
            data-annotation-id={annotation.id}
            className="annotation-trait"
            data-selectionnee={choisie}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            style={{ left: px(zone.x), top: px(zone.y), width: px(zone.largeur), height: px(zone.hauteur), pointerEvents: touchables ? "auto" : "none" }}
          >
            {traitTexte(zone, annotation.sorte, annotation.teinte)}
          </div>
        ));
      case "forme":
      case "dessin":
        return dessinSvg(annotation, true, choisie);
      case "tampon": {
        const { zone } = annotation;
        const { texte, rvb: encre } = TAMPONS[annotation.mot];
        const bord = Math.min(4, Math.max(0.8, zone.hauteur * 0.06));
        const utile = zone.largeur - 2 * bord - 0.24 * zone.hauteur;
        const taille = Math.min(zone.hauteur * (annotation.date ? 0.42 : 0.56), utile / (0.74 * texte.length));
        return (
          <div
            key={annotation.id}
            data-annotation-id={annotation.id}
            className="annotation-tampon"
            data-selectionnee={choisie}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            style={{
              left: px(zone.x),
              top: px(zone.y),
              width: px(zone.largeur),
              height: px(zone.hauteur),
              borderWidth: px(bord),
              borderRadius: px(Math.min(zone.hauteur, zone.largeur) * 0.14),
              borderColor: rvb(encre),
              color: rvb(encre),
              pointerEvents: touchables ? "auto" : "none",
            }}
          >
            <span style={{ fontSize: px(taille) }}>{texte}</span>
            {annotation.date && <span style={{ fontSize: px(Math.min(zone.hauteur * 0.2, utile / (0.62 * annotation.date.length))) }}>{annotation.date}</span>}
            {choisie && enSelection && <span className="poignee-annotation" aria-label={t.redimensionner} onPointerDown={prise("taille")} onPointerMove={pointeurMouvement} onPointerUp={pointeurHaut} />}
          </div>
        );
      }
      case "note":
        return (
          <button
            key={annotation.id}
            type="button"
            data-annotation-id={annotation.id}
            className="annotation-note"
            data-selectionnee={choisie}
            title={annotation.texte}
            aria-label={annotation.texte || t.note}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            onDoubleClick={() => actif && outil !== "gomme" && setBrouillon(annotation)}
            style={{ left: px(annotation.x), top: px(annotation.y), pointerEvents: actif ? "auto" : "none" }}
          >
            <IconMessageCircle size={15} stroke={2} />
          </button>
        );
      case "texte":
        return (
          <div
            key={annotation.id}
            data-annotation-id={annotation.id}
            className="annotation-texte"
            data-selectionnee={choisie}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            onDoubleClick={() => actif && outil !== "gomme" && setBrouillon(annotation)}
            style={{
              left: px(annotation.x),
              top: px(annotation.y),
              fontSize: px(annotation.taille),
              color: couleur(annotation.teinte),
              pointerEvents: touchables ? "auto" : "none",
            }}
          >
            {annotation.texte}
          </div>
        );
      case "correction":
        return (
          <div
            key={annotation.id}
            data-annotation-id={annotation.id}
            className="annotation-correction"
            data-selectionnee={choisie}
            title={`« ${annotation.ancien} » → « ${annotation.texte} »`}
            onPointerDown={
              enSelection
                ? (evenement) => {
                    evenement.stopPropagation();
                    onSelection(annotation.id);
                  }
                : undefined
            }
            onDoubleClick={() => actif && outil !== "gomme" && setBrouillon(annotation)}
            style={{ left: px(annotation.zone.x) - 1, top: px(annotation.zone.y) - 1, width: px(annotation.zone.largeur) + 2, height: px(annotation.zone.hauteur) + 2, pointerEvents: touchables ? "auto" : "none" }}
          >
            <span
              className="texte-corrige"
              style={{
                top: px(hautDeLigne(annotation.ligne, annotation.taille) - annotation.zone.y) + 1,
                ...(annotation.droite ? { right: 1 } : { left: 1 }),
                fontSize: px(annotation.taille),
                fontFamily: policeCss(annotation.famille),
                fontWeight: annotation.gras ? 700 : 400,
                fontStyle: annotation.italique ? "italic" : "normal",
                color: rvb(annotation.couleur),
              }}
            >
              {annotation.texte}
            </span>
          </div>
        );
      case "blanc":
        return (
          <div
            key={annotation.id}
            data-annotation-id={annotation.id}
            className="annotation-blanc"
            data-selectionnee={choisie}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            style={{ left: px(annotation.zone.x), top: px(annotation.zone.y), width: px(annotation.zone.largeur), height: px(annotation.zone.hauteur), pointerEvents: touchables ? "auto" : "none" }}
          />
        );
      case "image": {
        const image = props.images[annotation.chemin];
        return (
          <div
            key={annotation.id}
            data-annotation-id={annotation.id}
            className="annotation-image"
            data-selectionnee={choisie}
            onPointerDown={enSelection ? prise() : undefined}
            onPointerMove={pointeurMouvement}
            onPointerUp={pointeurHaut}
            style={{ left: px(annotation.zone.x), top: px(annotation.zone.y), width: px(annotation.zone.largeur), height: px(annotation.zone.hauteur), pointerEvents: touchables ? "auto" : "none" }}
          >
            {image && <img src={image.adresse} alt="" draggable={false} />}
            {choisie && enSelection && <span className="poignee-annotation" aria-label={t.redimensionner} onPointerDown={prise("taille")} onPointerMove={pointeurMouvement} onPointerUp={pointeurHaut} />}
          </div>
        );
      }
    }
  }

  // Ce qui se trace en ce moment : forme ou trait de dessin, en aperçu.
  const traceEnCours: (Annotation & { sorte: "forme" | "dessin" }) | null =
    geste?.sorte === "forme"
      ? { id: "trace", sorte: "forme", page, forme: reglages.forme, debut: geste.debut, fin: geste.courant, teinte: reglages.teinteTrait, epaisseur: reglages.epaisseur }
      : geste?.sorte === "dessin" && geste.points.length >= 2
        ? { id: "trace", sorte: "dessin", page, points: geste.points, teinte: reglages.teinteTrait, epaisseur: reglages.epaisseur }
        : null;

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
      {traceEnCours && dessinSvg(traceEnCours, false)}
      {motsVises.map((rang) => {
        const mot = props.mots![rang];
        return <div key={`vise-${rang}`} className="mot-a-corriger" style={{ left: px(mot.x) - 2, top: px(mot.y) - 1, width: px(mot.longueur) + 4, height: px(mot.taille) + 2 }} />;
      })}
      {cadreBlanc && <div className="annotation-blanc" data-trace style={{ left: px(cadreBlanc.x), top: px(cadreBlanc.y), width: px(cadreBlanc.largeur), height: px(cadreBlanc.hauteur) }} />}
      {brouillon?.sorte === "correction" && (() => {
        const largeur = Math.max(px(brouillon.zone.largeur) + 48, 120);
        return (
          <input
            autoFocus
            className="correction-en-ecriture"
            aria-label={t.outils.corriger.titre}
            value={brouillon.texte}
            onChange={(evenement) => setBrouillon({ ...brouillon, texte: evenement.currentTarget.value })}
            onBlur={() => valider(brouillon)}
            onKeyDown={(evenement) => {
              if (evenement.key === "Enter") valider(brouillon);
              else if (evenement.key === "Escape") abandonner();
            }}
            onFocus={(evenement) => evenement.currentTarget.select()}
            onPointerDown={(evenement) => evenement.stopPropagation()}
            style={{
              left: brouillon.droite ? px(brouillon.fin) - largeur + 3 : px(brouillon.x) - 3,
              top: px(hautDeLigne(brouillon.ligne, brouillon.taille)) - 3,
              width: largeur,
              height: px(brouillon.taille) + 6,
              fontSize: px(brouillon.taille),
              fontFamily: policeCss(brouillon.famille),
              fontWeight: brouillon.gras ? 700 : 400,
              fontStyle: brouillon.italique ? "italic" : "normal",
              color: rvb(brouillon.couleur),
              textAlign: brouillon.droite ? "right" : "left",
            }}
            data-correction-en-ecriture
          />
        );
      })()}
      {geste?.sorte === "surligner" &&
        apercu.map((zone: Zone, rang) =>
          geste.style === "surlignage" ? (
            <div key={`apercu-${rang}`} className="annotation-surlignage" style={{ left: px(zone.x), top: px(zone.y), width: px(zone.largeur), height: px(zone.hauteur), background: couleur(props.teinteSurligneur, 0.5) }} />
          ) : (
            <div key={`apercu-${rang}`} className="annotation-trait" style={{ left: px(zone.x), top: px(zone.y), width: px(zone.largeur), height: px(zone.hauteur) }}>
              {traitTexte(zone, geste.style, reglages.teinteTrait)}
            </div>
          ),
        )}
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
