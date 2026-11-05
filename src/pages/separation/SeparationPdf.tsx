import { useMemo, useState } from "react";
import { Alert, Button, Group, List, NumberInput, Paper, SegmentedControl, Stack, Text, TextInput, Title } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCheck, IconFileSearch, IconScissors } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { infosPdf, separerPdf } from "../../coeur";
import { nomsUniques } from "../../donnees/noms";
import { balisesDe, remplir, transformationsParDefaut, valeursDeLigne } from "../../donnees/valeurs";
import { textes } from "../../textes";
import EtapeListe from "../commun/EtapeListe";
import { dela, useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";
import { useListe } from "../commun/useListe";

const t = textes.pdf;
const ts = t.separation;

/** Séparer un PDF en fichiers d'une ou de N pages, nommés d'après une liste ou numérotés. */
export default function SeparationPdf() {
  const [source, setSource] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [mode, setMode] = useState<"page" | "groupe">("groupe");
  const [taille, setTaille] = useState(2);
  const [nommage, setNommage] = useState<"numeros" | "liste">("liste");
  const [modeleNom, setModeleNom] = useState("{{Prénom}} {{NOM}}");
  const [enCours, setEnCours] = useState(false);
  const balises = useMemo(() => balisesDe(modeleNom), [modeleNom]);
  const donnees = useListe(balises);
  // Mises en forme courantes : NOM en majuscules, Prénom en Nom Propre.
  const miseEnForme = useMemo(() => ({ transformations: transformationsParDefaut(balises) }), [balises]);
  const pagesParFichier = mode === "page" ? 1 : Math.max(1, taille);
  const nbFichiers = source ? Math.ceil(source.pages / pagesParFichier) : 0;
  const base = source?.nom.replace(/\.pdf$/i, "") ?? "";

  /** Nom de chaque fichier, sans extension ; vide quand il faut le numéroter. */
  const noms = useMemo(() => {
    if (nommage === "numeros" || !donnees.liste) return [];
    const lignes = donnees.liste.lignes.slice(0, nbFichiers);
    const bruts = lignes.map((ligne) => remplir(modeleNom, valeursDeLigne(miseEnForme, ligne, donnees.correspondances)));
    return nomsUniques(bruts).map((nom, i) => (bruts[i] ? nom : ""));
  }, [nommage, donnees.liste, donnees.correspondances, modeleNom, nbFichiers, miseEnForme]);
  const chiffres = Math.max(3, String(nbFichiers).length);
  const nomAffiche = (i: number) => `${noms[i] || `${base} - ${String(i + 1).padStart(chiffres, "0")}`}.pdf`;

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: t.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  async function ouvrir(chemin: string) {
    try {
      setSource({ chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin, pages: await infosPdf(chemin) });
    } catch (e) {
      signalerErreur(e);
    }
  }
  // Un PDF à séparer, et éventuellement la liste qui nomme les fichiers.
  useFichiersRecus("separer", (fichiers) => {
    const [pdf] = dela(fichiers, "pdf");
    const [listeRecue] = dela(fichiers, "liste");
    if (pdf) void ouvrir(pdf);
    if (listeRecue) {
      setNommage("liste");
      void donnees.recevoir(listeRecue);
    }
  });

  async function separer() {
    if (!source) return;
    const dossier = await open({ directory: true, multiple: false, title: ts.choisirDossier });
    if (typeof dossier !== "string") return;
    setEnCours(true);
    try {
      const chemin = await separerPdf(source.chemin, pagesParFichier, noms, dossier, ts.nomDossier(base));
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: ts.cree, message: chemin, autoClose: 10000 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md">
      <Group>
        <Button variant={source ? "light" : "filled"} leftSection={<IconFileSearch size={18} />} onClick={choisir}>
          {source ? ts.autre : ts.choisir}
        </Button>
        {source && (
          <Text size="sm">
            {source.nom} · {t.pages(source.pages)}
          </Text>
        )}
      </Group>

      {source && (
        <>
          <Paper withBorder p="md" maw={720}>
            <Stack gap="sm">
              <Title order={5}>{ts.decoupage}</Title>
              <SegmentedControl
                value={mode}
                onChange={(valeur) => setMode(valeur as typeof mode)}
                data={(["page", "groupe"] as const).map((value) => ({ value, label: ts.modes[value] }))}
              />
              {mode === "groupe" && (
                <NumberInput
                  w={220}
                  label={ts.pagesParFichier}
                  description={ts.pagesParFichierAide}
                  min={1}
                  max={Math.max(1, source.pages)}
                  value={taille}
                  onChange={(valeur) => typeof valeur === "number" && setTaille(valeur)}
                />
              )}
              <Text size="sm" fw={500}>
                {ts.resultat(nbFichiers)}
              </Text>
              <Title order={5} mt="xs">
                {ts.nommage}
              </Title>
              <SegmentedControl
                value={nommage}
                onChange={(valeur) => setNommage(valeur as typeof nommage)}
                data={(["liste", "numeros"] as const).map((value) => ({ value, label: ts.nommages[value] }))}
              />
              {nommage === "liste" && (
                <TextInput label={ts.modeleNom} description={ts.modeleNomAide} value={modeleNom} onChange={(e) => setModeleNom(e.currentTarget.value)} />
              )}
            </Stack>
          </Paper>

          {nommage === "liste" && balises.length > 0 && (
            <EtapeListe
              liste={donnees.liste}
              feuilles={donnees.feuilles}
              balises={balises}
              correspondances={donnees.correspondances}
              exemple={donnees.liste?.lignes[0] ? valeursDeLigne(miseEnForme, donnees.liste.lignes[0], donnees.correspondances) : {}}
              anomalies={donnees.anomalies}
              onImporter={donnees.importer}
              onExemple={donnees.utiliserExemple}
              onFeuille={donnees.setFeuille}
              onCorrespondance={donnees.relier}
            />
          )}

          <Paper withBorder p="md" maw={720}>
            <Stack gap="xs">
              <Title order={5}>{ts.apercu}</Title>
              {nommage === "liste" && donnees.liste && donnees.liste.lignes.length !== nbFichiers && (
                <Alert color="orange">{ts.ecart(donnees.liste.lignes.length, nbFichiers)}</Alert>
              )}
              <List size="sm" spacing={2} data-apercu-noms>
                {Array.from({ length: Math.min(5, nbFichiers) }, (_, i) => (
                  <List.Item key={i}>
                    {nomAffiche(i)}{" "}
                    <Text span size="xs" c="dimmed">
                      ({ts.plage(i * pagesParFichier + 1, Math.min(source.pages, (i + 1) * pagesParFichier))})
                    </Text>
                  </List.Item>
                ))}
              </List>
              <Button leftSection={<IconScissors size={18} />} loading={enCours} onClick={separer} w="fit-content" mt="xs">
                {ts.separer}
              </Button>
            </Stack>
          </Paper>
        </>
      )}
    </Stack>
  );
}
