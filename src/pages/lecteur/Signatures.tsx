// Signatures numériques des PDF reçus (0.7.0) : un bandeau dit si le PDF est signé et si ses
// signatures sont valables ; le panneau détaille chacune (intégrité, modifications depuis, identité,
// date) ; la signature visible sur la page s'ouvre d'un clic. Tout est vérifié par le cœur.
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Alert, Button, Collapse, Drawer, Group, Stack, Text, ThemeIcon, UnstyledButton } from "@mantine/core";
import { IconAlertTriangle, IconChevronDown, IconCircleCheck, IconCircleX, IconClockHour4, IconEye, IconHistory, IconInfoCircle, IconShieldCheck, IconShieldX } from "@tabler/icons-react";
import { versionSignee, type SignaturePdf } from "../../coeur";
import { dateLisible, etatDe, etatGeneral, jourLisible, modificationsDe, nonPermises, parPaires, signaturesDeLaPage, type EtatSignature } from "../../donnees/signatures";
import { MAC } from "../../systeme";
import { textes } from "../../textes";
import { signalerErreur } from "../commun/signalerErreur";

const t = textes.lecteur.signaturesNumeriques;

const COULEUR: Record<EtatSignature, string> = { valide: "teal", attention: "orange", invalide: "red" };
const ICONE = { valide: IconShieldCheck, attention: IconAlertTriangle, invalide: IconShieldX };

/** Le nom du signataire (ou de l'autorité d'horodatage). */
const nomDe = (signature: SignaturePdf) => signature.signataire ?? t.inconnu;

/** « Signé par Jeanne Martin le 26 novembre 2026 à 09:12 ». */
function qui(signature: SignaturePdf): string {
  const nom = nomDe(signature);
  const titre = signature.sorte === "certification" ? t.certifiePar(nom) : signature.sorte === "horodatage" ? t.horodatePar(nom) : t.signePar(nom);
  const date = dateLisible(signature.date);
  return date ? `${titre} ${t.le(date)}` : titre;
}

/** La raison principale du verdict, en quelques mots. */
export function motifDe(signature: SignaturePdf): string {
  const m = t.motifs;
  if (signature.integrite === "alteree") return m.alteree;
  if (signature.integrite === "fausse") return m.fausse;
  if (signature.confiance === "revoquee") return m.revoquee;
  if (signature.sorte === "certification" && nonPermises(signature).length > 0) return m.nonPermises;
  if (signature.integrite === "nonPriseEnCharge") return m.nonPriseEnCharge;
  if (signature.integrite === "illisible") return m.illisible;
  if (nonPermises(signature).length > 0) return m.modifie;
  if (signature.confiance === "horsValidite") return m.horsValidite;
  if (signature.confiance !== "reconnue") return m.identite;
  if (signature.empreinteAncienne) return m.ancienne;
  if (signature.horodatage && !signature.horodatage.valide) return m.horodatage;
  return m.valide;
}

interface PropsBandeau {
  signatures: SignaturePdf[];
  onDetails: () => void;
}

/** En haut du lecteur : le PDF est signé, et ce que valent ses signatures. */
export function BandeauSignatures({ signatures, onDetails }: PropsBandeau) {
  const etat = etatGeneral(signatures);
  const Icone = ICONE[etat];
  let message: string;
  if (signatures.length === 1) {
    message = `${qui(signatures[0])} · ${motifDe(signatures[0])}`;
  } else {
    const invalides = signatures.filter((s) => etatDe(s) === "invalide").length;
    const attention = signatures.filter((s) => etatDe(s) === "attention").length;
    const bilan = invalides > 0 ? t.invalides(invalides) : attention > 0 ? t.attention(attention) : t.toutesValides;
    message = `${t.plusieurs(signatures.length)} · ${bilan}`;
  }
  return (
    <Alert color={COULEUR[etat]} variant="light" icon={<Icone size={20} />} py={8} mb="xs" data-bandeau-signatures={etat}>
      <Group justify="space-between" gap="sm" wrap="nowrap">
        <Text size="sm" style={{ flex: 1 }}>
          {message}
        </Text>
        <Button size="xs" variant="default" leftSection={<IconInfoCircle size={15} />} onClick={onDetails} style={{ flexShrink: 0 }} data-details-signatures>
          {t.details}
        </Button>
      </Group>
    </Alert>
  );
}

/** Une ligne du verdict : bonne, à regarder, mauvaise, ou simple information. */
function Ligne({ sorte, children }: { sorte: "bon" | "attention" | "mauvais" | "info"; children: ReactNode }) {
  const [Icone, couleur] =
    sorte === "bon" ? [IconCircleCheck, "teal"] : sorte === "attention" ? [IconAlertTriangle, "orange"] : sorte === "mauvais" ? [IconCircleX, "red"] : [IconInfoCircle, "gray"];
  return (
    <Group gap={8} wrap="nowrap" align="flex-start">
      <ThemeIcon size={20} radius="xl" variant="light" color={couleur} mt={1} style={{ flexShrink: 0 }}>
        <Icone size={14} />
      </ThemeIcon>
      <Text size="sm" component="div" style={{ flex: 1 }}>
        {children}
      </Text>
    </Group>
  );
}

