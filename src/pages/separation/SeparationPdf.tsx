import { useMemo, useState } from "react";
import { Alert, List, NumberInput, SegmentedControl, Stack, Text } from "@mantine/core";
import { IconScissors } from "@tabler/icons-react";
import { open } from "@tauri-apps/plugin-dialog";
import { infosPdf, separerPdf } from "../../coeur";
import { dossierDe, nomDe } from "../../donnees/chemins";
import { nomsUniques } from "../../donnees/noms";
import { balisesDe, remplir, transformationsParDefaut, valeursDeLigne } from "../../donnees/valeurs";
import { textes } from "../../textes";
import Action from "../commun/Action";
import BarreAction from "../commun/BarreAction";
import ChampModele from "../commun/ChampModele";
import Etape from "../commun/Etape";
import EtapeListe from "../commun/EtapeListe";
import { dela, useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { useListe } from "../commun/useListe";
import { ZoneFichier } from "../commun/ZoneFichier";

const t = textes.pdf;
const ts = t.separation;

/** Séparer un PDF en fichiers d'une ou de N pages, numérotés ou nommés d'après une liste. */
export default function SeparationPdf() {
  const [source, setSource] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [mode, setMode] = useState<"page" | "groupe">("groupe");
  const [taille, setTaille] = useState(2);
  // Numérotés par défaut : rien d'autre à fournir. La liste sert à nommer les fichiers.
  const [nommage, setNommage] = useState<"numeros" | "liste">("numeros");
  const [modeleNom, setModeleNom] = useState("{{Prénom}} {{NOM}}");
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
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
      setSource({ chemin, nom: nomDe(chemin), pages: await infosPdf(chemin) });
      setCree(null);
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
    const dossier = await open({ directory: true, multiple: false, title: ts.choisirDossier, defaultPath: dossierDe(source.chemin) || undefined });
    if (typeof dossier !== "string") return;
    setEnCours(true);
    setCree(null);
    try {
      const chemin = await separerPdf(source.chemin, pagesParFichier, noms, dossier, ts.nomDossier(base));
      setCree({ titre: ts.cree(nbFichiers), chemin, dossier: true });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md" maw={900}>
      <Etape numero={1} titre={ts.etapes.pdf}>
        <ZoneFichier
          fichier={source ? { nom: source.nom, details: t.pages(source.pages) } : null}
          invite={textes.depot.pdf}
          bouton={ts.choisir}
          changer={ts.autre}
          onChoisir={() => void choisir()}
        />
      </Etape>

      {source && (
        <>
          <Etape numero={2} titre={ts.etapes.decoupage}>
            <SegmentedControl
              value={mode}
              onChange={(valeur) => setMode(valeur as typeof mode)}
              data={(["page", "groupe"] as const).map((value) => ({ value, label: ts.modes[value] }))}
            />
            {mode === "groupe" && (
              <NumberInput
                w={260}
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
          </Etape>

          <Etape numero={3} titre={ts.etapes.noms}>
            <SegmentedControl
              value={nommage}
              onChange={(valeur) => setNommage(valeur as typeof nommage)}
              data={(["numeros", "liste"] as const).map((value) => ({ value, label: ts.nommages[value] }))}
            />
            {nommage === "liste" && (
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
                apresImport={
                  <ChampModele
                    label={ts.modeleNom}
                    description={ts.modeleNomAide}
                    valeur={modeleNom}
                    onValeur={setModeleNom}
                    colonnes={donnees.liste?.colonnes ?? []}
                    exemple={nbFichiers > 0 ? nomAffiche(0) : undefined}
                  />
                }
              />
            )}
          </Etape>

          <Etape numero={4} titre={ts.etapes.separer}>
            {nommage === "liste" && donnees.liste && donnees.liste.lignes.length !== nbFichiers && <Alert color="orange">{ts.ecart(donnees.liste.lignes.length, nbFichiers)}</Alert>}
            <Text size="sm" fw={500}>
              {ts.apercu}
            </Text>
            <List size="sm" spacing={2} data-apercu-noms>
              {Array.from({ length: Math.min(5, nbFichiers) }, (_, i) => (
                <List.Item key={i}>
                  {nomAffiche(i)}{" "}
                  <Text span size="sm" c="dimmed">
                    ({ts.plage(i * pagesParFichier + 1, Math.min(source.pages, (i + 1) * pagesParFichier))})
                  </Text>
                </List.Item>
              ))}
            </List>
            <BarreAction>
              <Action icone={IconScissors} enCours={enCours} onClick={() => void separer()}>
                {ts.separer}
              </Action>
            </BarreAction>
            <Resultat cree={cree} onFermer={() => setCree(null)} />
          </Etape>
        </>
      )}
    </Stack>
  );
}
