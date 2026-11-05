import { useEffect, useMemo, useState } from "react";
import { Alert, Badge, Button, Group, List, Modal, Paper, Progress, SegmentedControl, Select, Stack, Switch, Table, Text, Textarea, TextInput, Title } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconAlertTriangle, IconCheck, IconDeviceFloppy, IconFileSearch, IconMail, IconSend, IconSettings, IconTrash, IconUsers } from "@tabler/icons-react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  enregistrerEnchainements,
  enregistrerMessagerie,
  envoyerParMessagerie,
  envoyerParOutlook,
  lireEnchainements as lireFichierEnchainements,
  lireMessagerie,
  motDePasseMessagerieEnregistre,
  separerParPersonne,
  textesPdf,
  type FichierPersonne,
  type Message,
} from "../../coeur";
import { avecEnchainement, lireEnchainements, reglagesDe, REGLAGES_PAR_DEFAUT, sansEnchainement, type Enchainement, type Reglages, type ReglagesEnvoi } from "../../donnees/enchainements";
import { lireReglagesMessagerie, manqueAuCompte, REGLAGES_MESSAGERIE_PAR_DEFAUT, type MoyenEnvoi, type ReglagesMessagerie } from "../../donnees/messagerie";
import { nomsUniques } from "../../donnees/noms";
import { attribuerParListe, attribuerParRepere, pagesAVerifier, pagesParPersonne, valeurApres, type EtatPage, type PositionValeur } from "../../donnees/reperage";
import { balisesDe, remplir, remplirTexte, transformationsParDefaut, valeursDeLigne } from "../../donnees/valeurs";
import { textes } from "../../textes";
import EtapeListe from "../commun/EtapeListe";
import { dela, useFichiersRecus } from "../commun/reception";
import { signalerErreur } from "../commun/signalerErreur";
import { useListe } from "../commun/useListe";
import FenetreMessagerie from "./ReglagesMessagerie";

const t = textes.parPersonne;
const APERCU = 8;

/** Numéros de pages lisibles (« 3, 7, 12 ») ; une longue suite est résumée. */
function numeros(pages: number[]): string {
  const affiches = pages.slice(0, 10).map((page) => String(page + 1));
  return pages.length > 10 ? `${affiches.join(", ")}… (${pages.length})` : affiches.join(", ");
}

const quelques = (noms: string[]) => (noms.length > 3 ? `${noms.slice(0, 3).join(", ")}…` : noms.join(", "));

/** Une action à confirmer : supprimer un enchaînement, envoyer des e-mails. */
interface Confirmation {
  titre: string;
  texte: string;
  bouton: string;
  couleur: string;
  action: () => void;
}

