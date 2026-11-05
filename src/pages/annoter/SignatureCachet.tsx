import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { ActionIcon, Box, Button, Group, Loader, Paper, SegmentedControl, Stack, Text, TextInput, Title, Tooltip } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCheck, IconChevronLeft, IconChevronRight, IconFileSearch, IconPhotoSearch, IconSignature } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { apposerImage, lireImage, pagesPdf, type Placement } from "../../coeur";
import { pagesSelon, type ChoixPages } from "../../donnees/plages";
import { textes } from "../../textes";
import { dela, useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";
import Vignette from "../commun/Vignette";
import { imagePage } from "../commun/vignettes";

const t = textes.signature;
const LARGEUR = 520;
const EXTENSIONS = ["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"];
const TYPES: Record<string, string> = { png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", webp: "image/webp", gif: "image/gif", bmp: "image/bmp" };

interface ImageSignature {
  chemin: string;
  nom: string;
  adresse: string;
  /** Largeur sur hauteur. */
  proportion: number;
}

const borner = (valeur: number, min: number, max: number) => Math.min(max, Math.max(min, valeur));

/** La page, avec l'image à placer : glisser pour la déplacer, poignée pour la taille, flèches du clavier. */
function PlacementImage({
  chemin,
  page,
  image,
  placement,
  onPlacement,
}: {
  chemin: string;
  page: number;
  image: ImageSignature;
  placement: Placement;
  onPlacement: (placement: Placement) => void;
}) {
  const [fond, setFond] = useState<string | null>(null);
  const [hauteur, setHauteur] = useState<number | null>(null);
  const geste = useRef<{ type: "deplacer" | "taille"; x: number; y: number; depart: Placement } | null>(null);

  useEffect(() => {
    let actuel = true;
    setFond(null);
    imagePage(chemin, page, LARGEUR * 2).then((adresse) => actuel && setFond(adresse), signalerErreur);
    return () => {
      actuel = false;
    };
  }, [chemin, page]);

  // L'emplacement garde les proportions de l'image sur cette page.
  useEffect(() => {
    if (!hauteur) return;
    const voulue = (placement.largeur * LARGEUR) / image.proportion / hauteur;
    if (Math.abs(voulue - placement.hauteur) > 1e-4) onPlacement({ ...placement, hauteur: voulue, y: borner(placement.y, 0, Math.max(0, 1 - voulue)) });
  }, [hauteur, image.proportion, placement, onPlacement]);

  function debut(type: "deplacer" | "taille", evenement: PointerEvent) {
    evenement.preventDefault();
    evenement.stopPropagation();
    (evenement.currentTarget as Element).setPointerCapture(evenement.pointerId);
    geste.current = { type, x: evenement.clientX, y: evenement.clientY, depart: placement };
  }

  function mouvement(evenement: PointerEvent) {
    const enCours = geste.current;
    if (!enCours || !hauteur) return;
    const dx = (evenement.clientX - enCours.x) / LARGEUR;
    const dy = (evenement.clientY - enCours.y) / hauteur;
    const { depart } = enCours;
    if (enCours.type === "deplacer") {
      onPlacement({ ...depart, x: borner(depart.x + dx, 0, 1 - depart.largeur), y: borner(depart.y + dy, 0, 1 - depart.hauteur) });
    } else {
      const largeur = borner(depart.largeur + dx, 0.03, 1 - depart.x);
      const hauteurImage = (largeur * LARGEUR) / image.proportion / hauteur;
      if (depart.y + hauteurImage <= 1) onPlacement({ ...depart, largeur, hauteur: hauteurImage });
    }
  }

  function clavier(evenement: KeyboardEvent) {
    const pas = evenement.shiftKey ? 0.02 : 0.005;
    const deplacements: Record<string, [number, number]> = { ArrowLeft: [-pas, 0], ArrowRight: [pas, 0], ArrowUp: [0, -pas], ArrowDown: [0, pas] };
    const deplacement = deplacements[evenement.key];
    if (!deplacement) return;
    evenement.preventDefault();
    onPlacement({
      ...placement,
      x: borner(placement.x + deplacement[0], 0, 1 - placement.largeur),
      y: borner(placement.y + deplacement[1], 0, 1 - placement.hauteur),
    });
  }

  return (
    <Box pos="relative" w={LARGEUR} h={hauteur ?? LARGEUR * 1.414} style={{ boxShadow: "0 1px 4px rgba(34, 37, 42, 0.25)", background: "white", flexShrink: 0 }}>
      {fond ? (
        <img
          src={fond}
          width={LARGEUR}
          alt=""
          draggable={false}
          onLoad={(e) => setHauteur((LARGEUR * e.currentTarget.naturalHeight) / e.currentTarget.naturalWidth)}
          style={{ display: "block" }}
        />
      ) : (
        <Loader size="sm" color="gray" pos="absolute" top="45%" left="47%" />
      )}
      {hauteur && (
        <Box
          data-placement-image
          tabIndex={0}
          aria-label={t.placer}
          pos="absolute"
          left={placement.x * LARGEUR}
          top={placement.y * hauteur}
          w={placement.largeur * LARGEUR}
          h={placement.hauteur * hauteur}
          onPointerDown={(e) => debut("deplacer", e)}
          onPointerMove={mouvement}
          onPointerUp={() => (geste.current = null)}
          onKeyDown={clavier}
          style={{ outline: "1.5px dashed var(--mantine-color-eau-6)", cursor: "move", touchAction: "none" }}
        >
          <img src={image.adresse} alt={image.nom} draggable={false} style={{ width: "100%", height: "100%", objectFit: "contain", pointerEvents: "none" }} />
          <Box
            data-poignee
            aria-label={t.redimensionner}
            pos="absolute"
            right={-7}
            bottom={-7}
            w={14}
            h={14}
            onPointerDown={(e) => debut("taille", e)}
            onPointerMove={mouvement}
            onPointerUp={() => (geste.current = null)}
            style={{ background: "var(--mantine-color-eau-6)", border: "2px solid white", borderRadius: 3, cursor: "nwse-resize" }}
          />
        </Box>
      )}
    </Box>
  );
}

/** Signature ou cachet : une image posée à l'endroit choisi, sur une page ou sur plusieurs. */
export default function SignatureCachet() {
  const [pdf, setPdf] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [image, setImage] = useState<ImageSignature | null>(null);
  const [page, setPage] = useState(0);
  const [placement, setPlacement] = useState<Placement>({ x: 0.62, y: 0.78, largeur: 0.28, hauteur: 0.1 });
  const [choix, setChoix] = useState<ChoixPages>("cette");
  const [plages, setPlages] = useState("1");
  const [enCours, setEnCours] = useState(false);
  const selection = pdf ? pagesSelon(choix, plages, pdf.pages, page) : null;

  async function choisirPdf() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrirPdf(chemin);
  }

  async function ouvrirPdf(chemin: string) {
    try {
      const pages = (await pagesPdf(chemin)).length;
      setPdf({ chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin, pages });
      setPage(pages - 1);
    } catch (e) {
      signalerErreur(e);
    }
  }

  async function choisirImage() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: t.filtre, extensions: EXTENSIONS }] });
    if (typeof chemin === "string") await ouvrirImage(chemin);
  }

  // Le PDF à signer et l'image de la signature (ou du cachet) peuvent arriver ensemble.
  useFichiersRecus("signature", (fichiers) => {
    const [document] = dela(fichiers, "pdf");
    const [signature] = dela(fichiers, "image");
    if (document) void ouvrirPdf(document);
    if (signature) void ouvrirImage(signature);
  });

  async function ouvrirImage(chemin: string) {
    try {
      const extension = chemin.split(".").pop()?.toLowerCase() ?? "png";
      const adresse = URL.createObjectURL(new Blob([await lireImage(chemin)], { type: TYPES[extension] ?? "image/png" }));
      const proportion = await new Promise<number>((resoudre, rejeter) => {
        const essai = new Image();
        essai.onload = () => resoudre(essai.naturalWidth / Math.max(1, essai.naturalHeight));
        essai.onerror = () => rejeter(new Error(`« ${chemin} » n'a pas pu être affichée.`));
        essai.src = adresse;
      });
      setImage((precedente) => {
        if (precedente) URL.revokeObjectURL(precedente.adresse);
        return { chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin, adresse, proportion };
      });
    } catch (e) {
      signalerErreur(e);
    }
  }

  async function creer() {
    if (!pdf || !image || !selection || "erreur" in selection) return;
    const sortie = await save({ defaultPath: t.nomFichier(pdf.nom.replace(/\.pdf$/i, "")), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    try {
      await apposerImage(pdf.chemin, image.chemin, placement, selection.pages, sortie);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.cree, message: sortie });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md">
      <Group>
        <Button variant={pdf ? "light" : "filled"} leftSection={<IconFileSearch size={18} />} onClick={choisirPdf}>
          {pdf ? t.autrePdf : t.choisirPdf}
        </Button>
        {pdf && (
          <Text size="sm">
            {pdf.nom} · {textes.pdf.pages(pdf.pages)}
          </Text>
        )}
      </Group>

      {pdf && (
        <Group align="flex-start" gap="lg" wrap="nowrap">
          <Stack gap="md" maw={420} style={{ flex: 1 }}>
            <Paper withBorder p="md">
              <Stack gap="xs">
                <Button variant={image ? "light" : "filled"} leftSection={<IconPhotoSearch size={18} />} onClick={choisirImage} w="fit-content">
                  {image ? t.autreImage : t.choisirImage}
                </Button>
                <Text size="xs" c="dimmed">
                  {image ? image.nom : t.imageAide}
                </Text>
                {image && (
                  <Text size="xs" c="dimmed">
                    {t.placer}
                  </Text>
                )}
              </Stack>
            </Paper>
            <Paper withBorder p="md">
              <Stack gap="xs">
                <Title order={5}>{t.pages}</Title>
                <SegmentedControl value={choix} onChange={(v) => setChoix(v as ChoixPages)} data={(["cette", "toutes", "derniere", "certaines"] as const).map((value) => ({ value, label: t.choixPages[value] }))} />
                {choix === "certaines" && (
                  <TextInput label={t.plages} description={t.plagesAide} value={plages} onChange={(e) => setPlages(e.currentTarget.value)} error={selection && "erreur" in selection ? selection.erreur : undefined} />
                )}
              </Stack>
            </Paper>
            <Button leftSection={<IconSignature size={18} />} loading={enCours} disabled={!image || !selection || "erreur" in selection} onClick={creer} w="fit-content">
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
                {t.page(page + 1, pdf.pages)}
              </Text>
              <Tooltip label={t.suivante}>
                <ActionIcon variant="default" aria-label={t.suivante} disabled={page >= pdf.pages - 1} onClick={() => setPage((p) => p + 1)}>
                  <IconChevronRight size={16} />
                </ActionIcon>
              </Tooltip>
            </Group>
            {image ? (
              <PlacementImage chemin={pdf.chemin} page={page} image={image} placement={placement} onPlacement={setPlacement} />
            ) : (
              <Vignette chemin={pdf.chemin} page={page} taille={LARGEUR} largeur={LARGEUR * 2} alt={t.page(page + 1, pdf.pages)} />
            )}
          </Stack>
        </Group>
      )}
    </Stack>
  );
}
