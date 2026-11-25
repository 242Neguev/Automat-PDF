import { useEffect, useMemo, useState } from "react";
import { ActionIcon, Alert, Box, Button, Center, Checkbox, Group, Loader, Modal, NumberInput, SegmentedControl, Select, Stack, Text, TextInput } from "@mantine/core";
import { IconChevronLeft, IconChevronRight, IconPrinter, IconSettings } from "@tabler/icons-react";
import {
  enErreur,
  feuilleImprimante,
  imprimantes,
  proprietesImprimante,
  type Ajustement,
  type Feuille,
  type FormatPage,
  type Imprimante,
  type OptionsImpression,
  type Orientation,
  type RectoVerso,
} from "../../coeur";
import { feuillesNecessaires, placementApercu } from "../../donnees/impression";
import { lirePlages } from "../../donnees/plages";
import { textes } from "../../textes";
import { imagePage } from "../commun/vignettes";
import { signalerErreur } from "../commun/signalerErreur";

const t = textes.lecteur.impression;

/** Largeur de la feuille dans l'aperçu, en pixels. */
const LARGEUR_APERCU = 300;

/** Imprimantes lues une fois par session (la liste prend du temps avec des imprimantes en réseau). */
let enCache: Imprimante[] | null = null;
/** Derniers choix, repris à la prochaine impression de la session. */
let derniers: Partial<Pick<OptionsImpression, "imprimante" | "couleur" | "rectoVerso" | "ajustement" | "orientation">> = {};

type ChoixPages = "toutes" | "courante" | "certaines";

interface Props {
  ouvert: boolean;
  chemin: string;
  formats: FormatPage[];
  /** Page lue (à partir de 0). */
  courante: number;
  avecAnnotations: boolean;
  onFermer: () => void;
  onImprimer: (options: OptionsImpression) => void;
}

/**
 * Fenêtre d'impression d'Automat-PDF (Windows) : aperçu de la feuille, imprimante, pages, copies,
 * couleur ou noir et blanc, recto verso, taille, orientation, papier, et les réglages du pilote à un
 * clic. Celle de Windows n'a pas d'aperçu, et son réglage de couleur change d'un pilote à l'autre.
 */