interface PropsCarte {
  signature: SignaturePdf;
  enAvant: boolean;
  onVoir?: () => void;
  onVersion?: () => void;
}

function CarteSignature({ signature, enAvant, onVoir, onVersion }: PropsCarte) {
  const p = t.panneau;
  const [certificat, setCertificat] = useState(false);
  const carte = useRef<HTMLDivElement>(null);
  const etat = etatDe(signature);
  const Icone = ICONE[etat];
  const nom = nomDe(signature);
  const titre = signature.sorte === "certification" ? p.certification(nom) : signature.sorte === "horodatage" ? p.horodatage(nom) : p.signature(nom);
  const modifications = modificationsDe(signature);
  const refusees = nonPermises(signature);
  const emetteur = signature.certificat?.emetteur ?? null;

  useEffect(() => {
    if (enAvant) carte.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [enAvant]);

  return (
    <div ref={carte} className="carte-signature" data-en-avant={enAvant || undefined} data-etat={etat} data-signature={signature.champ}>
      <Group gap="sm" wrap="nowrap" align="flex-start">
        <ThemeIcon size={34} radius="xl" variant="light" color={COULEUR[etat]}>
          <Icone size={20} />
        </ThemeIcon>
        <div style={{ flex: 1, minWidth: 0 }}>
          <Text fw={600}>{titre}</Text>
          <Text size="sm" c="dimmed">
            {dateLisible(signature.date) ?? ""}
          </Text>
          <Text size="sm" c={COULEUR[etat]} fw={500}>
            {motifDe(signature)}
          </Text>
        </div>
      </Group>

      <Stack gap={8} mt="sm">
        {/* L'intégrité : la signature correspond-elle à la version signée ? */}
        {signature.integrite === "intacte" ? (
          <Ligne sorte="bon">{p.integrite.intacte}</Ligne>
        ) : signature.integrite === "nonPriseEnCharge" ? (
          <Ligne sorte="attention">{p.integrite.nonPriseEnCharge(signature.nonPrisEnCharge)}</Ligne>
        ) : signature.integrite === "illisible" ? (
          <Ligne sorte="attention">{p.integrite.illisible}</Ligne>
        ) : (
          <Ligne sorte="mauvais">{p.integrite[signature.integrite]}</Ligne>
        )}

        {/* Ce qui a changé depuis. */}
        {signature.fin > 0 &&
          (signature.entiere ? (
            <Ligne sorte="bon">{p.entiere}</Ligne>
          ) : (
            <Ligne sorte={refusees.length > 0 ? (signature.sorte === "certification" ? "mauvais" : "attention") : "info"}>
              {p.modifie}
              <ul className="carte-signature__liste">
                {modifications.map((sorte) => (
                  <li key={sorte} data-modification={sorte}>
                    {p.sortes[sorte]}
                    {signature.sorte === "certification" && refusees.includes(sorte) ? ` (${p.nonPermise})` : ""}
                  </li>
                ))}
              </ul>
            </Ligne>
          ))}

        {/* L'identité du signataire. */}
        {signature.confiance === "reconnue" ? (
          <Ligne sorte="bon">{p.identite.reconnue(emetteur, MAC ? "macOS" : "Windows")}</Ligne>
        ) : signature.confiance === "nonReconnue" ? (
          <Ligne sorte="attention">{p.identite.nonReconnue(emetteur)}</Ligne>
        ) : signature.confiance === "revoquee" ? (
          <Ligne sorte="mauvais">{p.identite.revoquee}</Ligne>
        ) : (
          <Ligne sorte="attention">{p.identite[signature.confiance]}</Ligne>
        )}

        {/* La date. */}
        {signature.date && (
          <Ligne sorte={signature.dateHorodatee ? "bon" : "info"}>
            {signature.dateHorodatee ? p.dateHorodatee(signature.horodatage?.autorite ?? (signature.sorte === "horodatage" ? signature.signataire : null)) : p.dateDeclaree}
          </Ligne>
        )}
        {signature.horodatage && !signature.horodatage.valide && <Ligne sorte="attention">{p.horodatageInvalide}</Ligne>}
        {signature.empreinteAncienne && <Ligne sorte="attention">{p.ancienne}</Ligne>}

        {(signature.raison || signature.lieu || signature.contact) && (
          <dl className="carte-signature__proprietes">
            {signature.raison && (
              <>
                <dt>{p.raison}</dt>
                <dd>{signature.raison}</dd>
              </>
            )}
            {signature.lieu && (
              <>
                <dt>{p.lieu}</dt>
                <dd>{signature.lieu}</dd>
              </>
            )}
            {signature.contact && (
              <>
                <dt>{p.contact}</dt>
                <dd>{signature.contact}</dd>
              </>
            )}
          </dl>
        )}

        {signature.certificat && (
          <div>
            <UnstyledButton className="carte-signature__depli" onClick={() => setCertificat((avant) => !avant)} aria-expanded={certificat} data-certificat>
              <Group gap={4}>
                <Text size="sm" fw={500}>
                  {p.certificat}
                </Text>
                <IconChevronDown size={14} style={{ transform: certificat ? "rotate(180deg)" : undefined, transition: "transform 120ms" }} />
              </Group>
            </UnstyledButton>
            <Collapse expanded={certificat}>
              <dl className="carte-signature__proprietes">
                {(
                  [
                    [p.proprietes.sujet, signature.certificat.sujet],
                    [p.proprietes.organisation, signature.certificat.organisation],
                    [p.proprietes.courriel, signature.certificat.courriel],
                    [p.proprietes.emetteur, signature.certificat.emetteur],
                    [
                      p.proprietes.validite,
                      signature.certificat.debut && signature.certificat.fin ? p.validite(jourLisible(signature.certificat.debut)!, jourLisible(signature.certificat.fin)!) : null,
                    ],
                    [p.proprietes.cle, signature.certificat.cle],
                    [p.proprietes.numero, parPaires(signature.certificat.numero)],
                    [p.proprietes.empreinte, parPaires(signature.certificat.empreinte)],
                  ] as [string, string | null][]
                )
                  .filter(([, valeur]) => valeur)
                  .map(([cle, valeur]) => (
                    <div key={cle} className="carte-signature__propriete">
                      <dt>{cle}</dt>
                      <dd className={cle === p.proprietes.empreinte || cle === p.proprietes.numero ? "carte-signature__code" : undefined}>{valeur}</dd>
                    </div>
                  ))}
              </dl>
            </Collapse>
          </div>
        )}

        {(onVoir || onVersion) && (
          <Group gap="xs" mt={4}>
            {onVoir && (
              <Button size="xs" variant="default" leftSection={<IconEye size={15} />} onClick={onVoir} data-voir-signature>
                {p.voirSurLaPage}
              </Button>
            )}
            {onVersion && (
              <Button size="xs" variant="light" leftSection={<IconHistory size={15} />} onClick={onVersion} title={p.voirVersionAide} data-version-signee>
                {p.voirVersion}
              </Button>
            )}
          </Group>
        )}
        {signature.page === null && (
          <Text size="xs" c="dimmed">
            {p.invisible}
          </Text>
        )}
      </Stack>
    </div>
  );
}

interface PropsPanneau {
  ouvert: boolean;
  onFermer: () => void;
  chemin: string;
  signatures: SignaturePdf[];
  /** La signature à montrer d'abord (son champ). */
  enAvant: string | null;
  /** Aller à la page d'une signature visible. */
  onVoir: (signature: SignaturePdf) => void;
  /** Ouvrir un PDF (la version signée) dans un nouvel onglet. */
  onOuvrir: (chemin: string) => void;
}

/** Le détail de chaque signature, de la première faite à la dernière. */
export function PanneauSignatures({ ouvert, onFermer, chemin, signatures, enAvant, onVoir, onOuvrir }: PropsPanneau) {
  async function ouvrirVersion(signature: SignaturePdf) {
    try {
      onOuvrir(await versionSignee(chemin, signature.fin));
      onFermer();
    } catch (e) {
      signalerErreur(e);
    }
  }

  return (
    <Drawer opened={ouvert} onClose={onFermer} position="right" size={440} title={t.titre} data-panneau-signatures>
      <Stack gap="md">
        <Text size="sm" c="dimmed">
          {t.panneau.intro}
        </Text>
        {signatures.map((signature) => (
          <CarteSignature
            key={`${signature.champ}-${signature.fin}`}
            signature={signature}
            enAvant={signature.champ === enAvant}
            onVoir={
              signature.page !== null
                ? () => {
                    onVoir(signature);
                    onFermer();
                  }
                : undefined
            }
            onVersion={!signature.entiere && signature.fin > 0 ? () => void ouvrirVersion(signature) : undefined}
          />
        ))}
        <Group gap={6} wrap="nowrap" align="flex-start">
          <IconClockHour4 size={14} style={{ marginTop: 2, flexShrink: 0, opacity: 0.6 }} />
          <Text size="xs" c="dimmed">
            {t.panneau.horsLigne}
          </Text>
        </Group>
      </Stack>
    </Drawer>
  );
}

interface PropsCouche {
  page: number;
  echelle: number;
  signatures: SignaturePdf[];
  onChoisir: (signature: SignaturePdf) => void;
}

/** Les signatures visibles d'une page : un clic les vérifie. */
export function CoucheSignatures({ page, echelle, signatures, onChoisir }: PropsCouche) {
  return (
    <>
      {signaturesDeLaPage(signatures, page).map((signature) => {
        const [x, y, largeur, hauteur] = signature.zone!;
        const etat = etatDe(signature);
        return (
          <button
            key={`${signature.champ}-${signature.fin}`}
            type="button"
            className="signature-numerique"
            style={{ left: x * echelle, top: y * echelle, width: largeur * echelle, height: hauteur * echelle }}
            title={t.survol(nomDe(signature))}
            aria-label={t.survol(nomDe(signature))}
            onClick={() => onChoisir(signature)}
            data-etat={etat}
            data-signature-page={signature.champ}
          />
        );
      })}
    </>
  );
}
