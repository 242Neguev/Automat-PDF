import { useRef, useState } from "react";
import { Badge, Box, Button, Group, Paper, SegmentedControl, Stack, Text } from "@mantine/core";
import { IconClearAll, IconFilePlus, IconPhoto, IconPhotoPlus } from "@tabler/icons-react";
import { closestCenter, DndContext, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { rectSortingStrategy, SortableContext, sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { open, save } from "@tauri-apps/plugin-dialog";
import { imagesEnPdf, type OptionsImages } from "../../coeur";
import { aCote, nomDe } from "../../donnees/chemins";
import { textes } from "../../textes";
import Action from "../commun/Action";
import { useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { ZoneDepot } from "../commun/ZoneFichier";
import CartePage from "../organiser/CartePage";
import { deplacer, pivoter } from "../organiser/pages";
import "../organiser/organiser.css";

const t = textes.imagesPdf;
const COTE = 150;
const EXTENSIONS = ["jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "gif"];

interface ImageTravail {
  id: string;
  chemin: string;
  nom: string;
  rotation: number;
}

/** Images en PDF : photos et scans réunis en un PDF, une image par page, dans l'ordre choisi. */
export default function ImagesEnPdf() {
  const [images, setImages] = useState<ImageTravail[]>([]);
  const [format, setFormat] = useState<OptionsImages["format"]>("a4");
  const [marge, setMarge] = useState("10");
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
  const compteur = useRef(0);
  const largeur = Math.round(COTE * Math.min(2, window.devicePixelRatio || 1));
  const capteurs = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  async function ajouter() {
    const choisies = await open({ multiple: true, directory: false, filters: [{ name: t.filtre, extensions: EXTENSIONS }] });
    if (choisies) ajouterChemins(Array.isArray(choisies) ? choisies : [choisies]);
  }

  function ajouterChemins(chemins: string[]) {
    const nouvelles = chemins.map((chemin) => ({
      id: `i${++compteur.current}`,
      chemin,
      nom: nomDe(chemin),
      rotation: 0,
    }));
    setImages((actuelles) => [...actuelles, ...nouvelles]);
  }
  useFichiersRecus("images", ajouterChemins);

  function finDeplacement({ active, over }: DragEndEvent) {
    if (over) setImages((actuelles) => deplacer(actuelles, [String(active.id)], String(active.id), String(over.id)));
  }

  async function creer() {
    const sortie = await save({ defaultPath: aCote(images[0]?.chemin, t.nomFichier), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    setCree(null);
    try {
      await imagesEnPdf(
        images.map(({ chemin, rotation }) => ({ chemin, rotation })),
        { format, margeMm: Number(marge) },
        sortie,
      );
      setCree({ titre: t.cree, chemin: sortie, details: [textes.pdf.pages(images.length)] });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  if (images.length === 0)
    return (
      <Stack gap="md" maw={760}>
        <ZoneDepot icone={IconPhoto} invite={t.deposer} detail={t.deposerDetail} bouton={t.ajouter} onChoisir={() => void ajouter()} />
        <Resultat cree={cree} onFermer={() => setCree(null)} />
      </Stack>
    );

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Group gap="xs">
          <Button leftSection={<IconFilePlus size={18} />} variant="light" onClick={() => void ajouter()}>
            {t.ajouter}
          </Button>
          <Button variant="subtle" color="gray" leftSection={<IconClearAll size={18} />} onClick={() => setImages([])}>
            {t.vider}
          </Button>
        </Group>
        <Action icone={IconPhotoPlus} enCours={enCours} onClick={() => void creer()}>
          {t.creer}
        </Action>
      </Group>

      <Resultat cree={cree} onFermer={() => setCree(null)} />

      <Paper withBorder p="md">
        <Group gap="xl" align="flex-start">
          <Stack gap={4}>
            <Text size="sm" fw={500}>
              {t.format}
            </Text>
            <SegmentedControl value={format} onChange={(valeur) => setFormat(valeur as OptionsImages["format"])} data={(["a4", "ajustee"] as const).map((value) => ({ value, label: t.formats[value] }))} />
            <Text size="sm" c="dimmed" maw={360}>
              {t.formatsAide[format]}
            </Text>
          </Stack>
          {format === "a4" && (
            <Stack gap={4}>
              <Text size="sm" fw={500}>
                {t.marge}
              </Text>
              <SegmentedControl value={marge} onChange={setMarge} data={(["0", "10", "20"] as const).map((value) => ({ value, label: t.marges[value] }))} />
            </Stack>
          )}
        </Group>
      </Paper>

      <Group gap="xs">
        <Badge variant="light" size="lg">
          {t.images(images.length)}
        </Badge>
        <Text size="sm" c="dimmed">
          {t.aide}
        </Text>
      </Group>
      <DndContext sensors={capteurs} collisionDetection={closestCenter} onDragEnd={finDeplacement}>
        <SortableContext items={images.map((image) => image.id)} strategy={rectSortingStrategy}>
          <Box style={{ display: "grid", gridTemplateColumns: `repeat(auto-fill, minmax(${COTE + 14}px, 1fr))`, gap: 12 }} data-images-pdf>
            {images.map((image, index) => (
              <CartePage
                key={image.id}
                element={image}
                numero={index + 1}
                chemin={image.chemin}
                legende={image.nom}
                description={t.origine(image.nom)}
                couleur="eau"
                cote={COTE}
                largeur={largeur}
                choisie={false}
                onClic={() => undefined}
                onPivoter={(sens) => setImages((actuelles) => pivoter(actuelles, new Set([image.id]), sens))}
                onSupprimer={() => setImages((actuelles) => actuelles.filter((autre) => autre.id !== image.id))}
              />
            ))}
          </Box>
        </SortableContext>
      </DndContext>
    </Stack>
  );
}
