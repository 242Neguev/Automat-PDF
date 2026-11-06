import { useState } from "react";
import { Paper, PasswordInput, SegmentedControl, Stack, Switch, Text, Title } from "@mantine/core";
import { IconLock, IconLockOpen } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { deverrouillerPdf, protegerPdf } from "../../coeur";
import { aCote, nomDe } from "../../donnees/chemins";
import { textes } from "../../textes";
import Action from "../commun/Action";
import { useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { ZoneFichier } from "../commun/ZoneFichier";

const t = textes.motDePasse;

/** Mot de passe : protéger un PDF (chiffrement AES-256, droits choisis), ou retirer son mot de passe. */
export default function MotDePasse() {
  const [mode, setMode] = useState<"proteger" | "retirer">("proteger");
  const [source, setSource] = useState<{ chemin: string; nom: string } | null>(null);
  const [motDePasse, setMotDePasse] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [proprietaire, setProprietaire] = useState("");
  const [droits, setDroits] = useState({ impression: true, copie: false, modification: false });
  const [actuel, setActuel] = useState("");
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
  const differents = confirmation !== "" && confirmation !== motDePasse;
  const base = source?.nom.replace(/\.pdf$/i, "") ?? "";
  const raison = !motDePasse ? t.raisons.motDePasse : !confirmation ? t.raisons.confirmation : differents ? t.raisons.differents : null;

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") ouvrir(chemin);
  }

  function ouvrir(chemin: string) {
    setSource({ chemin, nom: nomDe(chemin) });
    setCree(null);
  }
  useFichiersRecus("motDePasse", (fichiers) => ouvrir(fichiers[0]));

  async function enregistrer(nom: string, action: (sortie: string) => Promise<void>, titre: string) {
    if (!source) return;
    const sortie = await save({ defaultPath: aCote(source.chemin, nom), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    setCree(null);
    try {
      await action(sortie);
      setCree({ titre, chemin: sortie });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={720}>
      <SegmentedControl value={mode} onChange={(v) => setMode(v as typeof mode)} data={(["proteger", "retirer"] as const).map((value) => ({ value, label: t.modes[value] }))} />
      <ZoneFichier fichier={source} invite={textes.depot.pdf} bouton={t.choisir} changer={t.autre} onChoisir={() => void choisir()} />

      {source && mode === "proteger" && (
        <Paper withBorder p="md">
          <Stack gap="sm">
            <PasswordInput label={t.motDePasse} description={t.motDePasseAide} value={motDePasse} onChange={(e) => setMotDePasse(e.currentTarget.value)} />
            <PasswordInput label={t.confirmation} value={confirmation} onChange={(e) => setConfirmation(e.currentTarget.value)} error={differents ? t.differents : undefined} />
            <Title order={6} mt="xs">
              {t.droits}
            </Title>
            <Switch label={t.impression} checked={droits.impression} onChange={(e) => setDroits({ ...droits, impression: e.currentTarget.checked })} />
            <Switch label={t.copie} checked={droits.copie} onChange={(e) => setDroits({ ...droits, copie: e.currentTarget.checked })} />
            <Switch label={t.modification} checked={droits.modification} onChange={(e) => setDroits({ ...droits, modification: e.currentTarget.checked })} />
            <PasswordInput label={t.proprietaire} description={t.proprietaireAide} value={proprietaire} onChange={(e) => setProprietaire(e.currentTarget.value)} mt="xs" />
            <Action
              icone={IconLock}
              enCours={enCours}
              raison={raison}
              onClick={() => void enregistrer(t.nomProtege(base), (sortie) => protegerPdf(source.chemin, { motDePasse, proprietaire, ...droits }, sortie), t.protege)}
            >
              {t.proteger}
            </Action>
          </Stack>
        </Paper>
      )}

      {source && mode === "retirer" && (
        <Paper withBorder p="md">
          <Stack gap="sm">
            <PasswordInput label={t.motDePasseActuel} value={actuel} onChange={(e) => setActuel(e.currentTarget.value)} />
            <Text size="sm" c="dimmed">
              {t.retirerAide}
            </Text>
            <Action icone={IconLockOpen} enCours={enCours} onClick={() => void enregistrer(t.nomLibre(base), (sortie) => deverrouillerPdf(source.chemin, actuel, sortie), t.libere)}>
              {t.retirer}
            </Action>
          </Stack>
        </Paper>
      )}
      <Resultat cree={cree} onFermer={() => setCree(null)} />
    </Stack>
  );
}
