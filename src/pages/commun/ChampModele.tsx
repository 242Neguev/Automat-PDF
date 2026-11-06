import { useRef } from "react";
import { Badge, Group, Stack, Text, Textarea, TextInput } from "@mantine/core";
import { textes } from "../../textes";

const t = textes.modele;

interface Props {
  label: string;
  description?: string;
  valeur: string;
  onValeur: (valeur: string) => void;
  /** Colonnes de la liste importée, à ajouter d'un clic ; vide sans liste. */
  colonnes: string[];
  /** Ce que donne le modèle pour la première personne (« Bulletin DUPONT Jean.pdf »). */
  exemple?: string;
  multiligne?: boolean;
}

/**
 * Texte à balises ({{NOM}}, {{Prénom}}…) : les colonnes de la liste s'y ajoutent d'un clic, là où
 * se trouve le curseur, sans rien taper entre accolades.
 */
export default function ChampModele({ label, description, valeur, onValeur, colonnes, exemple, multiligne }: Props) {
  const champ = useRef<HTMLInputElement | HTMLTextAreaElement | null>(null);

  function ajouter(colonne: string) {
    const element = champ.current;
    const debut = element?.selectionStart ?? valeur.length;
    const fin = element?.selectionEnd ?? valeur.length;
    const avant = valeur.slice(0, debut);
    const insere = `${avant && !/\s$/.test(avant) ? " " : ""}{{${colonne}}}`;
    onValeur(avant + insere + valeur.slice(fin));
    // Le curseur se place après la balise ajoutée, pour en ajouter une autre à la suite.
    const position = avant.length + insere.length;
    requestAnimationFrame(() => {
      element?.focus();
      element?.setSelectionRange(position, position);
    });
  }

  const commun = { label, description, value: valeur, onChange: (e: { currentTarget: { value: string } }) => onValeur(e.currentTarget.value) };
  return (
    <Stack gap={6}>
      {multiligne ? (
        <Textarea ref={(element) => void (champ.current = element)} autosize minRows={4} {...commun} />
      ) : (
        <TextInput ref={(element) => void (champ.current = element)} {...commun} />
      )}
      {colonnes.length > 0 && (
        <Group gap={6} data-colonnes-modele>
          <Text size="xs" c="dimmed">
            {t.inserer}
          </Text>
          {colonnes.map((colonne) => (
            <Badge key={colonne} component="button" type="button" variant="light" style={{ cursor: "pointer", textTransform: "none" }} onClick={() => ajouter(colonne)}>
              + {colonne}
            </Badge>
          ))}
        </Group>
      )}
      {exemple && (
        <Text size="sm" data-exemple-modele>
          {t.exemple(exemple)}
        </Text>
      )}
    </Stack>
  );
}
