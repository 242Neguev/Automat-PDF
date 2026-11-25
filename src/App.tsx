import { useEffect, useRef, useState } from "react";
import { ActionIcon, AppShell, Box, Button, CloseButton, Group, Modal, NavLink, ScrollArea, SimpleGrid, Stack, Text, ThemeIcon, Title, Tooltip, UnstyledButton } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { useDisclosure } from "@mantine/hooks";
import { IconFileText, IconFileTypePdf, IconHome, IconInfoCircle, IconPlus } from "@tabler/icons-react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { version } from "../package.json";
import Accueil, { TuileOutil } from "./Accueil";
import APropos from "./APropos";
import { enErreur, lireConfidentialite, recusAuLancement, reglerConfidentialite, signalerOuverture, type Confidentialite, type Recus } from "./coeur";
import { QuestionStatistiques, type Reglage } from "./Confidentialite";
import { nomDe } from "./donnees/chemins";
import { destination } from "./donnees/destination";
import { ajouterRecent, ecrireRecents, lireRecents, retirerRecent, type DocumentRecent } from "./donnees/recents";
import { BandeauMiseAJour, useMiseAJour } from "./MiseAJour";
import { GROUPE_DE, GROUPES, OUTILS, type DefinitionOutil, type Outil } from "./outils";
import { sortesDe, transmettre } from "./pages/commun/reception";
import { MAC } from "./systeme";
import Vignette from "./pages/commun/Vignette";
import Lecteur from "./pages/lecteur/Lecteur";
import { textes } from "./textes";
import { varsTeinte } from "./theme";

/** Ce qui est affiché : l'accueil, le lecteur de PDF, ou un outil. */
type Vue = Outil | "accueil" | "lecteur";

const tr = textes.reception;
const tb = textes.barre;
const prend = (definition: DefinitionOutil, fichier: string) => sortesDe(fichier).some((sorte) => definition.accepte.includes(sorte));
/** Les outils proposés à côté d'un PDF ouvert dans le lecteur. */
const OUTILS_PDF = OUTILS.filter((definition) => definition.accepte.includes("pdf"));

/** Signal d'ouverture des statistiques anonymes : le cœur ne l'envoie qu'avec l'accord enregistré. */
const signaler = () => void signalerOuverture(navigator.language).catch((erreur) => console.warn("Statistiques :", enErreur(erreur).message));

