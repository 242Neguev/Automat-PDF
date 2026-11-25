// Automat-PDF, lecteur de PDF par défaut : proposé dans le lecteur (une fois par session, tant que
// l'utilisateur ne l'a ni choisi ni refusé), et dans « À propos », à tout moment. Le système demande
// toujours l'accord de l'utilisateur : fenêtre de choix de Windows, confirmation de macOS.
import { useEffect, useState } from "react";
import { Alert, Button, Divider, Group, Text } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCircleCheck, IconFileText } from "@tabler/icons-react";
import { devenirLecteurParDefaut, lecteurParDefaut, type EtatLecteurPdf } from "../../coeur";
import { aProposer } from "../../donnees/lecteur-par-defaut";
import { MAC } from "../../systeme";
import { textes } from "../../textes";
import { signalerErreur } from "../commun/signalerErreur";

const t = textes.lecteurParDefaut;
/** Préférence gardée : « Ne plus proposer ». */
const CLE_REFUS = "automat-pdf.lecteur-par-defaut";
const INCONNU: EtatLecteurPdf = { possible: false, parDefaut: false };

/** L'état, demandé au système une fois (puis après chaque choix). */
let demande: Promise<EtatLecteurPdf> | null = null;
/** Événement : le choix a changé (« À propos » ou le lecteur) ; l'autre se met à jour. */
const CHOIX_CHANGE = "automat-pdf:lecteur-par-defaut";
/** « Plus tard » : la proposition ne revient pas avant le prochain lancement. */
let masqueeCetteSession = false;

function lireEtat(relire = false): Promise<EtatLecteurPdf> {
  if (!demande || relire) demande = lecteurParDefaut().catch(() => INCONNU);
  return demande;
}

function refusee(): boolean {
  try {
    return localStorage.getItem(CLE_REFUS) === "non";
  } catch {
    return false;
  }
}

/** Demande au système, puis dit ce qu'il en est ; renvoie l'état après coup (null : en échec). */
async function choisirAutomatPdf(): Promise<EtatLecteurPdf | null> {
  try {
    const { etat, parametres } = await devenirLecteurParDefaut();
    demande = Promise.resolve(etat);
    window.dispatchEvent(new CustomEvent<EtatLecteurPdf>(CHOIX_CHANGE, { detail: etat }));
    if (etat.parDefaut) notifications.show({ color: "teal", message: t.reussi, autoClose: 6000 });
    else if (parametres) notifications.show({ color: "eau", message: t.parametres, autoClose: 20000 });
    else notifications.show({ color: "gray", message: t.pasEncore, autoClose: 9000 });
    return etat;
  } catch (e) {
    signalerErreur(e);
    return null;
  }
}

/** Dans le lecteur, au-dessus des pages : la proposition, avec « Plus tard » et « Ne plus proposer ». */
export function PropositionLecteurParDefaut() {
  const [etat, setEtat] = useState<EtatLecteurPdf | null>(null);
  const [masquee, setMasquee] = useState(masqueeCetteSession);
  const [enCours, setEnCours] = useState(false);
  useEffect(() => {
    if (masquee) return;
    let actuelle = true;
    void lireEtat().then((lu) => actuelle && setEtat(lu));
    const suivre = (evenement: Event) => setEtat((evenement as CustomEvent<EtatLecteurPdf>).detail);
    window.addEventListener(CHOIX_CHANGE, suivre);
    return () => {
      actuelle = false;
      window.removeEventListener(CHOIX_CHANGE, suivre);
    };
  }, [masquee]);
  if (!aProposer(etat, refusee(), masquee)) return null;

  const masquer = () => {
    masqueeCetteSession = true;
    setMasquee(true);
  };
  async function choisir() {
    setEnCours(true);
    const apres = await choisirAutomatPdf();
    setEnCours(false);
    if (apres) setEtat(apres);
    masquer();
  }
  function jamais() {
    try {
      localStorage.setItem(CLE_REFUS, "non");
    } catch {
      // la proposition reviendra au prochain lancement
    }
    masquer();
  }

  return (
    <Alert color="eau" variant="light" icon={<IconFileText size={20} />} title={t.titre} withCloseButton onClose={masquer} py={8} data-proposition-lecteur>
      <Text size="sm">{t.texte}</Text>
      <Text size="xs" c="dimmed">
        {MAC ? t.aideMac : t.aideWindows}
      </Text>
      <Group gap="xs" mt={8}>
        <Button size="xs" radius="xl" color="eau" loading={enCours} onClick={() => void choisir()}>
          {t.choisir}
        </Button>
        <Button size="xs" radius="xl" variant="default" onClick={masquer}>
          {t.plusTard}
        </Button>
        <Button size="xs" radius="xl" variant="subtle" color="gray" onClick={jamais}>
          {t.jamais}
        </Button>
      </Group>
    </Alert>
  );
}

/** Dans « À propos » : qui ouvre les PDF, et le bouton pour choisir Automat-PDF (même après un refus). */
export function ReglageLecteurParDefaut() {
  const [etat, setEtat] = useState<EtatLecteurPdf | null>(null);
  const [enCours, setEnCours] = useState(false);
  useEffect(() => {
    let actuelle = true;
    void lireEtat(true).then((lu) => actuelle && setEtat(lu));
    return () => {
      actuelle = false;
    };
  }, []);
  if (!etat?.possible) return null;

  async function choisir() {
    setEnCours(true);
    const apres = await choisirAutomatPdf();
    setEnCours(false);
    if (apres) setEtat(apres);
  }

  return (
    <>
      <Divider label={t.reglage} labelPosition="left" />
      <Group justify="space-between" wrap="nowrap" gap="md" data-reglage-lecteur>
        <Group gap={8} wrap="nowrap">
          {etat.parDefaut ? <IconCircleCheck size={18} color="var(--mantine-color-teal-6)" /> : <IconFileText size={18} />}
          <Text size="sm">{etat.parDefaut ? t.estParDefaut : t.nEstPas}</Text>
        </Group>
        {!etat.parDefaut && (
          <Button size="xs" radius="xl" color="eau" variant="light" loading={enCours} onClick={() => void choisir()}>
            {t.choisir}
          </Button>
        )}
      </Group>
    </>
  );
}
