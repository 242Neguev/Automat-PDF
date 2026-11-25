import { useEffect, useMemo, useRef, useState } from "react";
import { ActionIcon, Text, Tooltip, UnstyledButton } from "@mantine/core";
import { IconChevronDown, IconChevronRight, IconExternalLink } from "@tabler/icons-react";
import type { DestinationPdf, Signet } from "../../coeur";
import { aDesSousSignets, ouvertsAuDepart, signetCourant, signetsVisibles } from "../../donnees/signets";
import { textes } from "../../textes";

const t = textes.lecteur;

interface Props {
  signets: Signet[];
  /** La page lue, pour mettre en valeur son signet. */
  courante: number;
  onSuivre: (destination: DestinationPdf | null, adresse: string | null) => void;
}

/** Le sommaire du PDF : un clic mène à la page du signet ; ceux qui ont des sous-signets se déplient. */
export default function ColonneSignets({ signets, courante, onSuivre }: Props) {
  const [deplies, setDeplies] = useState<Set<number>>(() => ouvertsAuDepart(signets));
  useEffect(() => setDeplies(ouvertsAuDepart(signets)), [signets]);
  const visibles = useMemo(() => signetsVisibles(signets, deplies), [signets, deplies]);
  const actuel = useMemo(() => signetCourant(signets, courante), [signets, courante]);
  const elements = useRef<Record<number, HTMLDivElement | null>>({});
  useEffect(() => {
    if (actuel !== null) elements.current[actuel]?.scrollIntoView({ block: "nearest" });
  }, [actuel]);

  function basculer(rang: number) {
    setDeplies((avant) => {
      const apres = new Set(avant);
      if (apres.has(rang)) apres.delete(rang);
      else apres.add(rang);
      return apres;
    });
  }

  return (
    <div className="colonne-signets" data-signets-lecteur aria-label={t.signetsAide}>
      {visibles.map((rang) => {
        const signet = signets[rang];
        const parent = aDesSousSignets(signets, rang);
        const deplie = deplies.has(rang);
        return (
          <div
            key={rang}
            ref={(element) => {
              elements.current[rang] = element;
            }}
            className="signet"
            data-courant={rang === actuel || undefined}
            style={{ paddingLeft: signet.niveau * 12 }}
          >
            {parent ? (
              <ActionIcon variant="subtle" color="gray" size={22} onClick={() => basculer(rang)} aria-label={deplie ? t.replier : t.deplier} aria-expanded={deplie}>
                {deplie ? <IconChevronDown size={14} /> : <IconChevronRight size={14} />}
              </ActionIcon>
            ) : (
              <span className="signet__vide" />
            )}
            <UnstyledButton className="signet__titre" onClick={() => onSuivre(signet.destination, signet.adresse)} disabled={!signet.destination && !signet.adresse}>
              <Text size="xs" lh={1.35} span>
                {signet.titre || "—"}
              </Text>
              {signet.adresse && (
                <Tooltip label={signet.adresse} withArrow openDelay={400}>
                  <IconExternalLink size={12} className="signet__externe" />
                </Tooltip>
              )}
            </UnstyledButton>
          </div>
        );
      })}
    </div>
  );
}
