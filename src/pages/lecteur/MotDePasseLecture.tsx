import { useState, type FormEvent } from "react";
import { Button, Center, Paper, PasswordInput, Stack, Text, ThemeIcon, Title } from "@mantine/core";
import { IconLock } from "@tabler/icons-react";
import { deverrouillerLecture, enErreur } from "../../coeur";
import { textes } from "../../textes";

const t = textes.lecteur.motDePasse;

/**
 * Un PDF qui demande son mot de passe d'ouverture : le lecteur le demande, puis le cœur le
 * déchiffre en mémoire seulement (rien n'est écrit sur le disque) et le lecteur l'ouvre.
 */
export default function MotDePasseLecture({ chemin, onOuvert }: { chemin: string; onOuvert: () => void }) {
  const [motDePasse, setMotDePasse] = useState("");
  const [erreur, setErreur] = useState<string | null>(null);
  const [enCours, setEnCours] = useState(false);

  async function ouvrir(evenement: FormEvent<HTMLFormElement>) {
    evenement.preventDefault();
    if (!motDePasse || enCours) return;
    setEnCours(true);
    setErreur(null);
    try {
      await deverrouillerLecture(chemin, motDePasse);
      setMotDePasse("");
      onOuvert();
    } catch (e) {
      setErreur(enErreur(e).message);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Center h="100%" p="md">
      <Paper withBorder p="lg" radius="md" maw={420} w="100%" data-mot-de-passe>
        <form onSubmit={(evenement) => void ouvrir(evenement)}>
          <Stack gap="sm">
            <ThemeIcon variant="light" size="lg" radius="xl">
              <IconLock size={20} />
            </ThemeIcon>
            <Title order={4}>{t.titre}</Title>
            <Text size="sm" c="dimmed">
              {t.aide}
            </Text>
            <PasswordInput label={t.champ} value={motDePasse} onChange={(evenement) => setMotDePasse(evenement.currentTarget.value)} error={erreur} autoFocus autoComplete="off" />
            <Button type="submit" loading={enCours} disabled={!motDePasse}>
              {t.ouvrir}
            </Button>
          </Stack>
        </form>
      </Paper>
    </Center>
  );
}
