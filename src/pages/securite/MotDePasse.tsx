import { useState } from "react";
import { Button, Group, Paper, PasswordInput, SegmentedControl, Stack, Switch, Text, Title } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCheck, IconFileSearch, IconLock, IconLockOpen } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { deverrouillerPdf, protegerPdf } from "../../coeur";
import { textes } from "../../textes";
import { useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";

const t = textes.motDePasse;

/** Mot de passe : protéger un PDF (AES-256, droits choisis), ou retirer son mot de passe. */
export default function MotDePasse() {
  const [mode, setMode] = useState<"proteger" | "retirer">("proteger");
  const [source, setSource] = useState<{ chemin: string; nom: string } | null>(null);
  const [motDePasse, setMotDePasse] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [proprietaire, setProprietaire] = useState("");
  const [droits, setDroits] = useState({ impression: true, copie: false, modification: false });
  const [actuel, setActuel] = useState("");
  const [enCours, setEnCours] = useState(false);
  const differents = confirmation !== "" && confirmation !== motDePasse;
  const base = source?.nom.replace(/\.pdf$/i, "") ?? "";

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") ouvrir(chemin);
  }

  const ouvrir = (chemin: string) => setSource({ chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin });
  useFichiersRecus("motDePasse", (fichiers) => ouvrir(fichiers[0]));

  async function enregistrer(nom: string, action: (sortie: string) => Promise<void>, titre: string) {
    const sortie = await save({ defaultPath: nom, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    try {
      await action(sortie);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: titre, message: sortie });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={640}>
      <SegmentedControl value={mode} onChange={(v) => setMode(v as typeof mode)} data={(["proteger", "retirer"] as const).map((value) => ({ value, label: t.modes[value] }))} />
      <Group>
        <Button variant={source ? "light" : "filled"} leftSection={<IconFileSearch size={18} />} onClick={choisir}>
          {source ? t.autre : t.choisir}
        </Button>
        {source && <Text size="sm">{source.nom}</Text>}
      </Group>

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
            <Button
              leftSection={<IconLock size={18} />}
              loading={enCours}
              disabled={!motDePasse || motDePasse !== confirmation}
              onClick={() =>
                enregistrer(t.nomProtege(base), (sortie) => protegerPdf(source.chemin, { motDePasse, proprietaire, ...droits }, sortie), t.protege)
              }
              w="fit-content"
            >
              {t.proteger}
            </Button>
          </Stack>
        </Paper>
      )}

      {source && mode === "retirer" && (
        <Paper withBorder p="md">
          <Stack gap="sm">
            <PasswordInput label={t.motDePasseActuel} value={actuel} onChange={(e) => setActuel(e.currentTarget.value)} />
            <Text size="xs" c="dimmed">
              {t.retirerAide}
            </Text>
            <Button
              leftSection={<IconLockOpen size={18} />}
              loading={enCours}
              onClick={() => enregistrer(t.nomLibre(base), (sortie) => deverrouillerPdf(source.chemin, actuel, sortie), t.libere)}
              w="fit-content"
            >
              {t.retirer}
            </Button>
          </Stack>
        </Paper>
      )}
    </Stack>
  );
}