export default function FenetreImpression({ ouvert, chemin, formats, courante, avecAnnotations, onFermer, onImprimer }: Props) {
  const [liste, setListe] = useState<Imprimante[] | null>(enCache);
  const [erreurListe, setErreurListe] = useState<string | null>(null);
  const [nom, setNom] = useState("");
  const [pilote, setPilote] = useState<number[]>([]);
  const [couleur, setCouleur] = useState(true);
  const [rectoVerso, setRectoVerso] = useState<RectoVerso>("non");
  const [ajustement, setAjustement] = useState<Ajustement>("reduire");
  const [orientation, setOrientation] = useState<Orientation>("auto");
  const [papier, setPapier] = useState<number | null>(null);
  const [copies, setCopies] = useState<number>(1);
  const [assembler, setAssembler] = useState(true);
  const [choixPages, setChoixPages] = useState<ChoixPages>("toutes");
  const [plages, setPlages] = useState("");
  const [feuille, setFeuille] = useState<Feuille | null>(null);
  const [rangApercu, setRangApercu] = useState(0);
  const [proprietesOuvertes, setProprietesOuvertes] = useState(false);

  const imprimante = liste?.find((i) => i.nom === nom) ?? null;

  // À l'ouverture : la liste des imprimantes (une fois par session), et les derniers choix.
  useEffect(() => {
    if (!ouvert) return;
    setChoixPages("toutes");
    setPlages("");
    setCopies(1);
    if (enCache) {
      choisirImprimante(enCache, derniers.imprimante);
      return;
    }
    let actuel = true;
    setErreurListe(null);
    imprimantes().then(
      (lues) => {
        if (!actuel) return;
        enCache = lues;
        setListe(lues);
        choisirImprimante(lues, derniers.imprimante);
      },
      (e) => actuel && setErreurListe(enErreur(e).message),
    );
    return () => {
      actuel = false;
    };
  }, [ouvert]);

  /** Choisit une imprimante et reprend ses réglages (ou les derniers choix, s'ils la concernent). */
  function choisirImprimante(lues: Imprimante[], voulue?: string) {
    const choisie = lues.find((i) => i.nom === voulue) ?? lues.find((i) => i.parDefaut) ?? lues[0];
    if (!choisie) return;
    setNom(choisie.nom);
    setPilote([]);
    const memes = derniers.imprimante === choisie.nom;
    setCouleur(choisie.couleur && (memes && derniers.couleur !== undefined ? derniers.couleur : choisie.reglages.couleur));
    setRectoVerso(choisie.rectoVerso ? (memes && derniers.rectoVerso ? derniers.rectoVerso : choisie.reglages.rectoVerso) : "non");
    setPapier(choisie.reglages.papier ?? choisie.papiers.find((p) => p.code === 9)?.code ?? choisie.papiers[0]?.code ?? null);
    setAjustement(derniers.ajustement ?? "reduire");
    setOrientation(derniers.orientation ?? "auto");
  }

  const total = formats.length;
  const pagesLues = useMemo(() => {
    if (choixPages === "courante") return { pages: [Math.min(courante, total - 1)] };
    if (choixPages === "certaines") return lirePlages(plages, total);
    return { pages: Array.from({ length: total }, (_, i) => i) };
  }, [choixPages, plages, courante, total]);
  const pages = "pages" in pagesLues ? pagesLues.pages : [];
  const erreurPages = "erreur" in pagesLues && choixPages === "certaines" && plages.trim() !== "" ? pagesLues.erreur : null;

  const options: OptionsImpression | null = imprimante
    ? {
        imprimante: imprimante.nom,
        pages: choixPages === "toutes" ? [] : pages,
        copies: Math.max(1, Math.min(999, Math.round(copies) || 1)),
        assembler,
        couleur,
        rectoVerso,
        ajustement,
        orientation,
        papier,
        pilote,
      }
    : null;

  // La feuille (taille, marges) telle que l'imprimante la prendra : pour l'aperçu.
  const cleFeuille = options ? JSON.stringify([options.imprimante, options.papier, options.orientation === "paysage", options.pilote.length]) : "";
  useEffect(() => {
    if (!ouvert || !options) return;
    let actuel = true;
    feuilleImprimante(options).then(
      (lue) => actuel && setFeuille(lue),
      () => {
        if (!actuel) return;
        // Sans réponse du pilote : le papier choisi, avec des marges courantes de 5 mm.
        const choisi = imprimante?.papiers.find((p) => p.code === papier);
        const [l, h] = choisi ? [choisi.largeur, choisi.hauteur] : [210, 297];
        setFeuille(orientation === "paysage" ? { largeur: Math.max(l, h), hauteur: Math.min(l, h), marges: [5, 5, 5, 5] } : { largeur: Math.min(l, h), hauteur: Math.max(l, h), marges: [5, 5, 5, 5] });
      },
    );
    return () => {
      actuel = false;
    };
  }, [ouvert, cleFeuille]);

  useEffect(() => setRangApercu(0), [choixPages, plages]);
  const pageApercu = pages[Math.min(rangApercu, pages.length - 1)] ?? null;

  async function ouvrirProprietes() {
    if (!imprimante) return;
    setProprietesOuvertes(true);
    try {
      const reglages = await proprietesImprimante(imprimante.nom, pilote);
      if (!reglages) return;
      setPilote(reglages.pilote);
      setCouleur(imprimante.couleur && reglages.couleur);
      if (imprimante.rectoVerso) setRectoVerso(reglages.rectoVerso);
      if (reglages.papier !== null) setPapier(reglages.papier);
      if (reglages.paysage) setOrientation("paysage");
    } catch (e) {
      signalerErreur(e);
    } finally {
      setProprietesOuvertes(false);
    }
  }

  function imprimer() {
    if (!options || pages.length === 0) return;
    derniers = { imprimante: options.imprimante, couleur, rectoVerso, ajustement, orientation };
    onImprimer(options);
  }

  const feuilles = feuillesNecessaires(pages.length, copies, rectoVerso !== "non", assembler);

  return (
    <Modal opened={ouvert} onClose={onFermer} title={t.titre} size={860} centered data-fenetre-impression>
      {erreurListe ? (
        <Alert color="red">{erreurListe}</Alert>
      ) : !liste ? (
        <Center h={240}>
          <Stack align="center" gap="xs">
            <Loader size="sm" color="gray" />
            <Text size="sm" c="dimmed">
              {t.chargement}
            </Text>
          </Stack>
        </Center>
      ) : liste.length === 0 ? (
        <Alert color="orange">{t.aucune}</Alert>
      ) : (
        <Group align="flex-start" gap="xl" wrap="nowrap">
          <Stack gap="sm" style={{ flex: 1, minWidth: 0 }}>
            <Group gap="xs" align="flex-end" wrap="nowrap">
              <Select
                label={t.imprimante}
                data={liste.map((i) => ({ value: i.nom, label: i.parDefaut ? `${i.nom}${t.parDefaut}` : i.nom }))}
                value={nom}
                onChange={(valeur) => valeur && choisirImprimante(liste, valeur)}
                allowDeselect={false}
                style={{ flex: 1, minWidth: 0 }}
              />
              <ActionIcon variant="default" size={36} onClick={() => void ouvrirProprietes()} loading={proprietesOuvertes} aria-label={t.proprietes} title={t.proprietes}>
                <IconSettings size={18} />
              </ActionIcon>
            </Group>

            <Stack gap={6}>
              <Text size="sm" fw={500}>
                {t.pages}
              </Text>
              <SegmentedControl
                value={choixPages}
                onChange={(valeur) => setChoixPages(valeur as ChoixPages)}
                data={[
                  { value: "toutes", label: t.toutes(total) },
                  { value: "courante", label: t.courante(Math.min(courante, total - 1) + 1) },
                  { value: "certaines", label: t.certaines },
                ]}
              />
              {choixPages === "certaines" && <TextInput placeholder={t.plagesAide} value={plages} onChange={(e) => setPlages(e.currentTarget.value)} error={erreurPages} autoFocus />}
            </Stack>

            <Group gap="md" align="flex-end">
              <NumberInput label={t.copies} value={copies} onChange={(valeur) => setCopies(typeof valeur === "number" ? valeur : 1)} min={1} max={999} w={110} />
              {copies > 1 && <Checkbox label={t.assembler} checked={assembler} onChange={(e) => setAssembler(e.currentTarget.checked)} mb={8} />}
            </Group>

            <Stack gap={6}>
              <Text size="sm" fw={500}>
                {t.couleur}
              </Text>
              <SegmentedControl
                value={couleur ? "couleur" : "gris"}
                onChange={(valeur) => setCouleur(valeur === "couleur")}
                disabled={!imprimante?.couleur}
                data={[
                  { value: "couleur", label: t.enCouleur },
                  { value: "gris", label: t.noirEtBlanc },
                ]}
              />
              {imprimante && !imprimante.couleur && (
                <Text size="xs" c="dimmed">
                  {t.sansCouleur}
                </Text>
              )}
            </Stack>

            {imprimante?.rectoVerso && (
              <Select
                label={t.rectoVerso}
                value={rectoVerso}
                onChange={(valeur) => valeur && setRectoVerso(valeur as RectoVerso)}
                allowDeselect={false}
                data={[
                  { value: "non", label: t.rectoVersoNon },
                  { value: "bordLong", label: t.bordLong },
                  { value: "bordCourt", label: t.bordCourt },
                ]}
              />
            )}

            <Group gap="sm" grow align="flex-start">
              <Select
                label={t.taille}
                value={ajustement}
                onChange={(valeur) => valeur && setAjustement(valeur as Ajustement)}
                allowDeselect={false}
                data={[
                  { value: "reduire", label: t.reduire },
                  { value: "reelle", label: t.reelle },
                  { value: "ajuster", label: t.ajuster },
                ]}
              />
              <Select
                label={t.orientation}
                value={orientation}
                onChange={(valeur) => valeur && setOrientation(valeur as Orientation)}
                allowDeselect={false}
                data={[
                  { value: "auto", label: t.auto },
                  { value: "portrait", label: t.portrait },
                  { value: "paysage", label: t.paysage },
                ]}
              />
            </Group>

            {imprimante && imprimante.papiers.length > 0 && (
              <Select
                label={t.papier}
                value={papier === null ? null : String(papier)}
                onChange={(valeur) => valeur && setPapier(Number(valeur))}
                allowDeselect={false}
                searchable
                data={imprimante.papiers.map((p) => ({ value: String(p.code), label: `${p.nom} (${Math.round(p.largeur)} × ${Math.round(p.hauteur)} mm)` }))}
              />
            )}
          </Stack>

          <Stack gap="xs" align="center" w={LARGEUR_APERCU + 20} style={{ flexShrink: 0 }}>
            <Text size="sm" fw={500} style={{ alignSelf: "flex-start" }}>
              {t.apercu}
            </Text>
            <Apercu chemin={chemin} page={pageApercu} format={pageApercu === null ? null : formats[pageApercu]} feuille={feuille} ajustement={ajustement} orientation={orientation} couleur={couleur} />
            <Group gap={4} justify="center">
              <ActionIcon variant="subtle" color="gray" onClick={() => setRangApercu((r) => Math.max(0, r - 1))} disabled={rangApercu <= 0} aria-label={t.precedente}>
                <IconChevronLeft size={16} />
              </ActionIcon>
              <Text size="sm" c="dimmed" data-page-apercu>
                {pages.length > 0 ? t.pageApercu(Math.min(rangApercu, pages.length - 1) + 1, pages.length) : "—"}
              </Text>
              <ActionIcon variant="subtle" color="gray" onClick={() => setRangApercu((r) => Math.min(pages.length - 1, r + 1))} disabled={rangApercu >= pages.length - 1} aria-label={t.suivante}>
                <IconChevronRight size={16} />
              </ActionIcon>
            </Group>
            {avecAnnotations && (
              <Text size="xs" c="dimmed" ta="center">
                {t.annotations}
              </Text>
            )}
          </Stack>
        </Group>
      )}

      <Group justify="space-between" mt="lg">
        <Text size="sm" c="dimmed">
          {pages.length > 0 ? t.feuilles(feuilles) : ""}
        </Text>
        <Group gap="sm">
          <Button variant="default" onClick={onFermer}>
            {t.annuler}
          </Button>
          <Button leftSection={<IconPrinter size={16} />} onClick={imprimer} disabled={!options || pages.length === 0} data-lancer-impression>
            {t.imprimer}
          </Button>
        </Group>
      </Group>
    </Modal>
  );
}

