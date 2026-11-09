import { Fragment, useEffect, useRef, useState } from "react";
import { ActionIcon, AppShell, Box, Group, Modal, NavLink, ScrollArea, SimpleGrid, Stack, Text, ThemeIcon, Title, Tooltip, UnstyledButton } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useDisclosure } from "@mantine/hooks";
import { IconHome, IconInfoCircle } from "@tabler/icons-react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { version } from "../package.json";
import Accueil, { TuileOutil } from "./Accueil";
import APropos from "./APropos";
import { enErreur, lireConfidentialite, recusAuLancement, reglerConfidentialite, signalerOuverture, type Confidentialite, type Recus } from "./coeur";
import { QuestionStatistiques, type Reglage } from "./Confidentialite";
import { nomDe } from "./donnees/chemins";
import { BandeauMiseAJour, useMiseAJour } from "./MiseAJour";
import { GROUPES, OUTILS, type DefinitionOutil, type Outil } from "./outils";
import { sortesDe, transmettre } from "./pages/commun/reception";
import { textes } from "./textes";
import { COULEURS } from "./theme";

/** Ce qui est affiché : l'accueil, ou un outil. */
type Vue = Outil | "accueil";

const tr = textes.reception;
const prend = (definition: DefinitionOutil, fichier: string) => sortesDe(fichier).some((sorte) => definition.accepte.includes(sorte));

/** Signal d'ouverture des statistiques anonymes : le cœur ne l'envoie qu'avec l'accord enregistré. */
const signaler = () => void signalerOuverture(navigator.language).catch((erreur) => console.warn("Statistiques :", enErreur(erreur).message));

