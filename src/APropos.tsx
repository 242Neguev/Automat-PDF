import { ActionIcon, CopyButton, Divider, Group, Image, Modal, Stack, Text, Title, Tooltip } from "@mantine/core";
import { IconBrandGithub, IconCheck, IconCopy, IconMail, IconPhone } from "@tabler/icons-react";
import nomEauSeche from "./assets/eau-seche/nom.png";
import symboleEauSeche from "./assets/eau-seche/symbole.png";
import { version } from "../package.json";
import type { Confidentialite } from "./coeur";
import { ReglagesConfidentialite, type Reglage } from "./Confidentialite";
import type { MiseAJour } from "./MiseAJour";
import { textes } from "./textes";

const t = textes.aPropos;

/** Coordonnée de l'éditeur, avec un bouton pour la copier. */
function Coordonnee({ icone: Icone, valeur }: { icone: typeof IconPhone; valeur: string }) {
  return (
    <Group gap={6} wrap="nowrap">
      <Icone size={16} stroke={1.5} />
      <Text size="sm">{valeur}</Text>
      <CopyButton value={valeur} timeout={1500}>
        {({ copied, copy }) => (
          <Tooltip label={copied ? t.copie : t.copier(valeur)} withArrow>
            <ActionIcon variant="subtle" color={copied ? "teal" : "gray"} size="sm" aria-label={t.copier(valeur)} onClick={copy}>
              {copied ? <IconCheck size={14} /> : <IconCopy size={14} />}
            </ActionIcon>
          </Tooltip>
        )}
      </CopyButton>
    </Group>
  );
}

interface Props {
  ouvert: boolean;
  fermer: () => void;
  /** null tant que les réglages ne sont pas lus. */
  confidentialite: Confidentialite | null;
  regler: (reglage: Reglage) => void;
  miseAJour: MiseAJour;
}

export default function APropos({ ouvert, fermer, confidentialite, regler, miseAJour }: Props) {
  return (
    <Modal opened={ouvert} onClose={fermer} title={t.titre} centered size="lg">
      <Stack gap="md">
        <div>
          <Title order={3}>{textes.application}</Title>
          <Text c="dimmed" size="sm">
            {t.version(version)}
          </Text>
          <Text mt="xs">{t.description}</Text>
        </div>
        {confidentialite && (
          <>
            <Divider label={textes.confidentialite.titre} labelPosition="left" />
            <ReglagesConfidentialite confidentialite={confidentialite} regler={regler} miseAJour={miseAJour} />
          </>
        )}
        <Divider label={t.editeur} labelPosition="left" />
        {/* La charte Eau sèche impose une zone de protection autour du symbole : on le laisse respirer. */}
        <Group gap="xl" py="sm" justify="center">
          <Image src={symboleEauSeche} alt="" h={64} w="auto" fit="contain" />
          <Image src={nomEauSeche} alt="Eau sèche" h={30} w="auto" fit="contain" />
        </Group>
        <Group gap="lg" justify="center">
          <Coordonnee icone={IconPhone} valeur={t.telephone} />
          <Coordonnee icone={IconMail} valeur={t.email} />
        </Group>
        <Divider />
        <Text size="xs" c="dimmed">
          {t.bibliotheque}
        </Text>
        <Text size="xs" c="dimmed">
          {t.droits}
        </Text>
        <Group justify="center">
          <Coordonnee icone={IconBrandGithub} valeur={t.codeSource} />
        </Group>
      </Stack>
    </Modal>
  );
}
