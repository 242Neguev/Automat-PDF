import { useState } from "react";
import { Box, Group, Paper, SimpleGrid, Stack, Text, Title, UnstyledButton } from "@mantine/core";
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
        border: "1px solid var(--mantine-color-gray-4)",
        background: "white",
        borderRadius: 2,
      }}
    >
      {pages.map((page, i) => (
        <Box
          key={i}
          style={{
            background: page ? "var(--mantine-color-eau-1)" : "transparent",
            border: page ? "1px solid var(--mantine-color-eau-4)" : "1px dashed var(--mantine-color-gray-3)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 10,
            color: "var(--mantine-color-eau-8)",
          }}
        >
          {page}
        </Box>
      ))}
    </Box>
  );
}

function Schema({ disposition }: { disposition: Disposition }) {
  switch (disposition) {
    case "deux-par-feuille":
      return <Feuille paysage colonnes={2} pages={[1, 2]} />;
    case "quatre-par-feuille":
      return <Feuille paysage={false} colonnes={2} pages={[1, 2, 3, 4]} />;
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

/** Nombre de pages du PDF produit. */
function pagesProduites(disposition: Disposition, pages: number): number {
  switch (disposition) {
    case "deux-par-feuille":
      return Math.ceil(pages / 2);
    case "quatre-par-feuille":
      return Math.ceil(pages / 4);
    case "livret":
      return Math.max(1, Math.ceil(pages / 4)) * 2;
    case "format-a4":
      return pages;
  }
}

/** Pages par feuille : 2 ou 4 pages par feuille, livret à plier, ou pages remises au format A4. */
export default function MiseEnPage() {
  const [source, setSource] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [disposition, setDisposition] = useState<Disposition>("deux-par-feuille");
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
  const produites = source ? pagesProduites(disposition, source.pages) : 0;

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      setSource({ chemin, nom: nomDe(chemin), pages: (await pagesPdf(chemin)).length });
      setCree(null);
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("miseEnPage", (fichiers) => void ouvrir(fichiers[0]));

  async function creer() {
    if (!source) return;
    const base = source.nom.replace(/\.pdf$/i, "");
    const sortie = await save({ defaultPath: aCote(source.chemin, t.nomFichier(base, t.dispositions[disposition].titre)), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    setCree(null);
    try {
      await imposerPdf(source.chemin, disposition, sortie);
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
                  outline: choisie ? "2px solid var(--mantine-color-eau-6)" : undefined,
                  background: choisie ? "var(--mantine-color-eau-0)" : undefined,
                }}
              >
                <Stack gap="xs" align="flex-start">
                  <Box h={80} style={{ display: "flex", alignItems: "center" }}>
                    <Schema disposition={valeur} />
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
