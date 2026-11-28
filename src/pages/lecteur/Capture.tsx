// Capturer une zone d'une page, dans le lecteur : le cadre se trace à la souris sur la page, et la
// barre propose de copier l'image ou le texte, ou de les enregistrer en image ou en PDF.
import { useRef, useState } from "react";
import { Button, Group, Paper, Select, Text } from "@mantine/core";
import { IconClipboardText, IconCopy, IconFileTypePdf, IconPhoto, IconScreenshot, IconX } from "@tabler/icons-react";
import type { FormatPage } from "../../coeur";
import { versLaPage, type Rotation } from "../../donnees/affichage";
import { assezGrande, tailleEnCm, zoneEntre, type ZonePoints } from "../../donnees/capture";
import { textes } from "../../textes";

const t = textes.lecteur.capture;

interface Point {
  x: number;
  y: number;
}

interface PropsCouche {
  page: number;
  format: FormatPage;
  echelle: number;
  rotation: Rotation;
  /** La zone tracée sur cette page (points de la page), s'il y en a une. */
  zone: ZonePoints | null;
  onZone: (page: number, zone: ZonePoints | null) => void;
  /** Faux : le cadre est seulement montré, on ne le trace pas. */
  actif?: boolean;
}

/**
 * Sur une page, en capture : le cadre se trace en glissant (un simple clic l'efface) ; autour de
 * lui, la page s'assombrit. La couche couvre la page : liens, texte et champs ne réagissent plus.
 */
export function CoucheCapture({ page, format, echelle, rotation, zone, onZone, actif = true }: PropsCouche) {
  const couche = useRef<HTMLDivElement>(null);
  const [trace, setTrace] = useState<{ debut: Point; courant: Point } | null>(null);

  const point = (evenement: { clientX: number; clientY: number }): Point => {
    // La boîte affichée de la couche (tournée), et sa taille droite : offsetWidth ignore la rotation.
    const element = couche.current!;
    const rect = element.getBoundingClientRect();
    const ici = versLaPage(evenement.clientX - rect.left, evenement.clientY - rect.top, element.offsetWidth, element.offsetHeight, rotation);
    return { x: ici.x / echelle, y: ici.y / echelle };
  };
  const montree = trace ? zoneEntre(trace.debut, trace.courant, format) : zone;

  return (
    <div
      ref={couche}
      className="couche-capture"
      data-couche-capture={page + 1}
      style={actif ? undefined : { cursor: "default" }}
      onPointerDown={(evenement) => {
        if (!actif || evenement.button !== 0) return;
        evenement.preventDefault();
        evenement.currentTarget.setPointerCapture(evenement.pointerId);
        const ici = point(evenement);
        setTrace({ debut: ici, courant: ici });
      }}
      onPointerMove={(evenement) => {
        if (trace) setTrace({ debut: trace.debut, courant: point(evenement) });
      }}
      onPointerUp={(evenement) => {
        if (!trace) return;
        const tracee = zoneEntre(trace.debut, point(evenement), format);
        setTrace(null);
        onZone(page, assezGrande(tracee) ? tracee : null);
      }}
      onPointerCancel={() => setTrace(null)}
    >
      {montree && (
        <div
          className="cadre-capture"
          data-cadre-capture
          style={{ left: montree.x * echelle, top: montree.y * echelle, width: montree.largeur * echelle, height: montree.hauteur * echelle }}
        />
      )}
    </div>
  );
}

export type QualiteCapture = "150" | "300" | "600";
export type TravailCapture = "image" | "fichier" | "pdf" | null;

interface PropsBarre {
  /** Le cadre tracé, et sa page (à partir de 0) ; rien tant qu'il n'est pas tracé. */
  capture: { page: number; zone: ZonePoints } | null;
  /** Nombre de mots dans le cadre (null : pas encore lu). */
  mots: number | null;
  qualite: QualiteCapture;
  onQualite: (qualite: QualiteCapture) => void;
  enCours: TravailCapture;
  onCopierImage: () => void;
  onCopierTexte: () => void;
  onEnregistrerImage: () => void;
  onEnregistrerPdf: () => void;
  onTerminer: () => void;
}

/** La barre de la capture : la consigne, puis, le cadre tracé, ce qu'on peut en faire. */
export function BarreCapture({ capture, mots, qualite, onQualite, enCours, onCopierImage, onCopierTexte, onEnregistrerImage, onEnregistrerPdf, onTerminer }: PropsBarre) {
  return (
    <Paper withBorder radius="lg" px="sm" py={8} className="barre-capture" data-barre-capture>
      <Group justify="space-between" gap="sm" wrap="nowrap" align="center">
        {capture ? (
          <Group gap="xs" wrap="wrap" align="center">
            <Text size="sm" fw={650} data-zone-capture>
              {t.zone(capture.page + 1, tailleEnCm(capture.zone))}
            </Text>
            <Button size="xs" leftSection={<IconCopy size={15} />} loading={enCours === "image"} onClick={onCopierImage} data-copier-image>
              {t.copierImage}
            </Button>
            <Button size="xs" variant="default" leftSection={<IconClipboardText size={15} />} disabled={!mots} onClick={onCopierTexte} data-copier-texte>
              {t.copierTexte}
            </Button>
            <Button size="xs" variant="default" leftSection={<IconPhoto size={15} />} loading={enCours === "fichier"} onClick={onEnregistrerImage} data-enregistrer-image>
              {t.enregistrerImage}
            </Button>
            <Button size="xs" variant="default" leftSection={<IconFileTypePdf size={15} />} loading={enCours === "pdf"} onClick={onEnregistrerPdf} data-enregistrer-pdf>
              {t.enregistrerPdf}
            </Button>
            <Select
              size="xs"
              w={170}
              aria-label={t.qualite}
              value={qualite}
              allowDeselect={false}
              onChange={(valeur) => valeur && onQualite(valeur as QualiteCapture)}
              data={(["150", "300", "600"] as const).map((value) => ({ value, label: `${t.qualites[value]} (${value} ppp)` }))}
            />
            {mots === 0 && (
              <Text size="xs" c="dimmed">
                {t.sansTexte}
              </Text>
            )}
          </Group>
        ) : (
          <Group gap="xs" wrap="nowrap" align="center">
            <IconScreenshot size={18} stroke={1.8} />
            <Text size="sm">{t.consigne}</Text>
          </Group>
        )}
        <Button size="xs" variant="subtle" color="gray" leftSection={<IconX size={15} />} onClick={onTerminer} style={{ flexShrink: 0 }} data-terminer-capture>
          {t.terminer}
        </Button>
      </Group>
    </Paper>
  );
}
