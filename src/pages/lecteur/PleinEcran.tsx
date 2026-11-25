// Plein écran, comme le mode plein écran de Reader : une page à la fois, à la taille de l'écran, sur
// fond noir. Flèches, Espace ou un clic pour avancer ; Échap pour revenir au lecteur, à la page vue.
// Les annotations en cours s'affichent aussi, comme dans le lecteur.
import { useEffect, useRef, useState } from "react";
import { ActionIcon, Portal, Text, Tooltip } from "@mantine/core";
import { IconChevronLeft, IconChevronRight, IconX } from "@tabler/icons-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { FormatPage } from "../../coeur";
import { tailleAffichee, transformation, type Rotation } from "../../donnees/affichage";
import type { Annotation } from "../../donnees/annotations";
import { textes } from "../../textes";
import { imagePage } from "../commun/vignettes";
import { CoucheAnnotations, type ImageChargee, type ReglagesCommentaires } from "./Annotations";

const t = textes.lecteur.pleinEcran;

/** Réglages sans effet : la couche d'annotations ne fait que montrer. */
const REGLAGES: ReglagesCommentaires = { teinteTrait: "rouge", forme: "rectangle", epaisseur: 2, tampon: "approuve", avecDate: false };
const rien = () => undefined;

/** Les commandes se cachent après ce délai sans mouvement de la souris. */
const DELAI_COMMANDES = 2500;

/** Largeur d'image à demander au cœur : celle de l'écran, par paliers de 200 pixels (2 400 au plus). */
const largeurDemandee = (pixels: number) => Math.min(2400, Math.max(200, Math.ceil((pixels * (window.devicePixelRatio || 1)) / 200) * 200));

interface Props {
  chemin: string;
  formats: FormatPage[];
  /** La page par laquelle commencer. */
  depart: number;
  rotation: Rotation;
  annotations: Annotation[];
  images: Record<string, ImageChargee>;
  /** Retour au lecteur, à la dernière page vue. */
  onFermer: (page: number) => void;
}

