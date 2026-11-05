import { Fragment, useEffect, useRef, useState, type ComponentType } from "react";
import { ActionIcon, AppShell, Box, Button, Group, Modal, NavLink, ScrollArea, Stack, Text, ThemeIcon, Title, Tooltip } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { notifications } from "@mantine/notifications";
import { IconArrowsMinimize, IconEyeOff, IconFiles, IconFileTypeDocx, IconFileTypePdf, IconInfoCircle, IconLock, IconLayersIntersect, IconLayoutColumns, IconLayoutGrid, IconNumbers, IconPhoto, IconPhotoPlus, IconScissors, IconSignature, IconUsers } from "@tabler/icons-react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import APropos from "./APropos";
import { recusAuLancement, type Recus } from "./coeur";
import { sortesDe, transmettre, type SorteFichier } from "./pages/commun/reception";
import OfficeEnPdf from "./pages/convertir/OfficeEnPdf";
import PdfEnWord from "./pages/convertir/PdfEnWord";
import Compresser from "./pages/optimiser/Compresser";
import MotDePasse from "./pages/securite/MotDePasse";
import Caviarder from "./pages/annoter/Caviarder";
import NumerosTampon from "./pages/annoter/NumerosTampon";
import SignatureCachet from "./pages/annoter/SignatureCachet";
import FusionPdf from "./pages/fusion/FusionPdf";
import ImagesEnPdf from "./pages/images/ImagesEnPdf";
import PdfEnImages from "./pages/images/PdfEnImages";
import IntercalerPdf from "./pages/intercaler/IntercalerPdf";
import MiseEnPage from "./pages/mise-en-page/MiseEnPage";
import OrganiserPdf from "./pages/organiser/OrganiserPdf";
import SeparationParPersonne from "./pages/separation/SeparationParPersonne";
import SeparationPdf from "./pages/separation/SeparationPdf";
import { textes } from "./textes";
import { COULEURS } from "./theme";

type Outil = keyof typeof textes.outils;

interface DefinitionOutil {
  cle: Outil;
  icone: typeof IconFiles;
  page: ComponentType;
  /** Fichiers qu'il prend de l'Explorateur (clic droit, glisser-déposer). */
  accepte: SorteFichier[];
}

const GROUPES: { cle: keyof typeof textes.groupes; outils: DefinitionOutil[] }[] = [
  {
    cle: "pages",
    outils: [
      { cle: "organiser", icone: IconLayoutGrid, page: OrganiserPdf, accepte: ["pdf"] },
      { cle: "miseEnPage", icone: IconLayoutColumns, page: MiseEnPage, accepte: ["pdf"] },
    ],
  },
  {
    cle: "assembler",
    outils: [
      { cle: "fusionner", icone: IconFiles, page: FusionPdf, accepte: ["pdf"] },
      { cle: "intercaler", icone: IconLayersIntersect, page: IntercalerPdf, accepte: ["pdf"] },
      { cle: "images", icone: IconPhotoPlus, page: ImagesEnPdf, accepte: ["image"] },
    ],
  },
  {
    cle: "decouper",
    outils: [
      { cle: "separer", icone: IconScissors, page: SeparationPdf, accepte: ["pdf", "liste"] },
      { cle: "parPersonne", icone: IconUsers, page: SeparationParPersonne, accepte: ["pdf", "liste"] },
    ],
  },
  {
    cle: "annoter",
    outils: [
      { cle: "marques", icone: IconNumbers, page: NumerosTampon, accepte: ["pdf"] },
      { cle: "signature", icone: IconSignature, page: SignatureCachet, accepte: ["pdf", "image"] },
    ],
  },
  {
    cle: "securite",
    outils: [
      { cle: "motDePasse", icone: IconLock, page: MotDePasse, accepte: ["pdf"] },
      { cle: "caviarder", icone: IconEyeOff, page: Caviarder, accepte: ["pdf"] },
    ],
  },
  { cle: "optimiser", outils: [{ cle: "compresser", icone: IconArrowsMinimize, page: Compresser, accepte: ["pdf"] }] },
  {
    cle: "convertir",
    outils: [
      { cle: "enImages", icone: IconPhoto, page: PdfEnImages, accepte: ["pdf"] },
      { cle: "officeEnPdf", icone: IconFileTypePdf, page: OfficeEnPdf, accepte: ["office"] },
      { cle: "pdfEnWord", icone: IconFileTypeDocx, page: PdfEnWord, accepte: ["pdf"] },
    ],
  },
];
const OUTILS = GROUPES.flatMap((groupe) => groupe.outils);
const tr = textes.reception;
const prend = (definition: DefinitionOutil, fichier: string) => sortesDe(fichier).some((sorte) => definition.accepte.includes(sorte));
const nomDe = (chemin: string) => chemin.split(/[\\/]/).pop() ?? chemin;