export default function App() {
  const [aProposOuvert, aPropos] = useDisclosure(false);
  const [vue, setVue] = useState<Vue>("accueil");
  const [survol, setSurvol] = useState(false);
  const [choix, setChoix] = useState<{ fichiers: string[]; outils: DefinitionOutil[]; depuisAccueil: boolean } | null>(null);
  /** Le PDF ouvert dans le lecteur, et les derniers ouverts (gardés sur l'ordinateur). */
  const [lu, setLu] = useState<string | null>(null);
  const [recents, setRecents] = useState<DocumentRecent[]>(() => lireRecents());
  useEffect(() => ecrireRecents(recents), [recents]);
  /**
   * Groupes d'outils ouverts dans la barre latérale. En passant à un outil, seul son groupe reste ouvert :
   * la barre reste courte ; on peut toujours en ouvrir d'autres.
   */
  const [groupesOuverts, setGroupesOuverts] = useState<string[]>([]);
  useEffect(() => {
    if (OUTILS.some((definition) => definition.cle === vue)) setGroupesOuverts([GROUPE_DE[vue as Outil].cle]);
  }, [vue]);
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

  /**
   * Confie à l'outil les fichiers qu'il prend ; les autres sont signalés. Une question encore
   * ouverte sur d'autres fichiers (« Que faire de ce fichier ? ») est refermée : elle cacherait l'outil.
   */
  function confier(definition: DefinitionOutil, fichiers: string[]) {
    const laisses = fichiers.filter((fichier) => !prend(definition, fichier));
    setChoix(null);
    setVue(definition.cle);
    transmettre(
      definition.cle,
      fichiers.filter((fichier) => prend(definition, fichier)),
    );
    if (laisses.length > 0) notifications.show({ color: "orange", title: tr.ignores(laisses.length), message: laisses.map(nomDe).join(", "), autoClose: 10000 });
  }

  /** Ouvre un PDF dans le lecteur ; il passe en tête des documents récents. */
  function lire(chemin: string) {
    setChoix(null);
    setLu(chemin);
    setVue("lecteur");
    setRecents((avant) => ajouterRecent(avant, chemin, Date.now()));
  }

  async function choisirPdf() {
    const choisi = await open({ multiple: false, directory: false, filters: [{ name: textes.lecteur.filtre, extensions: ["pdf"] }] });
    if (typeof choisi === "string") lire(choisi);
  }

  /**
   * Fichiers arrivés : à l'outil demandé (clic droit), sinon à l'outil affiché ; un seul PDF va au
   * lecteur ; sinon on demande quoi en faire, sauf s'il n'y a qu'une possibilité (donnees/destination).
   */
  const recevoir = useRef<(fichiers: string[], demande: string | null) => void>(() => {});
  recevoir.current = (fichiers, demande) => {
    const vers = destination(fichiers, demande, vueAffichee.current, OUTILS, prend);
    if (vers.sorte === "outil") return confier(vers.outil, fichiers);
    if (vers.sorte === "lecteur") return lire(vers.chemin);
    const depuisAccueil = vueAffichee.current === "accueil" || vueAffichee.current === "lecteur";
    if (vers.sorte === "choix") setChoix({ fichiers, outils: vers.outils, depuisAccueil });
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
    <AppShell navbar={{ width: 300, breakpoint: 0 }} padding="lg">
      {/* Mac : pas de barre de titre (fenêtre en verre) ; cette bande, en haut, sert à déplacer la fenêtre. */}
      {MAC && <div className="zone-glisser" data-tauri-drag-region />}
      {/* Barre latérale en verre : l'application, l'accueil et le lecteur, les documents récents, puis les outils. */}
      <AppShell.Navbar p="sm">
        <div className="barre-laterale" data-barre-laterale>
          <UnstyledButton onClick={() => setVue("accueil")} aria-label={textes.accueil.titre} px={8} pb="sm">
            <Group gap="sm" wrap="nowrap">
              <ThemeIcon size={40} radius="md" color="var(--automat-logo-fond)">
                <Text fw={800} size="xs" c="var(--automat-logo-texte)">
                  PDF
                </Text>
              </ThemeIcon>
              <div>
                <Text fw={750} size="lg" lh={1.15}>
                  {textes.application}
                </Text>
                <Text size="xs" c="dimmed">
                  {textes.parEditeur}
                </Text>
              </div>
            </Group>
          </UnstyledButton>

          {/* Le menu défile quand la fenêtre est trop basse pour tout montrer (écrans de portable). */}
          <ScrollArea style={{ flex: 1 }} type="hover" scrollbarSize={6}>
            <Stack gap={2}>
              <NavLink label={textes.accueil.titre} leftSection={<IconHome size={20} stroke={1.6} />} active={vue === "accueil"} onClick={() => setVue("accueil")} />
              <NavLink label={textes.lecteur.titre} leftSection={<IconFileText size={20} stroke={1.6} />} active={vue === "lecteur"} onClick={() => setVue("lecteur")} data-menu-lecteur />

              <div className="titre-section">
                <span>{tb.recents}</span>
                <Tooltip label={tb.ouvrir}>
                  <ActionIcon variant="subtle" color="gray" size="sm" radius="md" aria-label={tb.ouvrir} onClick={() => void choisirPdf()} data-ouvrir-pdf>
                    <IconPlus size={16} stroke={2} />
                  </ActionIcon>
                </Tooltip>
              </div>
              {recents.length === 0 ? (
                <Text size="xs" c="dimmed" px={10} pb={4}>
                  {tb.aucunRecent}
                </Text>
              ) : (
                recents.slice(0, 5).map(({ chemin }) => (
                  <div key={chemin} className="document-recent" data-actif={(vue === "lecteur" && chemin === lu) || undefined} data-document-recent>
                    <UnstyledButton onClick={() => lire(chemin)} title={chemin} style={{ display: "flex", alignItems: "center", gap: 10, flex: 1, minWidth: 0 }}>
                      <Vignette chemin={chemin} page={0} taille={34} largeur={96} alt="" />
                      <Text size="sm" truncate="end">
                        {nomDe(chemin)}
                      </Text>
                    </UnstyledButton>
                    <CloseButton className="retirer" size="sm" aria-label={tb.retirer(nomDe(chemin))} onClick={() => setRecents((avant) => retirerRecent(avant, chemin))} />
                  </div>
                ))
              )}

              <div className="titre-section">
                <span>{tb.outils}</span>
              </div>
              {/* Les outils par groupe, repliables : le groupe de l'outil affiché s'ouvre de lui-même. */}
              {GROUPES.map((groupe) => {
                const IconeGroupe = groupe.icone;
                return (
                  <NavLink
                    key={groupe.cle}
                    label={textes.groupes[groupe.cle]}
                    leftSection={
                      <span className="pastille" style={varsTeinte(groupe.teinte)}>
                        <IconeGroupe size={17} stroke={1.8} />
                      </span>
                    }
                    opened={groupesOuverts.includes(groupe.cle)}
                    onChange={(ouvert) => setGroupesOuverts((avant) => (ouvert ? [...avant, groupe.cle] : avant.filter((cle) => cle !== groupe.cle)))}
                    childrenOffset={22}
                    data-groupe-menu={groupe.cle}
                  >
                    {groupe.outils.map(({ cle, icone: Icone }) => (
                      <NavLink key={cle} label={textes.outils[cle].titre} leftSection={<Icone size={17} stroke={1.7} />} active={cle === vue} onClick={() => setVue(cle)} />
                    ))}
                  </NavLink>
                );
              })}
            </Stack>
          </ScrollArea>

          <NavLink label={textes.aPropos.bouton} leftSection={<IconInfoCircle size={20} stroke={1.6} />} onClick={aPropos.open} mt={4} data-a-propos />
        </div>
      </AppShell.Navbar>

      {/* Les outils restent montés : on retrouve son travail en changeant d'outil. */}
      <AppShell.Main>
        <BandeauMiseAJour miseAJour={miseAJour} versionActuelle={version} />
        <Box display={vue === "accueil" ? undefined : "none"}>
          <Accueil groupes={GROUPES} recents={recents} onLire={lire} onOutil={setVue} onFichiers={(fichiers) => recevoir.current(fichiers, null)} />
        </Box>
        <Box display={vue === "lecteur" ? undefined : "none"}>
          <Lecteur chemin={lu} actif={vue === "lecteur"} outils={OUTILS_PDF} onOutil={(definition) => lu && confier(definition, [lu])} onChoisir={lire} />
        </Box>
        {OUTILS.map(({ cle, icone: Icone, page: Page, accepte }) => (
          <Box key={cle} display={cle === vue ? undefined : "none"} className="page-outil" style={varsTeinte(GROUPE_DE[cle].teinte)}>
            <Stack gap="lg" maw={1240}>
              <Group justify="space-between" align="center" wrap="wrap" gap="md">
                <Group gap="md" wrap="nowrap" align="center">
                  <span className="pastille pastille-grande">
                    <Icone size={28} stroke={1.7} />
                  </span>
                  <div>
                    <Title order={2}>{textes.outils[cle].titre}</Title>
                    <Text c="dimmed">{textes.outils[cle].sousTitre}</Text>
                  </div>
                </Group>
                {/* Les PDF récents, à un clic : ils arrivent dans l'outil comme par le clic droit. */}
                {accepte.includes("pdf") && recents.length > 0 && (
                  <Group gap={6} wrap="nowrap" data-recents-outil>
                    <Text size="xs" fw={800} c="dimmed" tt="uppercase" style={{ letterSpacing: "0.07em" }}>
                      {tb.recentsOutil}
                    </Text>
                    {recents.slice(0, 3).map(({ chemin }) => (
                      <Tooltip key={chemin} label={tb.ouvrirDansOutil(nomDe(chemin))}>
                        <Button className="puce-recent" size="xs" radius="xl" variant="default" leftSection={<IconFileTypePdf size={14} />} onClick={() => transmettre(cle, [chemin])}>
                          <Text size="xs" truncate="end" inherit>
                            {nomDe(chemin)}
                          </Text>
                        </Button>
                      </Tooltip>
                    ))}
                  </Group>
                )}
              </Group>
              <Page />
            </Stack>
          </Box>
        ))}
      </AppShell.Main>

      {survol && (
        <Box
          pos="fixed"
          inset={12}
          style={{ zIndex: 300, pointerEvents: "none", border: "3px dashed var(--automat-accent-bord)", borderRadius: 12, background: "var(--automat-voile)" }}
          data-survol
        >
          <Stack h="100%" justify="center" align="center">
            <Title order={3}>{vue === "accueil" ? tr.deposerAccueil : vue === "lecteur" ? textes.lecteur.deposerIci : tr.deposer(textes.outils[vue].titre)}</Title>
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