export default function PleinEcran({ chemin, formats, depart, rotation, annotations, images, onFermer }: Props) {
  const [page, setPage] = useState(depart);
  const [ecran, setEcran] = useState({ largeur: window.innerWidth, hauteur: window.innerHeight });
  const [image, setImage] = useState<{ page: number; adresse: string } | null>(null);
  const [commandes, setCommandes] = useState(true);
  const minuterie = useRef<number | undefined>(undefined);
  const derniereRoulette = useRef(0);
  const page_ = useRef(page);
  page_.current = page;

  const changer = (pas: number) => setPage((avant) => Math.min(formats.length - 1, Math.max(0, avant + pas)));
  const fermer = () => onFermer(page_.current);

  // La fenêtre passe en plein écran (sauf si elle l'était déjà), et le quitte au retour.
  useEffect(() => {
    let fenetre: ReturnType<typeof getCurrentWindow> | null = null;
    let dejaPleinEcran = false;
    try {
      fenetre = getCurrentWindow();
    } catch {
      fenetre = null;
    }
    void (async () => {
      try {
        dejaPleinEcran = (await fenetre?.isFullscreen()) ?? false;
        if (fenetre && !dejaPleinEcran) await fenetre.setFullscreen(true);
      } catch {
        // hors de l'application (banc d'essai) : la page couvre la fenêtre, c'est tout
      }
    })();
    return () => {
      if (fenetre && !dejaPleinEcran) void fenetre.setFullscreen(false).catch(() => undefined);
    };
  }, []);

  useEffect(() => {
    const mesurer = () => setEcran({ largeur: window.innerWidth, hauteur: window.innerHeight });
    window.addEventListener("resize", mesurer);
    return () => window.removeEventListener("resize", mesurer);
  }, []);

  // Les commandes se montrent au début, puis à chaque mouvement de la souris.
  function montrerCommandes() {
    setCommandes(true);
    window.clearTimeout(minuterie.current);
    minuterie.current = window.setTimeout(() => setCommandes(false), DELAI_COMMANDES);
  }
  useEffect(() => {
    montrerCommandes();
    return () => window.clearTimeout(minuterie.current);
  }, []);

  // Clavier : avancer, reculer, aller au début ou à la fin, quitter.
  useEffect(() => {
    const touche = (evenement: KeyboardEvent) => {
      const { key } = evenement;
      if (["ArrowRight", "ArrowDown", "PageDown", " ", "Enter"].includes(key)) changer(1);
      else if (["ArrowLeft", "ArrowUp", "PageUp", "Backspace"].includes(key)) changer(-1);
      else if (key === "Home") setPage(0);
      else if (key === "End") setPage(formats.length - 1);
      else if (key === "Escape" || ((evenement.metaKey || evenement.ctrlKey) && key.toLowerCase() === "l")) fermer();
      else return;
      evenement.preventDefault();
      evenement.stopPropagation();
    };
    window.addEventListener("keydown", touche, true);
    return () => window.removeEventListener("keydown", touche, true);
  });

  // La page à la taille de l'écran, sans la déformer.
  const format = formats[page];
  const affichee = tailleAffichee(format, rotation);
  const echelle = Math.min(ecran.largeur / affichee.largeur, ecran.hauteur / affichee.hauteur);
  const [largeur, hauteur] = [format.largeur * echelle, format.hauteur * echelle];
  const demandee = largeurDemandee(largeur);

  useEffect(() => {
    let actuelle = true;
    imagePage(chemin, page, demandee).then(
      (adresse) => actuelle && setImage({ page, adresse }),
      () => undefined,
    );
    // La suivante se prépare pendant qu'on lit celle-ci.
    if (page + 1 < formats.length) {
      const suivante = formats[page + 1];
      const echelleSuivante = Math.min(ecran.largeur / tailleAffichee(suivante, rotation).largeur, ecran.hauteur / tailleAffichee(suivante, rotation).hauteur);
      void imagePage(chemin, page + 1, largeurDemandee(suivante.largeur * echelleSuivante)).catch(() => undefined);
    }
    return () => {
      actuelle = false;
    };
  }, [chemin, page, demandee, formats, ecran, rotation]);

  return (
    <Portal>
      <div
        className="plein-ecran"
        onMouseMove={montrerCommandes}
        onClick={() => changer(1)}
        onWheel={(evenement) => {
          // Un cran de roulette, une page : pas une page par événement d'un pavé tactile.
          const maintenant = Date.now();
          if (Math.abs(evenement.deltaY) < 4 || maintenant - derniereRoulette.current < 350) return;
          derniereRoulette.current = maintenant;
          changer(evenement.deltaY > 0 ? 1 : -1);
        }}
        data-plein-ecran-ouvert
        data-commandes={commandes || undefined}
      >
        <div className="plein-ecran__page" style={{ width: affichee.largeur * echelle, height: affichee.hauteur * echelle }} data-page-plein-ecran={page + 1}>
          <div className="page-droite" style={{ width: largeur, height: hauteur, transform: transformation(rotation, largeur, hauteur) }}>
            {image?.page === page && <img src={image.adresse} alt={textes.lecteur.alt(page + 1)} draggable={false} />}
            {annotations.some((a) => a.page === page) && (
              <CoucheAnnotations
                page={page}
                echelle={echelle}
                annotations={annotations.filter((a) => a.page === page)}
                mots={null}
                actif={false}
                outil="selection"
                teinteSurligneur="jaune"
                teinteTexte="noir"
                taille={12}
                reglages={REGLAGES}
                images={images}
                selection={null}
                onSelection={rien}
                onAjouter={rien}
                onModifier={rien}
                onSupprimer={rien}
                onEffacer={rien}
              />
            )}
          </div>
        </div>
        <div className="plein-ecran__commandes" onClick={(evenement) => evenement.stopPropagation()}>
          <Tooltip label={t.precedente}>
            <ActionIcon variant="subtle" color="gray" size={36} radius="xl" onClick={() => changer(-1)} disabled={page === 0} aria-label={t.precedente}>
              <IconChevronLeft size={20} />
            </ActionIcon>
          </Tooltip>
          <Text size="sm" fw={600} c="white" miw={64} ta="center" data-page-sur>
            {t.pageSur(page + 1, formats.length)}
          </Text>
          <Tooltip label={t.suivante}>
            <ActionIcon variant="subtle" color="gray" size={36} radius="xl" onClick={() => changer(1)} disabled={page === formats.length - 1} aria-label={t.suivante}>
              <IconChevronRight size={20} />
            </ActionIcon>
          </Tooltip>
          <Tooltip label={t.quitter}>
            <ActionIcon variant="subtle" color="gray" size={36} radius="xl" onClick={fermer} aria-label={t.quitter} data-quitter-plein-ecran>
              <IconX size={20} />
            </ActionIcon>
          </Tooltip>
          <Text size="xs" c="gray.5" pl={6} pr={8} visibleFrom="sm">
            {t.aide}
          </Text>
        </div>
      </div>
    </Portal>
  );
}
