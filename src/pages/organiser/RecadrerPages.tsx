import { useEffect, useState } from "react";
import { ActionIcon, Box, Button, Group, Loader, Modal, SegmentedControl, Stack, Text, Tooltip } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconChevronLeft, IconChevronRight, IconCrop, IconEraser, IconFocus2, IconLayoutGrid } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { bordsDuContenu, pagesPdf, recadrerPdf, type FormatPage, type ZoneFractions } from "../../coeur";
import { enFractions, tailleEnCm, type ZonePoints } from "../../donnees/capture";
import { aCote, nomDe } from "../../donnees/chemins";
import { ecrirePlages } from "../../donnees/plages";
import { textes } from "../../textes";
import Action from "../commun/Action";
import BarreAction from "../commun/BarreAction";
import ChoixPages from "../commun/ChoixPages";
import Etape from "../commun/Etape";
import { dela, pageConfiee, useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { useTailleFenetre } from "../commun/useTailleFenetre";
import { imagePage, oublier } from "../commun/vignettes";
import { ZoneFichier } from "../commun/ZoneFichier";
import { CoucheCapture } from "../lecteur/Capture";

const t = textes.recadrage;

/** Le cadre (fractions de la page) en points d'une page de ce format. */
function enPoints(cadre: ZoneFractions, format: FormatPage): ZonePoints {
  return { x: cadre.x * format.largeur, y: cadre.y * format.hauteur, largeur: cadre.largeur * format.largeur, hauteur: cadre.hauteur * format.hauteur };
}

interface PropsPage {
  chemin: string;
  page: number;
  format: FormatPage;
  /** Côté du carré où la page s'inscrit, en pixels. */
  cote: number;
  cadre: ZoneFractions | null;
  onCadre: (cadre: ZoneFractions | null) => void;
  /** Faux : le cadre est seulement montré (chaque page à son contenu). */
  actif: boolean;
}

/** La page en grand et le cadre qu'on y trace, le même pour toutes les pages ; autour, elle s'assombrit. */
function PageACadrer({ chemin, page, format, cote, cadre, onCadre, actif }: PropsPage) {
  const echelle = Math.min(cote / format.largeur, cote / format.hauteur);
  const [largeur, hauteur] = [Math.round(format.largeur * echelle), Math.round(format.hauteur * echelle)];
  const finesse = Math.min(2, window.devicePixelRatio || 1);
  // Par paliers de 200 pixels, comme le lecteur : les images déjà dessinées resservent.
  const demandee = Math.min(2400, Math.max(200, Math.ceil((largeur * finesse) / 200) * 200));
  const [adresse, setAdresse] = useState<string | null>(null);
  useEffect(() => {
    let actuelle = true;
    setAdresse(null);
    imagePage(chemin, page, demandee).then(
      (lue) => actuelle && setAdresse(lue),
      () => undefined,
    );
    return () => {
      actuelle = false;
    };
  }, [chemin, page, demandee]);
  return (
    <Box pos="relative" w={largeur} h={hauteur} style={{ background: "white", boxShadow: "var(--automat-ombre-page)", flexShrink: 0 }} data-page-a-cadrer={page + 1}>
      {adresse ? (
        <img src={adresse} alt={textes.choixPages.alt(page + 1)} draggable={false} style={{ width: "100%", height: "100%", display: "block", userSelect: "none" }} />
      ) : (
        <Loader size="sm" color="gray" pos="absolute" top="50%" left="50%" />
      )}
      <CoucheCapture
        page={page}
        format={format}
        echelle={echelle}
        rotation={0}
        zone={cadre ? enPoints(cadre, format) : null}
        onZone={(_, zone) => onCadre(zone ? enFractions(zone, format) : null)}
        actif={actif}
      />
    </Box>
  );
}

/**
 * Recadrer les pages : on trace sur une page le cadre à garder (ou on l'ajuste au contenu), il vaut
 * pour toutes les pages choisies ; ce qui est autour est retiré, dans un nouveau PDF.
 */
export default function RecadrerPages() {
  const [source, setSource] = useState<{ chemin: string; nom: string; formats: FormatPage[] } | null>(null);
  const [vue, setVue] = useState(0);
  const [cadre, setCadre] = useState<ZoneFractions | null>(null);
  /** Un cadre pour toutes les pages, ou chaque page à son contenu (ses marges retirées). */
  const [mode, setMode] = useState<"cadre" | "contenu">("cadre");
  /** Chaque page à son contenu : le cadre de chaque page vue (null : page blanche). */
  const [bords, setBords] = useState<Record<number, ZoneFractions | null>>({});
  const [choix, setChoix] = useState<"toutes" | "certaines">("toutes");
  const [choisies, setChoisies] = useState<number[]>([]);
  const [vueChoix, setVueChoix] = useState(0);
  const [choixOuvert, setChoixOuvert] = useState(false);
  const [ajustement, setAjustement] = useState(false);
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
  const fenetre = useTailleFenetre();
  const cote = Math.round(Math.max(260, Math.min(640, fenetre.hauteur - 280, fenetre.largeur * 0.5)));

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  /** Ouvre un PDF ; `page` : celle que montrait le lecteur, montrée d'abord. */
  async function ouvrir(chemin: string, page?: number) {
    try {
      const formats = await pagesPdf(chemin);
      const depart = page !== undefined && page < formats.length ? page : null;
      setSource({ chemin, nom: nomDe(chemin), formats });
      setVue(depart ?? 0);
      setVueChoix(depart ?? 0);
      setChoisies(depart === null ? [] : [depart]);
      setChoix("toutes");
      setCadre(null);
      setBords({});
      setCree(null);
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("recadrer", (fichiers) => {
    const [pdf] = dela(fichiers, "pdf");
    if (pdf) void ouvrir(pdf, pageConfiee(pdf));
  });

  // Chaque page à son contenu : le cadre de la page vue, demandé une fois.
  useEffect(() => {
    if (!source || mode !== "contenu" || vue in bords) return;
    let actuel = true;
    bordsDuContenu(source.chemin, vue).then(
      (lus) => actuel && setBords((avant) => ({ ...avant, [vue]: lus })),
      () => actuel && setBords((avant) => ({ ...avant, [vue]: null })),
    );
    return () => {
      actuel = false;
    };
  }, [source, mode, vue, bords]);

  async function ajuster() {
    if (!source) return;
    setAjustement(true);
    try {
      const bords = await bordsDuContenu(source.chemin, vue);
      if (bords) setCadre(bords);
      else notifications.show({ color: "orange", title: t.pageBlanche, message: "", autoClose: 6000 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setAjustement(false);
    }
  }

  async function recadrer() {
    if (!source || (mode === "cadre" && !cadre)) return;
    const pages = choix === "toutes" ? [] : choisies;
    if (choix === "certaines" && pages.length === 0) return;
    const sortie = await save({ defaultPath: aCote(source.chemin, t.nom(source.nom.replace(/\.pdf$/i, ""))), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    setCree(null);
    try {
      const bilan = await recadrerPdf(source.chemin, pages, mode === "cadre" ? cadre : null, sortie);
      oublier(sortie);
      const nombre = (pages.length || source.formats.length) - bilan.pagesBlanches.length;
      const blanches = bilan.pagesBlanches.length > 0 ? [t.blanches(bilan.pagesBlanches)] : [];
      const cachees = bilan.pagesCachees.length > 0 ? [t.cachees(bilan.pagesCachees)] : [];
      setCree({ titre: t.enregistre, chemin: sortie, details: [t.pages(nombre), ...blanches, ...cachees], attention: cachees.length > 0 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  const format = source?.formats[vue];
  const raison = mode === "cadre" && !cadre ? t.raisonCadre : choix === "certaines" && choisies.length === 0 ? t.raisonPages : null;
  const montre = mode === "cadre" ? cadre : (bords[vue] ?? null);
  return (
    <Stack gap="md">
      <Etape numero={1} titre={t.etapes.pdf}>
        <ZoneFichier
          fichier={source ? { nom: source.nom, details: textes.pdf.pages(source.formats.length) } : null}
          invite={textes.depot.pdf}
          bouton={t.choisir}
          changer={t.autre}
          onChoisir={() => void choisir()}
        />
      </Etape>

      {source && format && (
        <>
          <Etape numero={2} titre={t.etapes.cadre}>
            <SegmentedControl
              value={mode}
              onChange={(valeur) => setMode(valeur as typeof mode)}
              data={(["cadre", "contenu"] as const).map((value) => ({ value, label: t.modes[value] }))}
              style={{ alignSelf: "flex-start" }}
              data-mode-recadrage
            />
            <Text size="sm" c="dimmed">
              {mode === "cadre" ? t.consigne : t.consigneContenu}
            </Text>
            {mode === "contenu" ? (
              <Text size="sm" fw={500} data-taille-cadre>
                {vue in bords ? (bords[vue] ? t.taille(tailleEnCm(enPoints(bords[vue], format))) : t.pageBlanche) : " "}
              </Text>
            ) : (
              <Group gap="xs" align="center">
                <Tooltip label={t.ajusterAide} multiline w={280} withArrow>
                  <Button variant="light" leftSection={<IconFocus2 size={16} />} loading={ajustement} onClick={() => void ajuster()} data-ajuster-contenu>
                    {t.ajuster}
                  </Button>
                </Tooltip>
                <Button variant="subtle" color="gray" leftSection={<IconEraser size={16} />} disabled={!cadre} onClick={() => setCadre(null)}>
                  {t.effacer}
                </Button>
                <Text size="sm" fw={500} data-taille-cadre>
                  {cadre ? t.taille(tailleEnCm(enPoints(cadre, format))) : t.sansCadre}
                </Text>
              </Group>
            )}
            <Group gap="md" wrap="nowrap" align="center">
              <Tooltip label={t.precedente} withArrow>
                <ActionIcon variant="light" size="lg" aria-label={t.precedente} disabled={vue === 0} onClick={() => setVue(vue - 1)}>
                  <IconChevronLeft size={20} />
                </ActionIcon>
              </Tooltip>
              <PageACadrer chemin={source.chemin} page={vue} format={format} cote={cote} cadre={montre} onCadre={setCadre} actif={mode === "cadre"} />
              <Tooltip label={t.suivante} withArrow>
                <ActionIcon variant="light" size="lg" aria-label={t.suivante} disabled={vue >= source.formats.length - 1} onClick={() => setVue(vue + 1)}>
                  <IconChevronRight size={20} />
                </ActionIcon>
              </Tooltip>
            </Group>
            <Text size="sm" c="dimmed" data-position-recadrage>
              {t.position(vue + 1, source.formats.length)}
            </Text>
          </Etape>

          <Etape numero={3} titre={t.etapes.pages}>
            <SegmentedControl
              value={choix}
              onChange={(valeur) => setChoix(valeur as typeof choix)}
              data={(["toutes", "certaines"] as const).map((value) => ({ value, label: t[value] }))}
              style={{ alignSelf: "flex-start" }}
            />
            {choix === "certaines" && (
              <Group gap="sm" align="center">
                <Text size="sm" data-pages-recadrer>
                  {choisies.length > 0 ? t.choisies(ecrirePlages(choisies)) : t.aucune}
                </Text>
                <Button size="xs" variant="light" leftSection={<IconLayoutGrid size={14} />} onClick={() => setChoixOuvert(true)}>
                  {t.voirPages}
                </Button>
              </Group>
            )}
          </Etape>

          <Etape numero={4} titre={t.etapes.recadrer}>
            <BarreAction>
              <Action icone={IconCrop} enCours={enCours} raison={raison} onClick={() => void recadrer()}>
                {t.recadrer}
              </Action>
            </BarreAction>
            <Resultat cree={cree} onFermer={() => setCree(null)} />
          </Etape>

          <Modal opened={choixOuvert} onClose={() => setChoixOuvert(false)} size="90%" title={t.titreChoix}>
            <ChoixPages chemin={source.chemin} total={source.formats.length} choisies={choisies} onChoisies={setChoisies} vue={vueChoix} onVue={setVueChoix} />
            <Group justify="flex-end" mt="md">
              <Button onClick={() => setChoixOuvert(false)}>{t.valider}</Button>
            </Group>
          </Modal>
        </>
      )}
    </Stack>
  );
}
