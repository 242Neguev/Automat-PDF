import { useState } from "react";
import { Button, Checkbox, Divider, Group, List, Modal, Stack, Switch, Text } from "@mantine/core";
import { IconDownload, IconRefresh } from "@tabler/icons-react";
import type { Confidentialite } from "./coeur";
import { etatEnClair, type MiseAJour } from "./MiseAJour";
import { textes } from "./textes";

const t = textes.confidentialite;

/** Réglages de confidentialité à changer : seuls ceux donnés changent. */
export interface Reglage {
  misesAJour?: boolean;
  statistiques?: boolean;
}

/**
 * Question posée au premier lancement : envoyer ou non des statistiques anonymes. Rien n'est
 * envoyé avant la réponse ; « Oui » et « Non » ont le même poids. La recherche des mises à jour,
 * qui n'envoie rien sur la personne, est rappelée et peut être décochée.
 */
export function QuestionStatistiques({ ouverte, repondre }: { ouverte: boolean; repondre: (reponse: Required<Reglage>) => void }) {
  const [misesAJour, setMisesAJour] = useState(true);
  return (
    <Modal opened={ouverte} onClose={() => {}} withCloseButton={false} closeOnEscape={false} closeOnClickOutside={false} centered size="lg" title={t.question}>
      <Stack gap="md" data-question-statistiques>
        <Text>{t.pourquoi}</Text>
        <div>
          <Text size="sm" fw={600}>
            {t.envoyeTitre}
          </Text>
          <List size="sm" spacing={2}>
            {t.envoye.map((ligne) => (
              <List.Item key={ligne}>{ligne}</List.Item>
            ))}
          </List>
        </div>
        <Text size="sm">
          <Text span fw={600}>
            {t.jamaisTitre}
          </Text>{" "}
          {t.jamais}
        </Text>
        <Text size="sm" c="dimmed">
          {t.changerDAvis}
        </Text>
        <Divider />
        <Checkbox
          checked={misesAJour}
          onChange={(evenement) => {
            const coche = evenement.currentTarget.checked;
            setMisesAJour(coche);
          }}
          label={t.misesAJour}
          description={t.misesAJourDetail}
        />
        <Group grow>
          <Button variant="default" size="md" onClick={() => repondre({ statistiques: false, misesAJour })}>
            {t.non}
          </Button>
          <Button variant="default" size="md" onClick={() => repondre({ statistiques: true, misesAJour })}>
            {t.oui}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}

/** Dans « À propos » : recherche des mises à jour et statistiques, réglables à tout moment. */
export function ReglagesConfidentialite({ confidentialite, regler, miseAJour }: { confidentialite: Confidentialite; regler: (reglage: Reglage) => void; miseAJour: MiseAJour }) {
  const { etat } = miseAJour;
  const enCours = etat.etape === "recherche" || etat.etape === "telechargement" || etat.etape === "installation";
  const etatLisible = etatEnClair(etat);
  return (
    <Stack gap="sm" data-reglages-confidentialite>
      <Switch
        checked={confidentialite.misesAJour}
        onChange={(evenement) => {
          const coche = evenement.currentTarget.checked;
          regler({ misesAJour: coche });
        }}
        label={t.misesAJour}
        description={t.misesAJourDetail}
      />
      <Group gap="xs">
        <Button size="xs" variant="default" leftSection={<IconRefresh size={16} />} loading={etat.etape === "recherche"} disabled={enCours} onClick={() => void miseAJour.chercher(true)}>
          {textes.miseAJour.rechercher}
        </Button>
        {etat.etape === "disponible" && (
          <Button size="xs" leftSection={<IconDownload size={16} />} onClick={() => void miseAJour.installer()}>
            {textes.miseAJour.mettreAJour}
          </Button>
        )}
      </Group>
      {etatLisible && (
        <Text size="sm" c={etat.etape === "echec" ? "orange.8" : undefined} data-etat-mise-a-jour={etat.etape}>
          {etatLisible}
        </Text>
      )}
      {confidentialite.statistiquesPossibles && (
        <Switch
          checked={confidentialite.statistiques === true}
          onChange={(evenement) => {
            const coche = evenement.currentTarget.checked;
            regler({ statistiques: coche });
          }}
          label={t.statistiques}
          description={t.statistiquesDetail}
        />
      )}
    </Stack>
  );
}
