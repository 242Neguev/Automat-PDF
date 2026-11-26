import { useEffect, useState, type ReactNode } from "react";
import { ActionIcon, Alert, Button, Group, List, Paper, Progress, Stack, Text, Tooltip } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconFilePlus, IconTransform, IconX } from "@tabler/icons-react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Conversions } from "../../coeur";
import { partFaite, type Avancement } from "../../donnees/avancement";
import { dossierDe, nomDe } from "../../donnees/chemins";
import { textes } from "../../textes";
import Action from "../commun/Action";
import BarreAction from "../commun/BarreAction";
import { useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { ZoneDepot } from "../commun/ZoneFichier";

const t = textes.conversion;

interface Props {
  /** Clé de l'outil, pour recevoir les fichiers de l'Explorateur. */
  outil: string;
  /** Fichiers acceptés : nom du filtre et extensions. */
  filtre: { nom: string; extensions: string[] };
  /** Zone de dépôt : ce que l'on attend, et ce que chaque fichier deviendra. */
  deposer: string;
  deposerDetail: string;
  aide: string;
  convertir: (fichiers: string[], dossier: string) => Promise<Conversions>;
  /** Réglages de l'outil, sous la liste des fichiers. */
  options?: ReactNode;
  /** Libellés propres à l'outil, à la place de ceux d'une conversion. */
  libelles?: Partial<Libelles>;
}

interface Libelles {
  bouton: string;
  choisirDossier: string;
  cree: (n: number) => string;
  aucun: string;
  echecs: (n: number) => string;
  /** Avancement page par page (fichier en cours, fichiers, pages lues, pages à lire). */
  progressionPages: (fichier: number, fichiers: number, page: number, pages: number) => string;
}


/** Une liste de fichiers à convertir, chacun en un fichier du même nom dans le dossier choisi. */
export default function ConversionFichiers({ outil, filtre, deposer, deposerDetail, aide, convertir, options, libelles = {} }: Props) {
  const l = {
    bouton: t.convertir,
    choisirDossier: t.choisirDossier,
    cree: t.cree,
    aucun: t.aucun,
    echecs: t.echecs,
    ...libelles,
  };
  const [fichiers, setFichiers] = useState<string[]>([]);
  const [enCours, setEnCours] = useState(false);
  const [progression, setProgression] = useState<Avancement | null>(null);
  const [echecs, setEchecs] = useState<string[]>([]);
  const [cree, setCree] = useState<Cree | null>(null);

  useEffect(() => {
    const arret = listen<Avancement>("progression-conversion", (evenement) => setProgression(evenement.payload)).catch(() => () => {});
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
    const dossier = await open({ directory: true, multiple: false, title: l.choisirDossier, defaultPath: dossierDe(fichiers[0] ?? "") || undefined });
    if (typeof dossier !== "string") return;
    setEnCours(true);
    setProgression(null);
    setEchecs([]);
    setCree(null);
    try {
      const resultat = await convertir(fichiers, dossier);
      setEchecs(resultat.echecs.map((echec) => `${nomDe(fichiers[echec.rang] ?? "")} : ${echec.message}`));
      if (resultat.crees.length > 0) {
        // Un seul fichier : on peut l'ouvrir ; plusieurs : on ouvre leur dossier.
        setCree(
          resultat.crees.length === 1
            ? { titre: l.cree(1), chemin: resultat.crees[0] }
            : { titre: l.cree(resultat.crees.length), chemin: dossier, dossier: true },
        );
        // Les fichiers convertis quittent la liste ; ceux en échec y restent, pour réessayer.
        const enEchec = new Set(resultat.echecs.map((echec) => echec.rang));
        setFichiers((actuels) => actuels.filter((_, rang) => enEchec.has(rang)));
      } else notifications.show({ color: "orange", title: l.aucun, message: "", autoClose: 10000 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
      setProgression(null);
    }
  }

  return (
    <Stack gap="md" maw={720}>
      {fichiers.length === 0 ? (
        <ZoneDepot invite={deposer} detail={deposerDetail} bouton={t.ajouter} onChoisir={() => void ajouter()} />
      ) : (
        <>
          <Group>
            <Button variant="light" leftSection={<IconFilePlus size={18} />} onClick={() => void ajouter()}>
              {t.ajouter}
            </Button>
            <Button variant="subtle" color="gray" onClick={() => setFichiers([])}>
              {t.vider}
            </Button>
          </Group>
          <Paper withBorder p="md">
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
          </Paper>
        </>
      )}
      {options}
      <Text size="sm" c="dimmed">
        {aide}
      </Text>
      <BarreAction>
        <Action icone={IconTransform} enCours={enCours} raison={fichiers.length === 0 && !enCours ? t.raison : null} onClick={() => void lancer()}>
          {l.bouton}
        </Action>
      </BarreAction>
      {enCours && progression && (
        <div data-avancement>
          <Progress value={100 * partFaite(progression)} />
          <Text size="sm" c="dimmed" mt={4}>
            {progression.length === 4 && progression[3] > 0 && l.progressionPages
              ? l.progressionPages(Math.min(progression[0] + 1, progression[1]), progression[1], progression[2], progression[3])
              : t.progression(progression[0], progression[1])}
          </Text>
        </div>
      )}
      <Resultat cree={cree} onFermer={() => setCree(null)} />
      {echecs.length > 0 && (
        <Alert color="red" title={l.echecs(echecs.length)}>
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