export default function App() {
  const [aProposOuvert, aPropos] = useDisclosure(false);
  const [outil, setOutil] = useState<Outil>("organiser");
  const [survol, setSurvol] = useState(false);
  const [choix, setChoix] = useState<{ fichiers: string[]; outils: DefinitionOutil[] } | null>(null);
  const outilAffiche = useRef(outil);
  outilAffiche.current = outil;

  /** Confie à l'outil les fichiers qu'il prend ; les autres sont signalés. */
  function confier(definition: DefinitionOutil, fichiers: string[]) {
    const laisses = fichiers.filter((fichier) => !prend(definition, fichier));
    setOutil(definition.cle);
    transmettre(
      definition.cle,
      fichiers.filter((fichier) => prend(definition, fichier)),
    );
    if (laisses.length > 0) notifications.show({ color: "orange", title: tr.ignores(laisses.length), message: laisses.map(nomDe).join(", ") });
  }

  /** Fichiers arrivés : à l'outil demandé (clic droit), sinon à l'outil affiché ; s'il n'en prend aucun, on demande. */
  const recevoir = useRef<(fichiers: string[], demande: string | null) => void>(() => {});
  recevoir.current = (fichiers, demande) => {
    const cible = OUTILS.find((definition) => definition.cle === demande) ?? OUTILS.find((definition) => definition.cle === outilAffiche.current);
    if (cible && fichiers.some((fichier) => prend(cible, fichier))) return confier(cible, fichiers);
    const possibles = OUTILS.filter((definition) => fichiers.some((fichier) => prend(definition, fichier)));
    if (possibles.length > 0) setChoix({ fichiers, outils: possibles });
    else notifications.show({ color: "orange", title: tr.inconnus, message: tr.inconnusAide });
  };

  useEffect(() => {
    // Plusieurs fichiers choisis dans l'Explorateur arrivent chacun par son propre lancement : on les réunit.
    let attente: Recus | null = null;
    let minuterie: number | undefined;
    const livrer = () => {
      if (attente) recevoir.current(attente.fichiers, attente.outil);
      attente = null;
    };
    const ajouter = (recus: Recus) => {
      if (attente && attente.outil !== recus.outil) livrer();
      attente = { outil: recus.outil, fichiers: [...(attente?.fichiers ?? []), ...recus.fichiers] };
      window.clearTimeout(minuterie);
      minuterie = window.setTimeout(livrer, 600);
    };
    // L'écoute est en place avant que l'on demande les fichiers gardés : aucun ne se perd entre les deux.
    const ecoute = listen<Recus>("fichiers-recus", (evenement) => ajouter(evenement.payload));
    ecoute.then(() => recusAuLancement()).then((gardes) => gardes.forEach((recus) => ajouter(recus)), () => {});
    const arrets = [
      ecoute,
      (async () =>
        getCurrentWebview().onDragDropEvent((evenement) => {
          setSurvol(evenement.payload.type === "enter" || evenement.payload.type === "over");
          if (evenement.payload.type === "drop") recevoir.current(evenement.payload.paths, null);
        }))(),
    ].map((arret) => arret.catch(() => () => {}));
    return () => {
      window.clearTimeout(minuterie);
      arrets.forEach((arret) => void arret.then((arreter) => arreter()));
    };
  }, []);

  return (
    <AppShell header={{ height: 56 }} navbar={{ width: 280, breakpoint: 0 }} padding="lg">
      <AppShell.Header>
        <Group h="100%" px="lg" justify="space-between">
          <Group gap="sm">
            <ThemeIcon size="lg" radius="md" color={COULEURS.encre}>
              <Text fw={800} size="xs" c={COULEURS.sec}>
                PDF
              </Text>
            </ThemeIcon>
            <Title order={4}>{textes.application}</Title>
          </Group>
          <Group gap="xs">
            <Text size="sm" c="dimmed">
              {textes.parEditeur}
            </Text>
            <Tooltip label={textes.aPropos.bouton}>
              <ActionIcon variant="subtle" color="gray" aria-label={textes.aPropos.bouton} onClick={aPropos.open}>
                <IconInfoCircle size={20} stroke={1.6} />
              </ActionIcon>
            </Tooltip>
          </Group>
        </Group>
      </AppShell.Header>

      <AppShell.Navbar p="sm">
        {/* Le menu défile quand la fenêtre est trop basse pour tout montrer (écrans de portable). */}
        <AppShell.Section grow component={ScrollArea}>
          <Stack gap={4}>
            {GROUPES.map((groupe) => (
              <Fragment key={groupe.cle}>
                <Text size="xs" fw={700} c="dimmed" tt="uppercase" px="sm" pt="sm" pb={2}>
                  {textes.groupes[groupe.cle]}
                </Text>
                {groupe.outils.map(({ cle, icone: Icone }) => (
                  <NavLink
                    key={cle}
                    label={textes.outils[cle].titre}
                    description={textes.outils[cle].description}
                    leftSection={<Icone size={20} stroke={1.6} />}
                    active={cle === outil}
                    onClick={() => setOutil(cle)}
                  />
                ))}
              </Fragment>
            ))}
          </Stack>
        </AppShell.Section>
      </AppShell.Navbar>

      {/* Les outils restent montés : on retrouve son travail en changeant d'outil. */}
      <AppShell.Main>
        {OUTILS.map(({ cle, page: Page }) => (
          <Box key={cle} display={cle === outil ? undefined : "none"}>
            <Stack gap="lg" maw={1240}>
              <div>
                <Title order={2}>{textes.outils[cle].titre}</Title>
                <Text c="dimmed">{textes.outils[cle].sousTitre}</Text>
              </div>
              <Page />
            </Stack>
          </Box>
        ))}
      </AppShell.Main>

      {survol && (
        <Box
          pos="fixed"
          inset={12}
          style={{ zIndex: 300, pointerEvents: "none", border: "3px dashed var(--mantine-color-eau-6)", borderRadius: 12, background: "rgba(255, 255, 255, 0.8)" }}
          data-survol
        >
          <Stack h="100%" justify="center" align="center">
            <Title order={3}>{tr.deposer(textes.outils[outil].titre)}</Title>
          </Stack>
        </Box>
      )}

      <Modal opened={choix !== null} onClose={() => setChoix(null)} title={choix ? tr.choisir(choix.fichiers.length) : ""}>
        {choix && (
          <Stack gap="xs" data-choix-outil>
            <Text size="sm" c="dimmed">
              {tr.aide}
            </Text>
            {choix.outils.map((definition) => {
              const Icone = definition.icone;
              return (
                <Button
                  key={definition.cle}
                  variant="default"
                  justify="flex-start"
                  leftSection={<Icone size={18} stroke={1.6} />}
                  onClick={() => {
                    confier(definition, choix.fichiers);
                    setChoix(null);
                  }}
                >
                  {textes.outils[definition.cle].titre}
                </Button>
              );
            })}
          </Stack>
        )}
      </Modal>

      <APropos ouvert={aProposOuvert} fermer={aPropos.close} />
    </AppShell>
  );
}
