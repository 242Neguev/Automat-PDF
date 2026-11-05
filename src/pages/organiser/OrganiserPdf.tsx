import { useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import { Badge, Box, Button, Group, Paper, SegmentedControl, Stack, Text } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import {
  IconArrowBackUp,
  IconCheck,
  IconClearAll,
  IconDeviceFloppy,
  IconFileExport,
  IconFilePlus,
  IconRotate,
  IconRotateClockwise,
  IconSelectAll,
  IconTrash,
} from "@tabler/icons-react";
import { closestCenter, DndContext, DragOverlay, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent, type DragStartEvent } from "@dnd-kit/core";
import { rectSortingStrategy, SortableContext, sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { open, save } from "@tauri-apps/plugin-dialog";
import { enregistrerPages, pagesPdf } from "../../coeur";
import { textes } from "../../textes";
import { useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";
import Vignette from "../commun/Vignette";
import { oublier } from "../commun/vignettes";
import ApercuPage from "./ApercuPage";
import CartePage from "./CartePage";
import { deplacer, pivoter, plage, type PageTravail } from "./pages";
import "./organiser.css";

const t = textes.organisation;
const TAILLES = { petite: 110, moyenne: 160, grande: 230 } as const;
type Taille = keyof typeof TAILLES;
/** Une couleur par PDF ajouté, pour reconnaître d'où vient chaque page. */
const COULEURS = ["eau", "orange", "grape", "indigo", "pink", "lime", "cyan", "red"];
const RETOURS_MAX = 50;

interface Source {
  chemin: string;
  nom: string;
}

const nomDe = (chemin: string) => chemin.split(/[\\/]/).pop() ?? chemin;

/** Organiser : les pages d'un ou de plusieurs PDF, à remettre dans l'ordre, pivoter, retirer, extraire. */
export default function OrganiserPdf() {
  const [sources, setSources] = useState<Source[]>([]);
  const [pages, setPages] = useState<PageTravail[]>([]);
  const [selection, setSelection] = useState<Set<string>>(new Set());
  const [ancre, setAncre] = useState<string | null>(null);
  const [retours, setRetours] = useState<PageTravail[][]>([]);
  const [taille, setTaille] = useState<Taille>("moyenne");
  const [tirees, setTirees] = useState<{ actif: string; groupe: string[] } | null>(null);
  const [apercu, setApercu] = useState<number | null>(null);
  const [enCours, setEnCours] = useState(false);
  const compteur = useRef(0);
  const cote = TAILLES[taille];
  const largeur = Math.round(cote * Math.min(2, window.devicePixelRatio || 1));
  const capteurs = useSensors(
    // Un clic sélectionne ; il faut tirer de quelques pixels pour déplacer.
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const choisies = pages.filter((page) => selection.has(page.id));

  /** Nouvel état des pages, que « Annuler » sait défaire. */
  function modifier(nouvelles: PageTravail[]) {
    if (nouvelles === pages) return;
    setRetours((precedents) => [...precedents.slice(-(RETOURS_MAX - 1)), pages]);
    setPages(nouvelles);
  }

  function annuler() {
    const precedent = retours[retours.length - 1];
    if (!precedent) return;
    setRetours(retours.slice(0, -1));
    setPages(precedent);
  }

  async function ajouter() {
    const choisis = await open({ multiple: true, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (choisis) await ajouterChemins(Array.isArray(choisis) ? choisis : [choisis]);
  }

  async function ajouterChemins(chemins: string[]) {
    const nouvellesSources = [...sources];
    const nouvelles: PageTravail[] = [];
    for (const chemin of chemins) {
      try {
        const formats = await pagesPdf(chemin);
        let source = nouvellesSources.findIndex((s) => s.chemin === chemin);
        if (source < 0) source = nouvellesSources.push({ chemin, nom: nomDe(chemin) }) - 1;
        formats.forEach((_, page) => nouvelles.push({ id: `p${++compteur.current}`, source, page, rotation: 0 }));
      } catch (e) {
        signalerErreur(e);
      }
    }
    setSources(nouvellesSources);
    if (nouvelles.length > 0) modifier([...pages, ...nouvelles]);
  }

  useFichiersRecus("organiser", (fichiers) => void ajouterChemins(fichiers));

  function cliquer(id: string, evenement: MouseEvent) {
    if (evenement.shiftKey && ancre) {
      setSelection(new Set(plage(pages, ancre, id)));
      return;
    }
    if (evenement.ctrlKey || evenement.metaKey) {
      const nouvelle = new Set(selection);
      if (nouvelle.has(id)) nouvelle.delete(id);
      else nouvelle.add(id);
      setSelection(nouvelle);
    } else {
      setSelection(new Set([id]));
    }
    setAncre(id);
  }

  const supprimer = (ids: Set<string>) => {
    modifier(pages.filter((page) => !ids.has(page.id)));
    setSelection(new Set());
  };

  function debutDeplacement({ active }: DragStartEvent) {
    const id = String(active.id);
    if (!selection.has(id)) {
      setSelection(new Set([id]));
      setAncre(id);
    }
    const groupe = selection.has(id) ? pages.filter((page) => selection.has(page.id)).map((page) => page.id) : [id];
    setTirees({ actif: id, groupe });
  }

  function finDeplacement({ active, over }: DragEndEvent) {
    if (over && tirees) modifier(deplacer(pages, tirees.groupe, String(active.id), String(over.id)));
    setTirees(null);
  }

  async function enregistrer(lesquelles: PageTravail[], nom: string) {
    const sortie = await save({ defaultPath: nom, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    try {
      await enregistrerPages(
        lesquelles.map((page) => ({ chemin: sources[page.source].chemin, page: page.page, rotation: page.rotation })),
        sortie,
      );
      oublier(sortie);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.enregistre, message: sortie });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  function clavier(evenement: KeyboardEvent) {
    const commande = evenement.ctrlKey || evenement.metaKey;
    if ((evenement.key === "Delete" || evenement.key === "Backspace") && selection.size > 0) supprimer(selection);
    else if (commande && evenement.key.toLowerCase() === "a") setSelection(new Set(pages.map((page) => page.id)));
    else if (commande && evenement.key.toLowerCase() === "z") annuler();
    else if (evenement.key === "Escape") setSelection(new Set());
    else return;
    evenement.preventDefault();
  }

  // Pendant un déplacement de groupe, les pages qui suivent la page tirée sont masquées.
  const visibles = tirees ? pages.filter((page) => !tirees.groupe.includes(page.id) || page.id === tirees.actif) : pages;
  const pageTiree = tirees && pages.find((page) => page.id === tirees.actif);

  return (
    <Stack gap="md" onKeyDown={clavier} tabIndex={-1} style={{ outline: "none" }} data-organiser>
      <Group justify="space-between">
        <Group gap="xs">
          <Button leftSection={<IconFilePlus size={18} />} variant={pages.length > 0 ? "light" : "filled"} onClick={ajouter}>
            {t.ajouter}
          </Button>
          <Button variant="default" leftSection={<IconArrowBackUp size={18} />} disabled={retours.length === 0} onClick={annuler}>
            {t.annuler}
          </Button>
          <Button
            variant="subtle"
            color="gray"
            leftSection={<IconClearAll size={18} />}
            disabled={pages.length === 0}
            onClick={() => {
              modifier([]);
              setSelection(new Set());
            }}
          >
            {t.vider}
          </Button>
        </Group>
        <Group gap="sm">
          <SegmentedControl
            size="xs"
            aria-label={t.taille}
            value={taille}
            onChange={(valeur) => setTaille(valeur as Taille)}
            data={(Object.keys(TAILLES) as Taille[]).map((value) => ({ value, label: t.tailles[value] }))}
          />
          <Button leftSection={<IconDeviceFloppy size={18} />} disabled={pages.length === 0} loading={enCours} onClick={() => enregistrer(pages, t.nomFichier)}>
            {t.enregistrer}
          </Button>
        </Group>
      </Group>

      {pages.length === 0 ? (
        <Paper withBorder p="xl" ta="center">
          <Text c="dimmed">{t.vide}</Text>
        </Paper>
      ) : (
        <>
          <Paper withBorder p="sm" data-barre-organiser>
            <Group gap="xs">
                <Badge variant="light" size="lg">
                  {t.pages(pages.length)}
                </Badge>
                {selection.size > 0 && (
                  <Badge variant="outline" size="lg">
                    {t.selection(selection.size)}
                  </Badge>
                )}
                <Button size="xs" variant="default" leftSection={<IconRotate size={16} />} disabled={selection.size === 0} onClick={() => modifier(pivoter(pages, selection, -1))}>
                  {t.pivoterGauche}
                </Button>
                <Button size="xs" variant="default" leftSection={<IconRotateClockwise size={16} />} disabled={selection.size === 0} onClick={() => modifier(pivoter(pages, selection, 1))}>
                  {t.pivoterDroite}
                </Button>
                <Button size="xs" variant="default" color="red" leftSection={<IconTrash size={16} />} disabled={selection.size === 0} onClick={() => supprimer(selection)}>
                  {t.supprimer}
                </Button>
                <Button size="xs" variant="default" leftSection={<IconFileExport size={16} />} disabled={selection.size === 0} loading={enCours} onClick={() => enregistrer(choisies, t.nomExtrait)}>
                  {t.extraire}
                </Button>
                <Button
                  size="xs"
                  variant="subtle"
                  leftSection={<IconSelectAll size={16} />}
                  onClick={() => setSelection(selection.size === pages.length ? new Set() : new Set(pages.map((page) => page.id)))}
                >
                  {selection.size === pages.length ? t.deselectionner : t.toutSelectionner}
                </Button>
            </Group>
          </Paper>
          <Text size="xs" c="dimmed">
            {t.aide}
          </Text>

          <DndContext
            sensors={capteurs}
            collisionDetection={closestCenter}
            onDragStart={debutDeplacement}
            onDragEnd={finDeplacement}
            onDragCancel={() => setTirees(null)}
          >
            <SortableContext items={visibles.map((page) => page.id)} strategy={rectSortingStrategy}>
              <Box style={{ display: "grid", gridTemplateColumns: `repeat(auto-fill, minmax(${cote + 14}px, 1fr))`, gap: 12 }}>
                {visibles.map((page) => {
                  const source = sources[page.source];
                  const numero = pages.indexOf(page) + 1;
                  return (
                    <CartePage
                      key={page.id}
                      element={page}
                      numero={numero}
                      chemin={source.chemin}
                      page={page.page}
                      legende={`${source.nom.replace(/\.pdf$/i, "")} · ${page.page + 1}`}
                      description={t.origine(source.nom, page.page + 1)}
                      couleur={COULEURS[page.source % COULEURS.length]}
                      cote={cote}
                      largeur={largeur}
                      choisie={selection.has(page.id)}
                      onClic={(evenement) => cliquer(page.id, evenement)}
                      onAgrandir={() => setApercu(numero - 1)}
                      onPivoter={(sens) => modifier(pivoter(pages, new Set([page.id]), sens))}
                      onSupprimer={() => supprimer(new Set([page.id]))}
                    />
                  );
                })}
              </Box>
            </SortableContext>
            <DragOverlay>
              {pageTiree && tirees && (
                <Paper withBorder shadow="md" p={6} radius="md" style={{ cursor: "grabbing", position: "relative" }}>
                  <Vignette chemin={sources[pageTiree.source].chemin} page={pageTiree.page} rotation={pageTiree.rotation} taille={cote} largeur={largeur} alt="" />
                  {tirees.groupe.length > 1 && (
                    <Badge pos="absolute" top={-8} right={-8} size="lg" circle={false}>
                      {t.groupe(tirees.groupe.length)}
                    </Badge>
                  )}
                </Paper>
              )}
            </DragOverlay>
          </DndContext>
        </>
      )}

      <ApercuPage pages={pages} index={apercu} source={(page) => sources[page.source]} onIndex={setApercu} />
    </Stack>
  );
}
