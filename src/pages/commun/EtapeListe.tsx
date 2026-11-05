import { Alert, Badge, Button, Group, Paper, ScrollArea, Select, Stack, Table, Text, Title } from "@mantine/core";
import { IconFileSpreadsheet, IconListDetails } from "@tabler/icons-react";
import type { Anomalie } from "../../donnees/controle";
import type { Correspondances } from "../../donnees/correspondances";
import type { Liste } from "../../donnees/liste";
import { textes } from "../../textes";
import IconeGravite from "./IconeGravite";

const t = textes.liste;
const LIGNES_APERCU = 8;

function message(anomalie: Anomalie): string {
  const { messages } = t.controle;
  const n = anomalie.lignes.length;
  switch (anomalie.type) {
    case "colonne-vide":
      return messages["colonne-vide"](anomalie.balise);
    case "doublon":
      return messages.doublon(anomalie.balise);
    case "email":
    case "telephone":
      return messages[anomalie.type](anomalie.balise, n);
    case "vide":
      return messages.vide(anomalie.balise, n, anomalie.gravite === "info");
  }
}

/** Anomalies de la liste, avec les lignes à corriger dans le fichier. */
function ControleDonnees({ anomalies }: { anomalies: Anomalie[] }) {
  return (
    <Stack gap="xs" data-controle-liste>
      <div>
        <Title order={5}>{t.controle.titre}</Title>
        {anomalies.some((anomalie) => anomalie.gravite === "attention") && (
          <Text size="sm" c="dimmed">
            {t.controle.aide}
          </Text>
        )}
      </div>
      {anomalies.length === 0 ? (
        <Group gap="xs">
          <IconeGravite gravite={null} />
          <Text size="sm">{t.controle.aucune}</Text>
        </Group>
      ) : (
        anomalies.map((anomalie) => (
          <Group key={`${anomalie.type}:${anomalie.balise}:${anomalie.lignes.join(",")}`} gap="xs" wrap="nowrap" align="flex-start">
            <IconeGravite gravite={anomalie.gravite} />
            <div>
              <Text size="sm">{message(anomalie)}</Text>
              {(anomalie.lignes.length > 0 || anomalie.exemple) && (
                <Text size="xs" c="dimmed" dir="auto">
                  {[anomalie.lignes.length > 0 && t.controle.lignes(anomalie.lignes), anomalie.exemple && t.controle.exemple(anomalie.exemple)]
                    .filter(Boolean)
                    .join(" · ")}
                </Text>
              )}
            </div>
          </Group>
        ))
      )}
    </Stack>
  );
}

interface Props {
  liste: Liste | null;
  feuilles: string[];
  balises: string[];
  correspondances: Correspondances;
  /** Valeur nettoyée de chaque balise pour la première ligne, pour vérifier d'un coup d'œil. */
  exemple: Record<string, string>;
  anomalies: Anomalie[];
  onImporter: () => void;
  onExemple: () => void;
  onFeuille: (feuille: string) => void;
  onCorrespondance: (balise: string, colonne: string | null) => void;
}

export default function EtapeListe({ liste, feuilles, balises, correspondances, exemple, anomalies, onImporter, onExemple, onFeuille, onCorrespondance }: Props) {
  return (
    <Stack gap="lg">
      <Group>
        <Button leftSection={<IconFileSpreadsheet size={18} />} onClick={onImporter}>
          {t.importer}
        </Button>
        <Button variant="light" leftSection={<IconListDetails size={18} />} onClick={onExemple}>
          {t.exemple}
        </Button>
      </Group>

      {!liste ? (
        <Text c="dimmed">{t.aucuneListe}</Text>
      ) : (
        <>
          <Paper withBorder p="md">
            <Group justify="space-between">
              <Group gap="sm">
                <IconFileSpreadsheet size={22} stroke={1.5} />
                <Text fw={500}>{liste.nomFichier}</Text>
                {feuilles.length > 1 && (
                  <Select
                    size="xs"
                    aria-label={t.feuille}
                    value={liste.feuille}
                    data={feuilles}
                    allowDeselect={false}
                    onChange={(feuille) => feuille && onFeuille(feuille)}
                  />
                )}
              </Group>
              <Badge size="lg" variant="light">
                {t.nbAgents(liste.lignes.length)}
              </Badge>
            </Group>
          </Paper>

          {liste.lignes.length === 0 ? (
            <Alert color="orange">{t.vide}</Alert>
          ) : (
            <>
              <div>
                <Title order={5}>{t.correspondancesTitre}</Title>
                <Text size="sm" c="dimmed">
                  {t.correspondancesAide}
                </Text>
              </div>
              <Table withTableBorder verticalSpacing="xs">
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th>{t.information}</Table.Th>
                    <Table.Th>{t.colonne}</Table.Th>
                    <Table.Th>{t.exemple1}</Table.Th>
                  </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                  {balises.map((balise) => (
                    <Table.Tr key={balise}>
                      <Table.Td>
                        <Group gap="xs" wrap="nowrap">
                          <Text fw={500}>{balise}</Text>
                          <Badge size="xs" variant="light" color={correspondances[balise] ? "eau" : "orange"}>
                            {correspondances[balise] ? t.reliee : t.aChoisir}
                          </Badge>
                        </Group>
                      </Table.Td>
                      <Table.Td w={280}>
                        <Select
                          size="xs"
                          placeholder={t.choisir}
                          value={correspondances[balise] ?? null}
                          data={liste.colonnes}
                          clearable
                          searchable
                          onChange={(colonne) => onCorrespondance(balise, colonne)}
                        />
                      </Table.Td>
                      <Table.Td>
                        <Text size="sm" dir="auto">
                          {exemple[balise]}
                        </Text>
                      </Table.Td>
                    </Table.Tr>
                  ))}
                </Table.Tbody>
              </Table>

              <ControleDonnees anomalies={anomalies} />

              <div>
                <Title order={5}>{t.apercuTitre}</Title>
                <Text size="sm" c="dimmed">
                  {t.lignesAffichees(Math.min(LIGNES_APERCU, liste.lignes.length), liste.lignes.length)}
                </Text>
              </div>
              <ScrollArea type="auto">
                <Table striped withTableBorder fz="xs">
                  <Table.Thead>
                    <Table.Tr>
                      <Table.Th>#</Table.Th>
                      {liste.colonnes.map((colonne) => (
                        <Table.Th key={colonne} style={{ whiteSpace: "nowrap" }}>
                          {colonne}
                        </Table.Th>
                      ))}
                    </Table.Tr>
                  </Table.Thead>
                  <Table.Tbody>
                    {liste.lignes.slice(0, LIGNES_APERCU).map((ligne) => (
                      <Table.Tr key={ligne.numero}>
                        <Table.Td c="dimmed">{ligne.numero}</Table.Td>
                        {liste.colonnes.map((colonne) => (
                          <Table.Td key={colonne} dir="auto" style={{ whiteSpace: "nowrap" }}>
                            {ligne.valeurs[colonne]}
                          </Table.Td>
                        ))}
                      </Table.Tr>
                    ))}
                  </Table.Tbody>
                </Table>
              </ScrollArea>
            </>
          )}
        </>
      )}
    </Stack>
  );
}
