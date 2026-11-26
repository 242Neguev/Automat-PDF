import { useEffect, useState } from "react";
import { Alert, Badge, Box, Button, Checkbox, Group, Loader, Paper, SegmentedControl, Select, Stack, Switch, Text } from "@mantine/core";
import { IconClearAll, IconDeviceFloppy, IconPlayerStop, IconRefresh, IconScan } from "@tabler/icons-react";
import { closestCenter, DndContext, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { rectSortingStrategy, SortableContext, sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import {
  annulerNumerisation,
  enregistrerNumerisation,
  numeriser,
  oublierNumerisation,
  scanners as chercherScanners,
  type CouleurNumerisation,
  type Scanner,
  type SourceNumerisation,
} from "../../coeur";
import { ecrireReglages, lireReglages, nomNumerisation } from "../../donnees/numerisation";
import { textes } from "../../textes";
import Action from "../commun/Action";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import CartePage from "../organiser/CartePage";
import { deplacer, pivoter } from "../organiser/pages";
import "../organiser/organiser.css";

const t = textes.numeriser;
const COTE = 150;

interface PageTravail {
  id: string;
  chemin: string;
  resolution: number;
  rotation: number;
}

// Les pages de la session : on peut passer à un autre outil et revenir sans les perdre.
let pagesGardees: PageTravail[] = [];
let compteur = 0;

/** Numériser : le scanner, ses réglages, les pages numérisées (dans l'ordre choisi), puis le PDF. */
export default function Numeriser() {
  const [gardes] = useState(lireReglages);
  const [liste, setListe] = useState<Scanner[] | null>(null);
  const [scanner, setScanner] = useState<string | null>(gardes.scanner);
  const [source, setSource] = useState<SourceNumerisation>(gardes.source);
  const [rectoVerso, setRectoVerso] = useState(gardes.rectoVerso);
  const [couleur, setCouleur] = useState<CouleurNumerisation>(gardes.couleur);
  const [resolution, setResolution] = useState(String(gardes.resolution));
  const [reconnaitre, setReconnaitre] = useState(gardes.reconnaitre);
  const [pages, setPagesEnCours] = useState<PageTravail[]>(pagesGardees);
  /** Pages numérisées par la numérisation en cours ; null : aucune en cours. */
  const [enCours, setEnCours] = useState<number | null>(null);
  /** Enregistrement en cours ; avec la reconnaissance du texte : pages lues, pages à lire. */
  const [enregistrement, setEnregistrement] = useState<[number, number] | "debut" | null>(null);
  const [avis, setAvis] = useState<{ couleur: "blue" | "orange"; texte: string } | null>(null);
  const [cree, setCree] = useState<Cree | null>(null);
  const largeur = Math.round(COTE * Math.min(2, window.devicePixelRatio || 1));
  const capteurs = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  function setPages(miseAJour: (actuelles: PageTravail[]) => PageTravail[]) {
    setPagesEnCours((actuelles) => {
      pagesGardees = miseAJour(actuelles);
      return pagesGardees;
    });
  }

  useEffect(() => {
    const arrets = [
      listen<number>("progression-numerisation", (evenement) => setEnCours(evenement.payload)),
      listen<[number, number]>("progression-reconnaissance", (evenement) => setEnregistrement((actuel) => (actuel ? evenement.payload : actuel))),
    ].map((arret) => arret.catch(() => () => {}));
    void chercher();
    return () => arrets.forEach((arret) => void arret.then((arreter) => arreter()));
  }, []);

  useEffect(
    () => ecrireReglages({ scanner, source, rectoVerso, couleur, resolution: Number(resolution), reconnaitre }),
    [scanner, source, rectoVerso, couleur, resolution, reconnaitre],
  );

  async function chercher() {
    setListe(null);
    try {
      const trouves = await chercherScanners();
      setListe(trouves);
      setScanner((choisi) => (trouves.some((trouve) => trouve.id === choisi) ? choisi : (trouves[0]?.id ?? choisi)));
    } catch (e) {
      setListe([]);
      signalerErreur(e);
    }
  }

  const sources = liste?.find((trouve) => trouve.id === scanner)?.sources ?? null;
  // Quand on sait ce que fait le scanner, seules ses sources sont proposées.
  const sourceChoisie: SourceNumerisation = sources && !sources[source] ? (sources.vitre ? "vitre" : "chargeur") : source;
  const rectoVersoPossible = sourceChoisie === "chargeur" && (sources?.rectoVerso ?? true);
  const occupe = enCours !== null || enregistrement !== null;
  const pret = Boolean(scanner && liste?.some((trouve) => trouve.id === scanner));

  async function lancer() {
    if (!scanner) return;
    setEnCours(0);
    setAvis(null);
    setCree(null);
    try {
      const resultat = await numeriser({
        scanner,
        source: sourceChoisie,
        rectoVerso: rectoVersoPossible && rectoVerso,
        couleur,
        resolution: Number(resolution),
      });
      const nouvelles = resultat.pages.map((page) => ({ id: `n${++compteur}`, chemin: page.chemin, resolution: page.resolution, rotation: 0 }));
      setPages((actuelles) => [...actuelles, ...nouvelles]);
      if (resultat.sources) {
        const apprises = resultat.sources;
        setListe((actuelle) => actuelle?.map((trouve) => (trouve.id === scanner ? { ...trouve, sources: apprises } : trouve)) ?? actuelle);
      }
      if (resultat.interrompue) setAvis({ couleur: "orange", texte: t.interrompue(nouvelles.length, resultat.interrompue) });
      else if (resultat.annulee) setAvis({ couleur: "blue", texte: t.arretee(nouvelles.length) });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(null);
    }
  }

  async function enregistrer() {
    const sortie = await save({ defaultPath: nomNumerisation(new Date()), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnregistrement("debut");
    setCree(null);
    try {
      const bilan = await enregistrerNumerisation(
        pages.map(({ chemin, rotation, resolution: ppp }) => ({ chemin, rotation, resolution: ppp })),
        reconnaitre,
        sortie,
      );
      const details = [textes.pdf.pages(bilan.pages)];
      if (bilan.texte) details.push(t.texteReconnu(bilan.texte.pagesReconnues));
      if (bilan.sansTexte) details.push(t.sansTexte(bilan.sansTexte));
      setCree({ titre: t.cree, chemin: sortie, details, attention: Boolean(bilan.sansTexte) });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnregistrement(null);
    }
  }

  async function effacer() {
    setPages(() => []);
    setAvis(null);
    setCree(null);
    try {
      await oublierNumerisation();
    } catch (e) {
      signalerErreur(e);
    }
  }

  function finDeplacement({ active, over }: DragEndEvent) {
    if (over) setPages((actuelles) => deplacer(actuelles, [String(active.id)], String(active.id), String(over.id)));
  }

  return (
    <Stack gap="md">
      <Paper withBorder p="md">
        <Stack gap="md">
          <Group align="flex-end" gap="sm">
            <Select
              label={t.scanner}
              data={(liste ?? []).map((trouve) => ({ value: trouve.id, label: trouve.nom }))}
              value={pret ? scanner : null}
              onChange={(valeur) => valeur && setScanner(valeur)}
              placeholder={liste === null ? t.recherche : t.aucun}
              disabled={!liste?.length || occupe}
              allowDeselect={false}
              w={340}
              maw="100%"
              data-scanners
            />
            <Button
              variant="default"
              leftSection={liste === null ? <Loader size={16} /> : <IconRefresh size={18} />}
              onClick={() => void chercher()}
              disabled={liste === null || occupe}
            >
              {t.rechercher}
            </Button>
          </Group>
          {liste?.length === 0 && (
            <Alert color="orange" title={t.aucun} data-aucun-scanner>
              {t.aucunDetail}
            </Alert>
          )}
          <Group gap="xl" align="flex-start">
            <Stack gap={4}>
              <Text size="sm" fw={500}>
                {t.source}
              </Text>
              <SegmentedControl
                value={sourceChoisie}
                onChange={(valeur) => setSource(valeur as SourceNumerisation)}
                data={(["vitre", "chargeur"] as const).map((value) => ({ value, label: t.sources[value], disabled: Boolean(sources && !sources[value]) }))}
                disabled={occupe}
              />
              {rectoVersoPossible && (
                <Checkbox mt={6} label={t.rectoVerso} checked={rectoVerso} onChange={(evenement) => setRectoVerso(evenement.currentTarget.checked)} disabled={occupe} />
              )}
            </Stack>
            <Stack gap={4}>
              <Text size="sm" fw={500}>
                {t.couleur}
              </Text>
              <SegmentedControl
                value={couleur}
                onChange={(valeur) => setCouleur(valeur as CouleurNumerisation)}
                data={(["couleur", "gris", "noir-et-blanc"] as const).map((value) => ({ value, label: t.couleurs[value] }))}
                disabled={occupe}
              />
            </Stack>
            <Stack gap={4}>
              <Text size="sm" fw={500}>
                {t.resolution}
              </Text>
              <SegmentedControl
                value={resolution}
                onChange={setResolution}
                data={(["150", "300", "600"] as const).map((value) => ({ value, label: t.resolutions[value] }))}
                disabled={occupe}
              />
              <Text size="xs" c="dimmed" maw={320}>
                {t.resolutionAide}
              </Text>
            </Stack>
          </Group>
          <Group gap="sm">
            <Button
              size="md"
              leftSection={enCours !== null ? <Loader size={18} color="white" /> : <IconScan size={20} />}
              onClick={() => void lancer()}
              disabled={!pret || occupe}
              data-numeriser
            >
              {enCours !== null ? t.enCours(enCours) : pages.length ? t.encore : t.numeriser}
            </Button>
            {enCours !== null && (
              <Button size="md" variant="default" leftSection={<IconPlayerStop size={18} />} onClick={() => void annulerNumerisation().catch(signalerErreur)}>
                {t.arreter}
              </Button>
            )}
          </Group>
        </Stack>
      </Paper>

      {avis && (
        <Alert color={avis.couleur} withCloseButton onClose={() => setAvis(null)} data-avis-numerisation>
          {avis.texte}
        </Alert>
      )}

      {pages.length === 0 ? (
        <Text size="sm" c="dimmed">
          {t.vide}
        </Text>
      ) : (
        <>
          <Group justify="space-between">
            <Group gap="xs">
              <Badge variant="light" size="lg">
                {t.pages(pages.length)}
              </Badge>
              <Text size="sm" c="dimmed">
                {t.aide}
              </Text>
            </Group>
            <Button variant="subtle" color="gray" leftSection={<IconClearAll size={18} />} onClick={() => void effacer()} disabled={occupe}>
              {t.effacer}
            </Button>
          </Group>
          <DndContext sensors={capteurs} collisionDetection={closestCenter} onDragEnd={finDeplacement}>
            <SortableContext items={pages.map((page) => page.id)} strategy={rectSortingStrategy}>
              <Box style={{ display: "grid", gridTemplateColumns: `repeat(auto-fill, minmax(${COTE + 14}px, 1fr))`, gap: 12 }} data-pages-numerisees>
                {pages.map((page, index) => (
                  <CartePage
                    key={page.id}
                    element={page}
                    numero={index + 1}
                    chemin={page.chemin}
                    legende={t.legende(page.resolution)}
                    description={t.detail(index + 1, page.resolution)}
                    couleur="eau"
                    cote={COTE}
                    largeur={largeur}
                    choisie={false}
                    onClic={() => undefined}
                    onPivoter={(sens) => setPages((actuelles) => pivoter(actuelles, new Set([page.id]), sens))}
                    onSupprimer={() => setPages((actuelles) => actuelles.filter((autre) => autre.id !== page.id))}
                  />
                ))}
              </Box>
            </SortableContext>
          </DndContext>
          <Paper withBorder p="md">
            <Group justify="space-between" align="center" gap="md">
              <Switch
                label={t.reconnaitre}
                description={t.reconnaitreAide}
                checked={reconnaitre}
                onChange={(evenement) => setReconnaitre(evenement.currentTarget.checked)}
                disabled={occupe}
                data-reconnaitre
              />
              <Action icone={IconDeviceFloppy} enCours={enregistrement !== null} desactive={enCours !== null} onClick={() => void enregistrer()}>
                {t.enregistrer}
              </Action>
            </Group>
            {Array.isArray(enregistrement) && (
              <Text size="sm" c="dimmed" mt="xs">
                {t.reconnaissance(...enregistrement)}
              </Text>
            )}
          </Paper>
        </>
      )}

      <Resultat cree={cree} onFermer={() => setCree(null)} />
      <Text size="xs" c="dimmed">
        {t.confidentialite}
      </Text>
    </Stack>
  );
}
