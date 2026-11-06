import { useState } from "react";
import { Paper, SegmentedControl, Stack, Text, Title } from "@mantine/core";
import { IconArrowsMinimize } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { compresserPdf, tailleFichier } from "../../coeur";
import { aCote, nomDe } from "../../donnees/chemins";
import { formaterTaille } from "../../donnees/tailles";
import { textes } from "../../textes";
import Action from "../commun/Action";
import { useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { ZoneFichier } from "../commun/ZoneFichier";

const t = textes.compression;
type Niveau = "legere" | "moyenne" | "forte";

/** Compresser : un PDF plus léger (images réduites, doublons retirés), à la qualité choisie. */
export default function Compresser() {
  const [source, setSource] = useState<{ chemin: string; nom: string; taille: number } | null>(null);
  const [niveau, setNiveau] = useState<Niveau>("moyenne");
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      setSource({ chemin, nom: nomDe(chemin), taille: await tailleFichier(chemin) });
      setCree(null);
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("compresser", (fichiers) => void ouvrir(fichiers[0]));

  async function compresser() {
    if (!source) return;
    const sortie = await save({ defaultPath: aCote(source.chemin, t.nomFichier(source.nom.replace(/\.pdf$/i, ""))), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    setCree(null);
    try {
      const bilan = await compresserPdf(source.chemin, niveau, sortie);
      const gain = Math.round((1 - bilan.apres / Math.max(1, bilan.avant)) * 100);
      setCree({
        titre: t.cree,
        chemin: sortie,
        details: [bilan.apres < bilan.avant ? t.resultat(formaterTaille(bilan.avant), formaterTaille(bilan.apres), gain, bilan.images) : t.dejaCompact],
      });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={720}>
      <ZoneFichier
        fichier={source ? { nom: source.nom, details: formaterTaille(source.taille) } : null}
        invite={textes.depot.pdf}
        bouton={t.choisir}
        changer={t.autre}
        onChoisir={() => void choisir()}
      />
      {source && (
        <Paper withBorder p="md">
          <Stack gap="sm">
            <Title order={5}>{t.niveau}</Title>
            <SegmentedControl value={niveau} onChange={(v) => setNiveau(v as Niveau)} data={(["legere", "moyenne", "forte"] as const).map((value) => ({ value, label: t.niveaux[value] }))} />
            <Text size="sm" c="dimmed">
              {t.niveauxAide[niveau]}
            </Text>
            <Action icone={IconArrowsMinimize} enCours={enCours} onClick={() => void compresser()}>
              {t.compresser}
            </Action>
          </Stack>
        </Paper>
      )}
      <Resultat cree={cree} onFermer={() => setCree(null)} />
    </Stack>
  );
}
