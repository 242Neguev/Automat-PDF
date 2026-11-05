import { useRef, useState } from "react";
import { Badge, Box, Button, Group, Paper, SegmentedControl, Stack, Text } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCheck, IconClearAll, IconFilePlus, IconPhotoPlus } from "@tabler/icons-react";
import { closestCenter, DndContext, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { rectSortingStrategy, SortableContext, sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { open, save } from "@tauri-apps/plugin-dialog";
import { imagesEnPdf, type OptionsImages } from "../../coeur";
import { textes } from "../../textes";
import { useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";
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
      nom: chemin.split(/[\\/]/).pop() ?? chemin,
      rotation: 0,
    }));
    setImages((actuelles) => [...actuelles, ...nouvelles]);
  }
  useFichiersRecus("images", ajouterChemins);

  function finDeplacement({ active, over }: DragEndEvent) {
    if (over) setImages((actuelles) => deplacer(actuelles, [String(active.id)], String(active.id), String(over.id)));
  }

  async function creer() {
    const sortie = await save({ defaultPath: t.nomFichier, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    try {
      await imagesEnPdf(
        images.map(({ chemin, rotation }) => ({ chemin, rotation })),
        { format, margeMm: Number(marge) },
        sortie,
      );
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.cree, message: sortie });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <Group gap="xs">
          <Button leftSection={<IconFilePlus size={18} />} variant={images.length > 0 ? "light" : "filled"} onClick={ajouter}>
            {t.ajouter}
          </Button>
          <Button variant="subtle" color="gray" leftSection={<IconClearAll size={18} />} disabled={images.length === 0} onClick={() => setImages([])}>
            {t.vider}
          </Button>
        </Group>
        <Button leftSection={<IconPhotoPlus size={18} />} disabled={images.length === 0} loading={enCours} onClick={creer}>
          {t.creer}
        </Button>
      </Group>

      <Paper withBorder p="md">
        <Group gap="xl" align="flex-start">
          <Stack gap={4}>
            <Text size="sm" fw={500}>
              {t.format}
            </Text>
            <SegmentedControl value={format} onChange={(valeur) => setFormat(valeur as OptionsImages["format"])} data={(["a4", "ajustee"] as const).map((value) => ({ value, label: t.formats[value] }))} />
            <Text size="xs" c="dimmed" maw={340}>
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

      {images.length === 0 ? (
        <Paper withBorder p="xl" ta="center">
          <Text c="dimmed">{t.vide}</Text>
        </Paper>
      ) : (
        <>
          <Group gap="xs">
            <Badge variant="light" size="lg">
              {t.images(images.length)}
            </Badge>
            <Text size="xs" c="dimmed">
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
        </>
      )}
    </Stack>
  );
}
