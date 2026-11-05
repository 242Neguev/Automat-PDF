import { useEffect, useState } from "react";
import { Alert, Button, Checkbox, Group, Modal, NumberInput, PasswordInput, SegmentedControl, Stack, Text, TextInput } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconCheck, IconInfoCircle, IconPlugConnected } from "@tabler/icons-react";
import { enregistrerMotDePasseMessagerie, testerMessagerie, type CompteMessagerie } from "../../coeur";
import { avecAdresse, avecChiffrement, COMPTE_VIDE, manqueAuCompte, suggestion, type Chiffrement } from "../../donnees/messagerie";
import { textes } from "../../textes";
import { signalerErreur } from "../commun/signalerErreur";

const t = textes.messagerie;

interface Props {
  ouvert: boolean;
  compte: CompteMessagerie | null;
  /** Un mot de passe est déjà gardé par le système. */
  motDePasseGarde: boolean;
  onFermer: () => void;
  /** Le compte à enregistrer ; le mot de passe, lui, est déjà rangé dans le coffre du système. */
  onEnregistrer: (compte: CompteMessagerie, motDePasseGarde: boolean) => Promise<void>;
}

/** Réglages de la messagerie : adresse, serveur d'envoi, chiffrement, identifiant et mot de passe. */
export default function ReglagesMessagerie({ ouvert, compte, motDePasseGarde, onFermer, onEnregistrer }: Props) {
  const [brouillon, setBrouillon] = useState<CompteMessagerie>(compte ?? COMPTE_VIDE);
  const [motDePasse, setMotDePasse] = useState("");
  const [oublier, setOublier] = useState(false);
  const [test, setTest] = useState(false);
  const [enregistrement, setEnregistrement] = useState(false);

  useEffect(() => {
    if (!ouvert) return;
    setBrouillon(compte ?? COMPTE_VIDE);
    setMotDePasse("");
    setOublier(false);
  }, [ouvert, compte]);

  const regler = (partiel: Partial<CompteMessagerie>) => setBrouillon((actuel) => ({ ...actuel, ...partiel }));
  const conseil = suggestion(brouillon.adresse)?.conseil ?? null;
  const manque = manqueAuCompte(brouillon);
  const sansMotDePasse = brouillon.chiffrement === "aucun" || !brouillon.identifiant.trim();
  const propre = (c: CompteMessagerie): CompteMessagerie => ({ ...c, serveur: c.serveur.trim(), adresse: c.adresse.trim(), identifiant: c.identifiant.trim(), nom: c.nom.trim() });

  async function tester() {
    setTest(true);
    try {
      await testerMessagerie(propre(brouillon), sansMotDePasse ? null : motDePasse || null);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.testReussi, message: sansMotDePasse ? t.testReussiSansMotDePasse : t.testReussiMessage, autoClose: 8000 });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setTest(false);
    }
  }

  async function enregistrer() {
    setEnregistrement(true);
    try {
      let garde = motDePasseGarde;
      if (sansMotDePasse || oublier) {
        if (garde) await enregistrerMotDePasseMessagerie("");
        garde = false;
      }
      if (!sansMotDePasse && motDePasse) {
        await enregistrerMotDePasseMessagerie(motDePasse);
        garde = true;
      }
      await onEnregistrer(propre(brouillon), garde);
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.enregistre, message: "" });
      onFermer();
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnregistrement(false);
    }
  }

  return (
    <Modal opened={ouvert} onClose={onFermer} title={t.titre} size="lg">
      <Stack gap="sm" data-reglages-messagerie>
        <Text size="sm" c="dimmed">
          {t.intro}
        </Text>
        <TextInput label={t.adresse} placeholder="rh@exemple.com" value={brouillon.adresse} onChange={(e) => setBrouillon((actuel) => avecAdresse(actuel, e.currentTarget.value))} data-autofocus />
        <TextInput label={t.nom} description={t.nomAide} value={brouillon.nom} onChange={(e) => regler({ nom: e.currentTarget.value })} />
        <Group align="flex-start" gap="sm" grow>
          <TextInput label={t.serveur} placeholder="smtp.exemple.com" value={brouillon.serveur} onChange={(e) => regler({ serveur: e.currentTarget.value })} style={{ flexGrow: 3 }} />
          <NumberInput label={t.port} min={1} max={65535} allowDecimal={false} value={brouillon.port} onChange={(v) => regler({ port: typeof v === "number" ? v : 0 })} style={{ flexGrow: 1 }} />
        </Group>
        <Text size="xs" c="dimmed" mt={-6}>
          {t.serveurAide}
        </Text>
        <div>
          <Text size="sm" fw={500} mb={4}>
            {t.chiffrement}
          </Text>
          <SegmentedControl
            value={brouillon.chiffrement}
            onChange={(v) => setBrouillon((actuel) => avecChiffrement(actuel, v as Chiffrement))}
            data={(["starttls", "ssl", "aucun"] as const).map((value) => ({ value, label: t.chiffrements[value] }))}
          />
          <Text size="xs" c="dimmed" mt={4}>
            {t.chiffrementAide[brouillon.chiffrement]}
          </Text>
        </div>
        {brouillon.chiffrement !== "aucun" && (
          <>
            <TextInput label={t.identifiant} description={t.identifiantAide} value={brouillon.identifiant} onChange={(e) => regler({ identifiant: e.currentTarget.value })} />
            <PasswordInput
              label={t.motDePasse}
              description={motDePasseGarde && !oublier ? t.motDePasseGarde : undefined}
              value={motDePasse}
              onChange={(e) => setMotDePasse(e.currentTarget.value)}
              disabled={oublier}
              autoComplete="off"
            />
            {motDePasseGarde && <Checkbox label={t.oublier} checked={oublier} onChange={(e) => setOublier(e.currentTarget.checked)} />}
            {conseil && (
              <Alert color="blue" icon={<IconInfoCircle size={18} />}>
                {t.conseils[conseil]}
              </Alert>
            )}
          </>
        )}
        {manque && brouillon.adresse.trim() && (
          <Text size="sm" c="orange">
            {t.manque[manque]}
          </Text>
        )}
        <Group justify="space-between" mt="xs">
          <Button variant="light" leftSection={<IconPlugConnected size={18} />} loading={test} disabled={manque !== null || enregistrement} onClick={() => void tester()}>
            {t.tester}
          </Button>
          <Group gap="sm">
            <Button variant="default" onClick={onFermer}>
              {t.annuler}
            </Button>
            <Button loading={enregistrement} disabled={manque !== null || test} onClick={() => void enregistrer()}>
              {t.enregistrer}
            </Button>
          </Group>
        </Group>
      </Stack>
    </Modal>
  );
}