/** La feuille et la page dessus, à l'échelle, comme elle sortira de l'imprimante. */
function Apercu({ chemin, page, format, feuille, ajustement, orientation, couleur }: { chemin: string; page: number | null; format: FormatPage | null; feuille: Feuille | null; ajustement: Ajustement; orientation: Orientation; couleur: boolean }) {
  const [image, setImage] = useState<string | null>(null);
  useEffect(() => {
    setImage(null);
    if (page === null) return;
    let actuelle = true;
    imagePage(chemin, page, 600).then(
      (adresse) => actuelle && setImage(adresse),
      () => undefined,
    );
    return () => {
      actuelle = false;
    };
  }, [chemin, page]);

  if (!feuille || !format) {
    return (
      <Center w={LARGEUR_APERCU} h={LARGEUR_APERCU * 1.41}>
        <Loader size="sm" color="gray" />
      </Center>
    );
  }
  const echelle = LARGEUR_APERCU / feuille.largeur;
  const hauteur = feuille.hauteur * echelle;
  const p = placementApercu(format, feuille, ajustement, orientation);
  const [gauche, haut, droite, bas] = feuille.marges.map((m) => m * echelle);
  const boite = { left: p.x * echelle, top: p.y * echelle, width: p.largeur * echelle, height: p.hauteur * echelle };
  return (
    <Box w={LARGEUR_APERCU} h={hauteur} pos="relative" style={{ background: "white", boxShadow: "var(--automat-ombre-page)", overflow: "hidden" }} data-apercu-impression>
      <Box pos="absolute" style={{ left: gauche, top: haut, right: droite, bottom: bas, outline: "1px dashed rgba(0,0,0,0.15)" }} />
      <Box pos="absolute" style={{ ...boite, overflow: "hidden" }}>
        {image && (
          <img
            src={image}
            alt=""
            draggable={false}
            style={{
              position: "absolute",
              left: "50%",
              top: "50%",
              width: p.tournee ? boite.height : boite.width,
              height: p.tournee ? boite.width : boite.height,
              transform: `translate(-50%, -50%)${p.tournee ? " rotate(90deg)" : ""}`,
              filter: couleur ? undefined : "grayscale(1)",
            }}
          />
        )}
      </Box>
    </Box>
  );
}