export default function App() {
  const [aProposOuvert, aPropos] = useDisclosure(false);
  const [vue, setVue] = useState<Vue>("accueil");
  const [survol, setSurvol] = useState(false);
  const [choix, setChoix] = useState<{ fichiers: string[]; outils: DefinitionOutil[]; depuisAccueil: boolean } | null>(null);
  const vueAffichee = useRef(vue);
  vueAffichee.current = vue;
  // Réglages de confidentialité : null tant qu'ils ne sont pas lus (ni question, ni recherche).
  const [confidentialite, setConfidentialite] = useState<Confidentialite | null>(null);
  // La question du premier lancement propose aussi de couper la recherche des mises à jour : rien
  // n'est cherché avant la réponse.
  const questionEnAttente = confidentialite?.statistiquesPossibles === true && confidentialite.statistiques === null;
  const miseAJour = useMiseAJour(confidentialite?.misesAJour === true && !questionEnAttente);

  useEffect(() => {
    lireConfidentialite().then(
      (lus) => {
        setConfidentialite(lus);
        if (lus.statistiques === true) signaler();
      },
      (erreur) => {
        console.warn("Réglages de confidentialité :", enErreur(erreur).message);
        setConfidentialite({ misesAJour: true, statistiques: null, statistiquesPossibles: false });
      },
    );
  }, []);

  /** Enregistre un réglage de confidentialité ; au premier « Oui », le signal part aussitôt. */
  async function regler(reglage: Reglage) {
    try {
      setConfidentialite(await reglerConfidentialite(reglage));
      if (reglage.statistiques === true) signaler();
    } catch (erreur) {
      // Le choix vaut au moins pour cette fois : la question ne bloque pas la fenêtre.
      setConfidentialite((avant) => avant && { ...avant, ...reglage });
      notifications.show({ color: "red", title: textes.confidentialite.nonEnregistre, message: enErreur(erreur).message, autoClose: 15000 });
    }
  }
  // Chaque écran s'ouvre par son haut : son titre et ce qu'il faut faire d'abord. (WebView2 renvoie
  // une promesse de `scrollTo` : l'effet ne doit rien renvoyer.)
  useEffect(() => {
    window.scrollTo({ top: 0 });
  }, [vue]);

  /** Confie à l'outil les fichiers qu'il prend ; les autres sont signalés. */
  function confier(definition: DefinitionOutil, fichiers: string[]) {
    const laisses = fichiers.filter((fichier) => !prend(definition, fichier));
    setVue(definition.cle);
    transmettre(
      definition.cle,
      fichiers.filter((fichier) => prend(definition, fichier)),
    );
    if (laisses.length > 0) notifications.show({ color: "orange", title: tr.ignores(laisses.length), message: laisses.map(nomDe).join(", "), autoClose: 10000 });
  }

  /**
   * Fichiers arrivés : à l'outil demandé (clic droit), sinon à l'outil affiché. Depuis l'accueil, ou
   * si l'outil affiché n'en prend aucun, on demande quoi en faire (sauf s'il n'y a qu'une possibilité).
   */
  const recevoir = useRef<(fichiers: string[], demande: string | null) => void>(() => {});
  recevoir.current = (fichiers, demande) => {
    const cible = OUTILS.find((definition) => definition.cle === demande) ?? OUTILS.find((definition) => definition.cle === vueAffichee.current);
    if (cible && fichiers.some((fichier) => prend(cible, fichier))) return confier(cible, fichiers);
    const possibles = OUTILS.filter((definition) => fichiers.some((fichier) => prend(definition, fichier)));
    const depuisAccueil = vueAffichee.current === "accueil";
    if (possibles.length === 1 && depuisAccueil) return confier(possibles[0], fichiers);
    if (possibles.length > 0) setChoix({ fichiers, outils: possibles, depuisAccueil });
    else notifications.show({ color: "orange", title: tr.inconnus, message: tr.inconnusAide, autoClose: 10000 });
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
          <UnstyledButton onClick={() => setVue("accueil")} aria-label={textes.accueil.titre}>
            <Group gap="sm">
              <ThemeIcon size="lg" radius="md" color={COULEURS.encre}>
                <Text fw={800} size="xs" c={COULEURS.sec}>
                  PDF
                </Text>
              </ThemeIcon>
              <Title order={4}>{textes.application}</Title>
            </Group>
          </UnstyledButton>
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
            <NavLink label={textes.accueil.titre} leftSection={<IconHome size={20} stroke={1.6} />} active={vue === "accueil"} onClick={() => setVue("accueil")} />
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
                    active={cle === vue}
                    onClick={() => setVue(cle)}
                  />
                ))}
              </Fragment>
            ))}
          </Stack>
        </AppShell.Section>
      </AppShell.Navbar>

      {/* Les outils restent montés : on retrouve son travail en changeant d'outil. */}
      <AppShell.Main>
        <BandeauMiseAJour miseAJour={miseAJour} versionActuelle={version} />
        <Box display={vue === "accueil" ? undefined : "none"}>
          <Accueil groupes={GROUPES} onOutil={setVue} onFichiers={(fichiers) => recevoir.current(fichiers, null)} />
        </Box>
        {OUTILS.map(({ cle, page: Page }) => (
          <Box key={cle} display={cle === vue ? undefined : "none"}>
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
          style={{ zIndex: 300, pointerEvents: "none", border: "3px dashed var(--mantine-color-eau-6)", borderRadius: 12, background: "rgba(255, 255, 255, 0.85)" }}
          data-survol
        >
          <Stack h="100%" justify="center" align="center">
            <Title order={3}>{vue === "accueil" ? tr.deposerAccueil : tr.deposer(textes.outils[vue].titre)}</Title>
          </Stack>
        </Box>
      )}

      <Modal opened={choix !== null} onClose={() => setChoix(null)} title={choix ? tr.choisir(choix.fichiers.length) : ""} size="lg">
        {choix && (
          <Stack gap="sm" data-choix-outil>
            <Text size="sm" c="dimmed">
              {choix.depuisAccueil ? tr.aideAccueil : tr.aide}
            </Text>
            <SimpleGrid cols={choix.outils.length > 3 ? 2 : 1} spacing="xs" verticalSpacing="xs">
              {choix.outils.map((definition) => (
                <TuileOutil
                  key={definition.cle}
                  definition={definition}
                  onClick={() => {
                    confier(definition, choix.fichiers);
                    setChoix(null);
                  }}
                />
              ))}
            </SimpleGrid>
          </Stack>
        )}
      </Modal>

      <APropos ouvert={aProposOuvert} fermer={aPropos.close} confidentialite={confidentialite} regler={(reglage) => void regler(reglage)} miseAJour={miseAJour} />
      <QuestionStatistiques ouverte={questionEnAttente} repondre={(reponse) => void regler(reponse)} />
    </AppShell>
  );
}
