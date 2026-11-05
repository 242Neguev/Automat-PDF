import { useEffect, useState } from "react";
import { Alert, Button, Group, Loader, Modal, Stack, Text } from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { IconAlertTriangle, IconCheck, IconDownload, IconExternalLink, IconInfoCircle, IconRefresh } from "@tabler/icons-react";
import { installerLibreoffice, ouvrirPageLibreoffice, type Convertisseurs } from "../../coeur";
import { besoinLibreoffice, dureeLisible } from "../../donnees/convertisseurs";
import { textes } from "../../textes";
import { signalerErreur } from "../commun/signalerErreur";

const t = textes.conversion.libreoffice;

interface Props {
  convertisseurs: Convertisseurs | null;
  /** Relire ce qui est installé : après l'installation, ou quand l'utilisateur l'a faite lui-même. */
  onVerifier: () => void;
}

/** LibreOffice absent : ce qu'il apporte, et de quoi l'installer par son installateur officiel, ou le télécharger. */
export default function LibreOffice({ convertisseurs, onVerifier }: Props) {
  const [confirmation, setConfirmation] = useState(false);
  const [debut, setDebut] = useState<number | null>(null);
  const [maintenant, setMaintenant] = useState(() => Date.now());

  useEffect(() => {
    if (debut === null) return;
    const minuterie = setInterval(() => setMaintenant(Date.now()), 1000);
    return () => clearInterval(minuterie);
  }, [debut]);

  const besoin = besoinLibreoffice(convertisseurs);
  if (!besoin || !convertisseurs) return null;

  async function installer() {
    setDebut(Date.now());
    setMaintenant(Date.now());
    try {
      await installerLibreoffice();
      notifications.show({ color: "teal", icon: <IconCheck size={18} />, title: t.installe, message: t.installeMessage, autoClose: 10000 });
      onVerifier();
    } catch (e) {
      signalerErreur(e);
    } finally {
      setDebut(null);
    }
  }

  async function telecharger() {
    try {
      await ouvrirPageLibreoffice();
    } catch (e) {
      signalerErreur(e);
    }
  }

  return (
    <Alert
      color={besoin === "manque" ? "orange" : "blue"}
      icon={besoin === "manque" ? <IconAlertTriangle size={18} /> : <IconInfoCircle size={18} />}
      title={t.titre[besoin]}
      data-libreoffice={besoin}
    >
      <Stack gap="xs">
        <Text size="sm">{t.texte[besoin]}</Text>
        {debut !== null ? (
          <Group gap="sm" wrap="nowrap">
            <Loader size="sm" />
            <Text size="sm">{t.enCours(dureeLisible((maintenant - debut) / 1000))}</Text>
          </Group>
        ) : (
          <Group gap="sm">
            {convertisseurs.installable && (
              <Button size="xs" leftSection={<IconDownload size={16} />} onClick={() => setConfirmation(true)}>
                {t.installer}
              </Button>
            )}
            <Button size="xs" variant={convertisseurs.installable ? "subtle" : "light"} leftSection={<IconExternalLink size={16} />} onClick={() => void telecharger()}>
              {t.telecharger}
            </Button>
            <Button size="xs" variant="subtle" color="gray" leftSection={<IconRefresh size={16} />} onClick={onVerifier}>
              {t.verifier}
            </Button>
          </Group>
        )}
      </Stack>
      <Modal opened={confirmation} onClose={() => setConfirmation(false)} title={t.confirmerTitre}>
        <Stack gap="md">
          <Text size="sm">{t.confirmerTexte}</Text>
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setConfirmation(false)}>
              {t.annuler}
            </Button>
            <Button
              onClick={() => {
                setConfirmation(false);
                void installer();
              }}
            >
              {t.confirmer}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Alert>
  );
}
