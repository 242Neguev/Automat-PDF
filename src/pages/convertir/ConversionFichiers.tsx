import { useEffect, useState } from "react";
import { ActionIcon, Alert, Button, Group, List, Paper, Progress, Stack, Text, Tooltip } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCheck, IconFilePlus, IconTransform, IconX } from "@tabler/icons-react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Conversions } from "../../coeur";
import { textes } from "../../textes";
import { useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";

const t = textes.conversion;
const nomDe = (chemin: string) => chemin.split(/[\\/]/).pop() ?? chemin;

interface Props {
  /** Clé de l'outil, pour recevoir les fichiers de l'Explorateur. */
  outil: string;
  /** Fichiers acceptés : nom du filtre et extensions. */
  filtre: { nom: string; extensions: string[] };
  vide: string;
  aide: string;
  convertir: (fichiers: string[], dossier: string) => Promise<Conversions>;
}

/** Une liste de fichiers à convertir, chacun en un fichier du même nom dans le dossier choisi. */
export default function ConversionFichiers({ outil, filtre, vide, aide, convertir }: Props) {
  const [fichiers, setFichiers] = useState<string[]>([]);
  const [enCours, setEnCours] = useState(false);
  const [progression, setProgression] = useState<[number, number] | null>(null);
  const [echecs, setEchecs] = useState<string[]>([]);

  useEffect(() => {
    const arret = listen<[number, number]>("progression-conversion", (evenement) => setProgression(evenement.payload)).catch(() => () => {});
    return () => void arret.then((arreter) => arreter());
  }, []);

  async function ajouter() {
    const choisis = await open({ multiple: true, directory: false, filters: [{ name: filtre.nom, extensions: filtre.extensions }] });
    if (!choisis) return;
    ajouterChemins(Array.isArray(choisis) ? choisis : [choisis]);
  }

  const ajouterChemins = (nouveaux: string[]) => setFichiers((actuels) => [...actuels, ...nouveaux.filter((chemin) => !actuels.includes(chemin))]);
  useFichiersRecus(outil, ajouterChemins);

  async function lancer() {
    const dossier = await open({ directory: true, multiple: false, title: t.choisirDossier });
    if (typeof dossier !== "string") return;
    setEnCours(true);
    setProgression(null);
    setEchecs([]);
    try {
      const resultat = await convertir(fichiers, dossier);
      setEchecs(resultat.echecs.map((echec) => `${nomDe(fichiers[echec.rang] ?? "")} : ${echec.message}`));
      if (resultat.crees.length > 0) {
        notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.cree(resultat.crees.length), message: dossier, autoClose: 10000 });
        // Les fichiers convertis quittent la liste ; ceux en échec y restent, pour réessayer.
        const enEchec = new Set(resultat.echecs.map((echec) => echec.rang));
        setFichiers((actuels) => actuels.filter((_, rang) => enEchec.has(rang)));
      } else notifications.show({ color: "orange", title: t.aucun, message: "" });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
      setProgression(null);
    }
  }

  return (
    <Stack gap="md" maw={720}>
      <Group>
        <Button leftSection={<IconFilePlus size={18} />} onClick={ajouter}>
          {t.ajouter}
        </Button>
        {fichiers.length > 0 && (
          <Button variant="subtle" color="gray" onClick={() => setFichiers([])}>
            {t.vider}
          </Button>
        )}
      </Group>
      <Paper withBorder p="md">
        {fichiers.length === 0 ? (
          <Text size="sm" c="dimmed">
            {vide}
          </Text>
        ) : (
          <Stack gap={4} data-fichiers>
            <Text size="sm" fw={500}>
              {t.total(fichiers.length)}
            </Text>
            {fichiers.map((chemin) => (
              <Group key={chemin} justify="space-between" wrap="nowrap">
                <Text size="sm" truncate>
                  {nomDe(chemin)}
                </Text>
                <Tooltip label={t.retirer}>
                  <ActionIcon variant="subtle" color="gray" aria-label={t.retirer} onClick={() => setFichiers((actuels) => actuels.filter((autre) => autre !== chemin))}>
                    <IconX size={16} />
                  </ActionIcon>
                </Tooltip>
              </Group>
            ))}
          </Stack>
        )}
      </Paper>
      <Text size="xs" c="dimmed">
        {aide}
      </Text>
      <Button leftSection={<IconTransform size={18} />} loading={enCours} disabled={fichiers.length === 0} onClick={lancer} w="fit-content">
        {t.convertir}
      </Button>
      {enCours && progression && (
        <div>
          <Progress value={(100 * progression[0]) / Math.max(1, progression[1])} />
          <Text size="xs" c="dimmed" mt={4}>
            {t.progression(progression[0], progression[1])}
          </Text>
        </div>
      )}
      {echecs.length > 0 && (
        <Alert color="red" title={t.echecs(echecs.length)}>
          <List size="sm" spacing={2}>
            {echecs.map((ligne) => (
              <List.Item key={ligne}>{ligne}</List.Item>
            ))}
          </List>
        </Alert>
      )}
    </Stack>
  );
}