/** Séparer par personne : un PDF par personne, d'après le matricule ou le nom imprimé sur chaque page. */
export default function SeparationParPersonne() {
  const [source, setSource] = useState<{ chemin: string; nom: string; textes: string[] } | null>(null);
  const [lecture, setLecture] = useState(false);
  const [reglages, setReglages] = useState<Reglages>(REGLAGES_PAR_DEFAUT);
  const regler = (partiel: Partial<Reglages>) => setReglages((actuels) => ({ ...actuels, ...partiel }));
  const reglerEnvoi = (partiel: Partial<ReglagesEnvoi>) => setReglages((actuels) => ({ ...actuels, envoi: { ...actuels.envoi, ...partiel } }));
  const { mode, envoi } = reglages;
  const [enCours, setEnCours] = useState(false);
  const [enchainements, setEnchainements] = useState<Enchainement[]>([]);
  const [choisi, setChoisi] = useState<string | null>(null);
  const [nomEnregistrement, setNomEnregistrement] = useState<string | null>(null);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  /** Fichiers créés, à joindre aux messages. */
  const [creation, setCreation] = useState<{ personne: number; nom: string; chemin: string }[] | null>(null);
  const [adresseEssai, setAdresseEssai] = useState("");
  const [envoiEnCours, setEnvoiEnCours] = useState(false);
  const [progression, setProgression] = useState<[number, number] | null>(null);
  const [echecs, setEchecs] = useState<string[]>([]);
  /** Moyen d'envoi et compte de la messagerie, propres à ce PC (hors enchaînements). */
  const [messagerie, setMessagerie] = useState<ReglagesMessagerie>(REGLAGES_MESSAGERIE_PAR_DEFAUT);
  const [motDePasseGarde, setMotDePasseGarde] = useState(false);
  const [reglageMessagerie, setReglageMessagerie] = useState(false);
  const parMessagerie = messagerie.moyen === "messagerie";
  // Avec la messagerie, pas de brouillons : envoi direct, après confirmation et essai.
  const modeEnvoi: ReglagesEnvoi["mode"] = parMessagerie ? "direct" : envoi.mode;
  const compteMessagerie = messagerie.compte && manqueAuCompte(messagerie.compte) === null ? messagerie.compte : null;
  const pretAEnvoyer = !parMessagerie || compteMessagerie !== null;
  // L'essai part, par défaut, à l'adresse même de la messagerie.
  const adresseEssaiEffective = adresseEssai.trim() || (parMessagerie && compteMessagerie ? compteMessagerie.adresse : "");
  const base = source?.nom.replace(/\.pdf$/i, "") ?? "";
  const avecTexte = source ? source.textes.filter((texte) => texte.trim()).length : 0;
  const protection = reglages.proteger && mode === "liste";
  const avecEnvoi = envoi.actif && mode === "liste";

  useEffect(() => {
    lireFichierEnchainements().then((contenu) => setEnchainements(lireEnchainements(contenu)), signalerErreur);
    lireMessagerie().then((contenu) => setMessagerie(lireReglagesMessagerie(contenu)), signalerErreur);
    motDePasseMessagerieEnregistre().then(setMotDePasseGarde, () => setMotDePasseGarde(false));
    const arret = listen<[number, number]>("progression-envoi", (evenement) => setProgression(evenement.payload)).catch(() => () => {});
    return () => void arret.then((arreter) => arreter());
  }, []);

  const balises = useMemo(
    () => [
      ...new Set([
        ...balisesDe(reglages.identifiant),
        ...balisesDe(reglages.modeleNom),
        ...(protection ? balisesDe(reglages.modeleMotDePasse) : []),
        ...(avecEnvoi ? [...balisesDe(envoi.destinataire), ...balisesDe(envoi.objet), ...balisesDe(envoi.message)] : []),
      ]),
    ],
    [reglages.identifiant, reglages.modeleNom, protection, reglages.modeleMotDePasse, avecEnvoi, envoi.destinataire, envoi.objet, envoi.message],
  );
  const donnees = useListe(balises);
  const { liste, correspondances } = donnees;
  // Mises en forme courantes pour les noms et les adresses ; le mot de passe, lui, est pris tel quel.
  const miseEnForme = useMemo(() => ({ transformations: transformationsParDefaut(balises) }), [balises]);

  /** État de chaque page, et pour chaque rang (ligne de la liste, ou valeur lue) : nom de fichier, nom lisible, mot de passe. */
  const reperage = useMemo(() => {
    if (!source) return null;
    if (mode === "liste") {
      if (!liste) return null;
      const brutes = liste.lignes.map((ligne) => valeursDeLigne({}, ligne, correspondances));
      const identifiants = brutes.map((valeurs) => remplir(reglages.identifiant, valeurs));
      const personnes = liste.lignes.map((ligne, rang) => ({
        identifiant: identifiants[rang],
        informations: liste.colonnes.map((colonne) => ligne.valeurs[colonne] ?? ""),
      }));
      const seulementBalises = balisesDe(reglages.modeleNom)
        .map((balise) => `{{${balise}}}`)
        .join(" ");
      return {
        etats: attribuerParListe(source.textes, personnes, reglages.suite),
        identifiants,
        fichier: (rang: number) => remplir(reglages.modeleNom, valeursDeLigne(miseEnForme, liste.lignes[rang], correspondances)),
        qui: (rang: number) =>
          remplir(seulementBalises, valeursDeLigne(miseEnForme, liste.lignes[rang], correspondances)) || identifiants[rang] || `ligne ${liste.lignes[rang].numero}`,
        motDePasse: (rang: number) => remplir(reglages.modeleMotDePasse, brutes[rang]),
      };
    }
    if (!reglages.repere.trim()) return null;
    const { etats, valeurs } = attribuerParRepere(source.textes, reglages.repere, reglages.position, reglages.suite);
    return {
      etats,
      identifiants: valeurs,
      fichier: (rang: number) => remplir(reglages.modeleNomRepere, { Repère: valeurs[rang] }),
      qui: (rang: number) => valeurs[rang],
      motDePasse: null,
    };
  }, [source, mode, liste, correspondances, miseEnForme, reglages.identifiant, reglages.modeleNom, reglages.modeleNomRepere, reglages.modeleMotDePasse, reglages.repere, reglages.position, reglages.suite]);

  const fichiers = useMemo(() => {
    if (!reperage) return [];
    const groupes = pagesParPersonne(reperage.etats);
    const noms = nomsUniques(groupes.map(({ personne }, i) => reperage.fichier(personne) || `${base} - ${i + 1}`));
    return groupes.map(({ personne, pages }, i) => ({
      personne,
      nom: noms[i],
      pages,
      motDePasse: protection && reperage.motDePasse ? reperage.motDePasse(personne) : undefined,
    }));
  }, [reperage, base, protection]);
  // Les fichiers créés ne valent que pour la séparation qui les a produits.
  const signature = JSON.stringify([source?.chemin, fichiers.map(({ personne, nom, pages, motDePasse }) => [personne, nom, pages, motDePasse])]);
  useEffect(() => setCreation(null), [signature]);

  const aVerifier = useMemo(() => (reperage ? pagesAVerifier(reperage.etats) : []), [reperage]);
  const problemes = useMemo(() => {
    if (!reperage) return [];
    const lignes: string[] = [];
    const dansLEtat = (etat: EtatPage["etat"]) => reperage.etats.flatMap((e, page) => (e.etat === etat ? [page] : []));
    const sansTexte = dansLEtat("sans-texte");
    if (sansTexte.length > 0) lignes.push(t.etats["sans-texte"](numeros(sansTexte)));
    const sansPersonne = dansLEtat("sans-personne");
    if (sansPersonne.length > 0) lignes.push((mode === "repere" ? t.etats.sansRepere : t.etats["sans-personne"])(numeros(sansPersonne)));
    reperage.etats.forEach((etat, page) => {
      if (etat.etat === "douteuse") lignes.push(t.etats.douteuse(page + 1, reperage.qui(etat.personnes[0])));
      if (etat.etat === "ambigue") lignes.push(t.etats.ambigue(page + 1, quelques(etat.personnes.map(reperage.qui))));
    });
    return lignes;
  }, [reperage, mode]);
  const absents = useMemo(() => {
    if (!reperage || mode !== "liste") return [];
    const trouves = new Set(fichiers.map((fichier) => fichier.personne));
    return reperage.identifiants.flatMap((identifiant, rang) => (identifiant && !trouves.has(rang) ? [reperage.qui(rang)] : []));
  }, [reperage, fichiers, mode]);
  const sansMotDePasse = protection ? fichiers.filter((fichier) => !fichier.motDePasse) : [];
  const pagesAttribuees = fichiers.reduce((total, fichier) => total + fichier.pages.length, 0);

  /** Message de chaque fichier créé ; les fichiers sans adresse sont mis à part. */
  const envois = useMemo(() => {
    const messages: { nom: string; message: Message }[] = [];
    const sansAdresse: string[] = [];
    if (!creation || !liste || !avecEnvoi) return { messages, sansAdresse };
    for (const { personne, nom, chemin } of creation) {
      const valeurs = valeursDeLigne(miseEnForme, liste.lignes[personne], correspondances);
      const a = remplir(envoi.destinataire, valeurs);
      if (a) messages.push({ nom, message: { a, objet: remplir(envoi.objet, valeurs), texte: remplirTexte(envoi.message, valeurs), pieces: [chemin] } });
      else sansAdresse.push(nom);
    }
    return { messages, sansAdresse };
  }, [creation, liste, avecEnvoi, miseEnForme, correspondances, envoi.destinataire, envoi.objet, envoi.message]);

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  // Le PDF des bulletins, et éventuellement la liste des personnes.
  useFichiersRecus("parPersonne", (fichiers) => {
    const [pdf] = dela(fichiers, "pdf");
    const [listeRecue] = dela(fichiers, "liste");
    if (pdf) void ouvrir(pdf);
    if (listeRecue) {
      regler({ mode: "liste" });
      void donnees.recevoir(listeRecue);
    }
  });

  async function ouvrir(chemin: string) {
    setLecture(true);
    try {
      setSource({ chemin, nom: chemin.split(/[\\/]/).pop() ?? chemin, textes: await textesPdf(chemin) });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setLecture(false);
    }
  }

  async function creer() {
    if (!source || fichiers.length === 0) return;
    const dossier = await open({ directory: true, multiple: false, title: t.choisirDossier });
    if (typeof dossier !== "string") return;
    setEnCours(true);
    try {
      const personnes: FichierPersonne[] = fichiers.map(({ nom, pages, motDePasse }) => ({ nom, pages, motDePasse }));
      if (aVerifier.length > 0) personnes.push({ nom: t.nomAVerifier, pages: aVerifier });
      const separation = await separerParPersonne(source.chemin, personnes, dossier, t.nomDossier(base));
      setCreation(fichiers.map(({ personne, nom }, i) => ({ personne, nom, chemin: separation.fichiers[i] })));
      setEchecs([]);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.cree, message: t.creeMessage(personnes.length, separation.dossier), autoClose: 10000 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  /** Le moyen d'envoi choisi sur ce PC, gardé pour la prochaine fois. */
  function choisirMoyen(moyen: MoyenEnvoi) {
    const nouveaux = { ...messagerie, moyen };
    setMessagerie(nouveaux);
    enregistrerMessagerie(JSON.stringify(nouveaux, null, 2)).catch(signalerErreur);
  }

  async function envoyer(aEnvoyer: { nom: string; message: Message }[]) {
    if (parMessagerie && !compteMessagerie) return setReglageMessagerie(true);
    setEnvoiEnCours(true);
    setProgression(null);
    try {
      const messages = aEnvoyer.map(({ message }) => message);
      const bilan = compteMessagerie && parMessagerie ? await envoyerParMessagerie(compteMessagerie, messages) : await envoyerParOutlook(messages, envoi.mode);
      setEchecs(bilan.echecs.map((echec) => `${aEnvoyer[echec.rang]?.nom ?? echec.rang + 1} : ${echec.message}`));
      notifications.show({
        color: bilan.echecs.length > 0 ? "orange" : "teal",
        icon: <IconCheck size={18} />,
        title: t.envoi.resultat[modeEnvoi](bilan.reussis),
        message: bilan.echecs.length > 0 ? t.envoi.echecs(bilan.echecs.length) : "",
        autoClose: 10000,
      });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnvoiEnCours(false);
      setProgression(null);
    }
  }

  function envoyerTout() {
    if (modeEnvoi === "brouillons") return void envoyer(envois.messages);
    setConfirmation({
      titre: t.envoi.confirmerTitre,
      texte: t.envoi.confirmerTexte(envois.messages.length, parMessagerie && compteMessagerie ? compteMessagerie.adresse : "Outlook"),
      bouton: t.envoi.confirmer,
      couleur: "eau",
      action: () => void envoyer(envois.messages),
    });
  }

  function reprendre(nom: string | null) {
    setChoisi(nom);
    const enchainement = enchainements.find((existant) => existant.nom === nom);
    if (!enchainement) return;
    setReglages(reglagesDe(enchainement));
    if (enchainement.liste) {
      donnees.restaurer(enchainement.liste.correspondances, Object.keys(enchainement.liste.correspondances));
      void donnees.importerDepuis(enchainement.liste.chemin, enchainement.liste.feuille);
    }
    notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.enchainement.repris(enchainement.nom), message: enchainement.liste || enchainement.mode === "repere" ? "" : t.enchainement.sansListe });
  }

  async function enregistrer() {
    const nom = nomEnregistrement?.trim();
    if (!nom) return;
    const nouveaux = avecEnchainement(enchainements, {
      ...reglages,
      nom,
      liste: donnees.chemin ? { chemin: donnees.chemin, feuille: liste?.feuille ?? null, correspondances } : null,
    });
    try {
      await enregistrerEnchainements(JSON.stringify(nouveaux, null, 2));
      setEnchainements(nouveaux);
      setChoisi(nouveaux.find((existant) => existant.nom === nom)?.nom ?? nom);
      setNomEnregistrement(null);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.enchainement.enregistre(nom), message: "" });
    } catch (e) {
      signalerErreur(e);
    }
  }

  function supprimer() {
    if (!choisi) return;
    const nom = choisi;
    setConfirmation({
      titre: t.enchainement.supprimerTitre(nom),
      texte: t.enchainement.supprimerTexte,
      bouton: t.enchainement.supprimer,
      couleur: "red",
      action: async () => {
        const nouveaux = sansEnchainement(enchainements, nom);
        try {
          await enregistrerEnchainements(JSON.stringify(nouveaux, null, 2));
          setEnchainements(nouveaux);
          setChoisi(null);
          notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.enchainement.supprime(nom), message: "" });
        } catch (e) {
          signalerErreur(e);
        }
      },
    });
  }

  const nomExistant = nomEnregistrement !== null && enchainements.some((existant) => existant.nom.trim().toLocaleLowerCase("fr") === nomEnregistrement.trim().toLocaleLowerCase("fr"));

  return (
    <Stack gap="md">
      <Paper withBorder p="md" maw={720} data-enchainements>
        <Stack gap={6}>
          <Group align="flex-end" gap="sm">
            <Select
              label={t.enchainement.titre}
              placeholder={enchainements.length > 0 ? t.enchainement.choisir : t.enchainement.aucun}
              data={enchainements.map((enchainement) => enchainement.nom)}
              value={choisi}
              onChange={reprendre}
              disabled={enchainements.length === 0}
              style={{ flex: 1 }}
            />
            <Button variant="light" leftSection={<IconDeviceFloppy size={18} />} onClick={() => setNomEnregistrement(choisi ?? t.enchainement.nomParDefaut)}>
              {t.enchainement.enregistrer}
            </Button>
            {choisi && (
              <Button variant="subtle" color="red" leftSection={<IconTrash size={18} />} onClick={supprimer}>
                {t.enchainement.supprimer}
              </Button>
            )}
          </Group>
          <Text size="xs" c="dimmed">
            {t.enchainement.aide}
          </Text>
        </Stack>
      </Paper>

      <Group>
        <Button variant={source ? "light" : "filled"} leftSection={<IconFileSearch size={18} />} loading={lecture} onClick={choisir}>
          {source ? t.autre : t.choisir}
        </Button>
        {source && (
          <Text size="sm">
            {source.nom} · {t.lu(source.textes.length, avecTexte)}
          </Text>
        )}
      </Group>
      {source && avecTexte === 0 && (
        <Alert color="orange" maw={720}>
          {t.sansTexte}
        </Alert>
      )}

      {source && avecTexte > 0 && (
        <>
          <Paper withBorder p="md" maw={720}>
            <Stack gap="sm">
              <Title order={5}>{t.reperage}</Title>
              <SegmentedControl value={mode} onChange={(v) => regler({ mode: v as Reglages["mode"] })} data={(["liste", "repere"] as const).map((value) => ({ value, label: t.modes[value] }))} />
              {mode === "liste" ? (
                <TextInput label={t.identifiant} description={t.identifiantAide} value={reglages.identifiant} onChange={(e) => regler({ identifiant: e.currentTarget.value })} />
              ) : (
                <>
                  <TextInput label={t.repere} description={t.repereAide} value={reglages.repere} onChange={(e) => regler({ repere: e.currentTarget.value })} />
                  <SegmentedControl
                    value={reglages.position}
                    onChange={(v) => regler({ position: v as PositionValeur })}
                    data={(["droite", "dessous"] as const).map((value) => ({ value, label: t.positions[value] }))}
                  />
                  <div data-valeurs-lues>
                    <Text size="sm" fw={500}>
                      {t.valeursLues}
                    </Text>
                    {source.textes.slice(0, 3).map((texte, page) => (
                      <Text key={page} size="sm" c="dimmed">
                        {t.pageValeur(page + 1, valeurApres(texte, reglages.repere, reglages.position))}
                      </Text>
                    ))}
                  </div>
                </>
              )}
              <Switch label={t.suite} description={t.suiteAide} checked={reglages.suite} onChange={(e) => regler({ suite: e.currentTarget.checked })} />
            </Stack>
          </Paper>

          <Paper withBorder p="md" maw={720}>
            <Stack gap="sm">
              <Title order={5}>{t.fichiers}</Title>
              {mode === "liste" ? (
                <TextInput label={t.modeleNom} description={t.modeleNomAide} value={reglages.modeleNom} onChange={(e) => regler({ modeleNom: e.currentTarget.value })} />
              ) : (
                <TextInput label={t.modeleNom} description={t.modeleNomRepereAide} value={reglages.modeleNomRepere} onChange={(e) => regler({ modeleNomRepere: e.currentTarget.value })} />
              )}
              {mode === "liste" && (
                <>
                  <Switch label={t.proteger} checked={reglages.proteger} onChange={(e) => regler({ proteger: e.currentTarget.checked })} />
                  {reglages.proteger && (
                    <TextInput label={t.motDePasse} description={t.motDePasseAide} value={reglages.modeleMotDePasse} onChange={(e) => regler({ modeleMotDePasse: e.currentTarget.value })} />
                  )}
                  <Switch label={t.envoi.activer} description={t.envoi.aide} checked={envoi.actif} onChange={(e) => reglerEnvoi({ actif: e.currentTarget.checked })} />
                  {envoi.actif && (
                    <Stack gap="xs" data-message>
                      <TextInput label={t.envoi.destinataire} description={t.envoi.destinataireAide} value={envoi.destinataire} onChange={(e) => reglerEnvoi({ destinataire: e.currentTarget.value })} />
                      <TextInput label={t.envoi.objet} value={envoi.objet} onChange={(e) => reglerEnvoi({ objet: e.currentTarget.value })} />
                      <Textarea label={t.envoi.message} description={t.envoi.messageAide} autosize minRows={4} value={envoi.message} onChange={(e) => reglerEnvoi({ message: e.currentTarget.value })} />
                      <div data-moyen-envoi>
                        <Text size="sm" fw={500} mb={4}>
                          {t.envoi.moyen}
                        </Text>
                        <SegmentedControl value={messagerie.moyen} onChange={(v) => choisirMoyen(v as MoyenEnvoi)} data={(["messagerie", "outlook"] as const).map((value) => ({ value, label: t.envoi.moyens[value] }))} />
                      </div>
                      {parMessagerie ? (
                        <>
                          <Group gap="sm" justify="space-between" wrap="nowrap">
                            <Text size="sm" c={compteMessagerie ? undefined : "orange"}>
                              {compteMessagerie ? t.envoi.depuis(compteMessagerie.adresse, compteMessagerie.serveur) : t.envoi.aRegler}
                            </Text>
                            <Button variant="light" size="xs" leftSection={<IconSettings size={16} />} onClick={() => setReglageMessagerie(true)} style={{ flexShrink: 0 }}>
                              {t.envoi.regler}
                            </Button>
                          </Group>
                          <Text size="xs" c="dimmed">
                            {t.envoi.messagerieAide}
                          </Text>
                        </>
                      ) : (
                        <>
                          <SegmentedControl
                            value={envoi.mode}
                            onChange={(v) => reglerEnvoi({ mode: v as ReglagesEnvoi["mode"] })}
                            data={(["brouillons", "direct"] as const).map((value) => ({ value, label: t.envoi.modes[value] }))}
                          />
                          <Text size="xs" c="dimmed">
                            {t.envoi.outlookAide} {t.envoi.modesAide[envoi.mode]}
                          </Text>
                        </>
                      )}
                    </Stack>
                  )}
                </>
              )}
            </Stack>
          </Paper>

          {mode === "liste" && (
            <EtapeListe
              liste={liste}
              feuilles={donnees.feuilles}
              balises={balises}
              correspondances={correspondances}
              exemple={liste?.lignes[0] ? valeursDeLigne(miseEnForme, liste.lignes[0], correspondances) : {}}
              anomalies={donnees.anomalies}
              onImporter={donnees.importer}
              onExemple={donnees.utiliserExemple}
              onFeuille={donnees.setFeuille}
              onCorrespondance={donnees.relier}
            />
          )}

          {reperage && (
            <Paper withBorder p="md" maw={720} data-verification>
              <Stack gap="sm">
                <Group justify="space-between">
                  <Title order={5}>{t.verification}</Title>
                  <Badge size="lg" variant="light" style={{ textTransform: "none" }}>
                    {t.resume(fichiers.length, pagesAttribuees)}
                  </Badge>
                </Group>
                {fichiers.length === 0 ? (
                  <Alert color="orange">{t.aucune}</Alert>
                ) : (
                  <Table withTableBorder fz="sm" verticalSpacing={4}>
                    <Table.Thead>
                      <Table.Tr>
                        <Table.Th>{t.colonnes.fichier}</Table.Th>
                        <Table.Th>{t.colonnes.pages}</Table.Th>
                        {protection && <Table.Th>{t.colonnes.motDePasse}</Table.Th>}
                      </Table.Tr>
                    </Table.Thead>
                    <Table.Tbody>
                      {fichiers.slice(0, APERCU).map((fichier) => (
                        <Table.Tr key={fichier.nom}>
                          <Table.Td>{fichier.nom}.pdf</Table.Td>
                          <Table.Td>{numeros(fichier.pages)}</Table.Td>
                          {protection && (
                            <Table.Td>
                              <Badge size="sm" variant="light" color={fichier.motDePasse ? "teal" : "red"}>
                                {fichier.motDePasse ? t.defini : t.vide}
                              </Badge>
                            </Table.Td>
                          )}
                        </Table.Tr>
                      ))}
                    </Table.Tbody>
                  </Table>
                )}
                {fichiers.length > APERCU && (
                  <Text size="xs" c="dimmed">
                    {t.autres(fichiers.length - APERCU)}
                  </Text>
                )}
                {aVerifier.length > 0 && (
                  <Alert color="orange" icon={<IconAlertTriangle size={18} />} title={t.aVerifier(aVerifier.length)}>
                    <List size="sm" spacing={2}>
                      {problemes.slice(0, 8).map((ligne) => (
                        <List.Item key={ligne}>{ligne}</List.Item>
                      ))}
                    </List>
                    {problemes.length > 8 && (
                      <Text size="xs" c="dimmed" mt={4}>
                        {t.autres(problemes.length - 8)}
                      </Text>
                    )}
                  </Alert>
                )}
                {absents.length > 0 && (
                  <Text size="sm" c="dimmed">
                    {t.absents(absents.length, quelques(absents))}
                  </Text>
                )}
                {sansMotDePasse.length > 0 && <Alert color="red">{t.motDePasseVide(quelques(sansMotDePasse.map((fichier) => fichier.nom)))}</Alert>}
                <Button leftSection={<IconUsers size={18} />} loading={enCours} disabled={fichiers.length === 0 || sansMotDePasse.length > 0} onClick={creer} w="fit-content">
                  {t.creer}
                </Button>
              </Stack>
            </Paper>
          )}

          {avecEnvoi && reperage && fichiers.length > 0 && (
            <Paper withBorder p="md" maw={720} data-envoi>
              <Stack gap="sm">
                <Group justify="space-between">
                  <Title order={5}>{t.envoi.titre}</Title>
                  {creation && (
                    <Badge size="lg" variant="light" style={{ textTransform: "none" }}>
                      {t.envoi.pret(envois.messages.length)}
                    </Badge>
                  )}
                </Group>
                {!creation ? (
                  <Text size="sm" c="dimmed">
                    {t.envoi.creerAvant}
                  </Text>
                ) : (
                  <>
                    {envois.sansAdresse.length > 0 && <Alert color="orange">{t.envoi.sansAdresse(envois.sansAdresse.length, quelques(envois.sansAdresse))}</Alert>}
                    {!pretAEnvoyer && (
                      <Alert color="orange">
                        <Group gap="sm" justify="space-between" wrap="nowrap">
                          <Text size="sm">{t.envoi.aRegler}</Text>
                          <Button variant="light" size="xs" leftSection={<IconSettings size={16} />} onClick={() => setReglageMessagerie(true)} style={{ flexShrink: 0 }}>
                            {t.envoi.regler}
                          </Button>
                        </Group>
                      </Alert>
                    )}
                    <Group align="flex-end" gap="sm">
                      <TextInput
                        label={t.envoi.essai}
                        description={t.envoi.essaiAide}
                        placeholder={compteMessagerie && parMessagerie ? compteMessagerie.adresse : undefined}
                        value={adresseEssai}
                        onChange={(e) => setAdresseEssai(e.currentTarget.value)}
                        style={{ flex: 1 }}
                      />
                      <Button
                        variant="light"
                        leftSection={<IconMail size={18} />}
                        disabled={!adresseEssaiEffective.includes("@") || envois.messages.length === 0 || envoiEnCours || !pretAEnvoyer}
                        onClick={() => void envoyer([{ ...envois.messages[0], message: { ...envois.messages[0].message, a: adresseEssaiEffective } }])}
                      >
                        {t.envoi.essayer[modeEnvoi]}
                      </Button>
                    </Group>
                    <Button leftSection={<IconSend size={18} />} loading={envoiEnCours} disabled={envois.messages.length === 0 || !pretAEnvoyer} onClick={envoyerTout} w="fit-content">
                      {t.envoi.envoyer[modeEnvoi](envois.messages.length)}
                    </Button>
                    {envoiEnCours && progression && (
                      <div>
                        <Progress value={(100 * progression[0]) / Math.max(1, progression[1])} />
                        <Text size="xs" c="dimmed" mt={4}>
                          {t.envoi.progression(progression[0], progression[1])}
                        </Text>
                      </div>
                    )}
                    {echecs.length > 0 && (
                      <Alert color="red" title={t.envoi.echecs(echecs.length)}>
                        <List size="sm" spacing={2}>
                          {echecs.slice(0, 8).map((ligne) => (
                            <List.Item key={ligne}>{ligne}</List.Item>
                          ))}
                        </List>
                      </Alert>
                    )}
                  </>
                )}
              </Stack>
            </Paper>
          )}
        </>
      )}

      <Modal opened={nomEnregistrement !== null} onClose={() => setNomEnregistrement(null)} title={t.enchainement.enregistrer}>
        <Stack gap="sm">
          <TextInput label={t.enchainement.nom} value={nomEnregistrement ?? ""} onChange={(e) => setNomEnregistrement(e.currentTarget.value)} description={nomExistant ? t.enchainement.remplace : undefined} data-autofocus />
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setNomEnregistrement(null)}>
              {t.enchainement.annuler}
            </Button>
            <Button onClick={() => void enregistrer()} disabled={!nomEnregistrement?.trim()}>
              {t.enchainement.valider}
            </Button>
          </Group>
        </Stack>
      </Modal>

      <FenetreMessagerie
        ouvert={reglageMessagerie}
        compte={messagerie.compte}
        motDePasseGarde={motDePasseGarde}
        onFermer={() => setReglageMessagerie(false)}
        onEnregistrer={async (compte, garde) => {
          const nouveaux = { ...messagerie, compte };
          await enregistrerMessagerie(JSON.stringify(nouveaux, null, 2));
          setMessagerie(nouveaux);
          setMotDePasseGarde(garde);
        }}
      />

      <Modal opened={confirmation !== null} onClose={() => setConfirmation(null)} title={confirmation?.titre}>
        <Stack gap="md">
          <Text size="sm">{confirmation?.texte}</Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setConfirmation(null)}>
              {t.envoi.annuler}
            </Button>
            <Button
              color={confirmation?.couleur}
              onClick={() => {
                confirmation?.action();
                setConfirmation(null);
              }}
            >
              {confirmation?.bouton}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  );
}
