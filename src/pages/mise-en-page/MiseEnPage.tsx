import { useState } from "react";
import { Box, Group, Paper, SegmentedControl, SimpleGrid, Stack, Text, Title, UnstyledButton } from "@mantine/core";
import { IconLayoutColumns } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { imposerPdf, pagesPdf, type Disposition } from "../../coeur";
import { aCote, nomDe } from "../../donnees/chemins";
import { textes } from "../../textes";
import Action from "../commun/Action";
import BarreAction from "../commun/BarreAction";
import { useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { ZoneFichier } from "../commun/ZoneFichier";

const t = textes.miseEnPage;
const DISPOSITIONS: Disposition[] = ["deux-par-feuille", "quatre-par-feuille", "livret", "format-a4"];
/** Cases des dispositions à plusieurs pages par feuille : on y met les pages à la suite, ou chaque page répétée. */
const CASES: Partial<Record<Disposition, number>> = { "deux-par-feuille": 2, "quatre-par-feuille": 4 };

/** Feuille schématique : ses pages numérotées, posées en grille. */
function Feuille({ paysage, colonnes, pages }: { paysage: boolean; colonnes: number; pages: (number | null)[] }) {
  return (
    <Box
      w={paysage ? 74 : 52}
      h={paysage ? 52 : 74}
      p={4}
      style={{
        display: "grid",
        gridTemplateColumns: `repeat(${colonnes}, 1fr)`,
        gap: 3,
        border: "1px solid var(--mantine-color-default-border)",
        background: "var(--automat-feuille)",
        borderRadius: 2,
      }}
    >
      {pages.map((page, i) => (
        <Box
          key={i}
          style={{
            background: page ? "var(--automat-feuille-page)" : "transparent",
            border: page ? "1px solid var(--automat-feuille-page-bord)" : "1px dashed var(--automat-feuille-vide)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 10,
            color: "var(--automat-feuille-page-texte)",
          }}
        >
          {page}
        </Box>
      ))}
    </Box>
  );
}

function Schema({ disposition, repeter }: { disposition: Disposition; repeter: boolean }) {
  switch (disposition) {
    case "deux-par-feuille":
      return <Feuille paysage colonnes={2} pages={repeter ? [1, 1] : [1, 2]} />;
    case "quatre-par-feuille":
      return <Feuille paysage={false} colonnes={2} pages={repeter ? [1, 1, 1, 1] : [1, 2, 3, 4]} />;
    case "livret":
      return (
        <Group gap={6}>
          <Feuille paysage colonnes={2} pages={[4, 1]} />
          <Feuille paysage colonnes={2} pages={[2, 3]} />
        </Group>
      );
    case "format-a4":
      return <Feuille paysage={false} colonnes={1} pages={[1]} />;
  }
}

/** Nombre de pages du PDF produit ; répétée, chaque page a sa propre feuille. */
function pagesProduites(disposition: Disposition, pages: number, repeter: boolean): number {
  switch (disposition) {
    case "deux-par-feuille":
      return repeter ? pages : Math.ceil(pages / 2);
    case "quatre-par-feuille":
      return repeter ? pages : Math.ceil(pages / 4);
    case "livret":
      return Math.max(1, Math.ceil(pages / 4)) * 2;
    case "format-a4":
      return pages;
  }
}

/** Pages par feuille : 2 ou 4 pages par feuille (à la suite ou répétées), livret à plier, ou pages remises au format A4. */
export default function MiseEnPage() {
  const [source, setSource] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [disposition, setDisposition] = useState<Disposition>("deux-par-feuille");
  // Chaque page répétée sur sa feuille : d'office pour un PDF d'une seule page (une invitation,
  // une carte), dont les autres cases resteraient vides.
  const [repeter, setRepeter] = useState(false);
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
  const cases = CASES[disposition];
  const repetee = repeter && cases !== undefined;
  const produites = source ? pagesProduites(disposition, source.pages, repetee) : 0;

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      const pages = (await pagesPdf(chemin)).length;
      setSource({ chemin, nom: nomDe(chemin), pages });
      setRepeter(pages === 1);
      setCree(null);
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("miseEnPage", (fichiers) => void ouvrir(fichiers[0]));

  async function creer() {
    if (!source) return;
    const base = source.nom.replace(/\.pdf$/i, "");
    const nomDisposition = repetee ? t.exemplaires(cases) : t.dispositions[disposition].titre;
    const sortie = await save({ defaultPath: aCote(source.chemin, t.nomFichier(base, nomDisposition)), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    setCree(null);
    try {
      await imposerPdf(source.chemin, disposition, repetee, sortie);
      setCree({ titre: t.cree, chemin: sortie, details: [disposition === "livret" ? t.livret(produites / 2) : t.resultat(produites)] });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={980}>
      <ZoneFichier
        fichier={source ? { nom: source.nom, details: textes.pdf.pages(source.pages) } : null}
        invite={textes.depot.pdf}
        bouton={t.choisir}
        changer={t.autre}
        onChoisir={() => void choisir()}
      />

      <Title order={5}>{t.disposition}</Title>
      <SimpleGrid cols={{ base: 2, md: 4 }} spacing="sm">
        {DISPOSITIONS.map((valeur) => {
          const choisie = valeur === disposition;
          return (
            <UnstyledButton key={valeur} onClick={() => setDisposition(valeur)} aria-pressed={choisie} data-disposition={valeur}>
              <Paper
                withBorder
                p="md"
                h="100%"
                style={{
                  outline: choisie ? "2px solid var(--automat-accent-bord)" : undefined,
                  background: choisie ? "var(--automat-accent-fond)" : undefined,
                }}
              >
                <Stack gap="xs" align="flex-start">
                  <Box h={80} style={{ display: "flex", alignItems: "center" }}>
                    <Schema disposition={valeur} repeter={repeter && CASES[valeur] !== undefined} />
                  </Box>
                  <Text fw={600} size="sm">
                    {t.dispositions[valeur].titre}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {t.dispositions[valeur].detail}
                  </Text>
                </Stack>
              </Paper>
            </UnstyledButton>
          );
        })}
      </SimpleGrid>

      {cases !== undefined && (
        <Stack gap={4} align="flex-start">
          <Text size="sm" fw={500}>
            {t.remplissage}
          </Text>
          <SegmentedControl
            value={repeter ? "repetee" : "suite"}
            onChange={(valeur) => setRepeter(valeur === "repetee")}
            data={(["suite", "repetee"] as const).map((value) => ({ value, label: t.remplissages[value] }))}
          />
          <Text size="sm" c="dimmed">
            {t.remplissagesAide[repeter ? "repetee" : "suite"](cases)}
          </Text>
        </Stack>
      )}

      <BarreAction>
        <Group>
          <Action icone={IconLayoutColumns} enCours={enCours} raison={source ? null : t.raison} onClick={() => void creer()}>
            {t.creer}
          </Action>
          {source && (
            <Text size="sm" c="dimmed">
              {disposition === "livret" ? t.livret(produites / 2) : t.resultat(produites)}
            </Text>
          )}
        </Group>
      </BarreAction>
      <Resultat cree={cree} onFermer={() => setCree(null)} />
    </Stack>
  );
}
