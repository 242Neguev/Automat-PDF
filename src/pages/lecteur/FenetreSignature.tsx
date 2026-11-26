// Signer (0.7.0) : la fenêtre où l'on dessine sa signature ou ses initiales, à la souris, au pavé
// tactile ou au stylet, et où l'on retrouve celles gardées sur cet ordinateur pour la prochaine fois.
import { useEffect, useRef, useState, type PointerEvent as EvenementPointeur } from "react";
import { Anchor, Button, Checkbox, ColorSwatch, Group, Modal, SegmentedControl, Stack, Text, Tooltip } from "@mantine/core";
import { IconCheck, IconEraser, IconSignature } from "@tabler/icons-react";
import { garderSignatures, signaturesGardees } from "../../coeur";
import type { Point, Teinte } from "../../donnees/annotations";
import { chemin, ecrireSignatures, ENCRES, lireSignatures, normaliser, type SignatureChoisie, type SignaturesGardees } from "../../donnees/signature";
import { textes } from "../../textes";
import { signalerErreur } from "../commun/signalerErreur";
import { couleur } from "./Annotations";

const t = textes.lecteur.signature;

/** Le pavé, en unités de son dessin (le cadre SVG le met à la largeur de la fenêtre). */
const PAVE = { signature: { largeur: 600, hauteur: 200 }, initiales: { largeur: 300, hauteur: 200 } };
/** Épaisseur du trait dans le pavé, en ces unités. */
const EPAISSEUR = 3.5;
/** Écart sous lequel un point de plus n'est pas gardé (le trait reste léger). */
const ECART_MIN = 1.5;

type Sorte = "signature" | "initiales";

interface Props {
  ouvert: boolean;
  onFermer: () => void;
  /** La signature (ou les initiales) choisie, à poser. */
  onChoisir: (choix: SignatureChoisie) => void;
  /** Poser plutôt l'image d'une signature (scan, photo). */
  onImage: () => void;
}

export default function FenetreSignature({ ouvert, onFermer, onChoisir, onImage }: Props) {
  const [sorte, setSorte] = useState<Sorte>("signature");
  const [gardees, setGardees] = useState<SignaturesGardees>({});
  const [refaire, setRefaire] = useState(false);
  const [traits, setTraits] = useState<Point[][]>([]);
  const [teinte, setTeinte] = useState<Teinte>("marine");
  const [garder, setGarder] = useState(true);
  const enCours = useRef(false);
  const pave = useRef<SVGSVGElement>(null);

  // À l'ouverture : celles gardées sur cet ordinateur, et un pavé vide.
  useEffect(() => {
    if (!ouvert) return;
    setTraits([]);
    setRefaire(false);
    let actuelle = true;
    signaturesGardees().then(
      (contenu) => actuelle && setGardees(lireSignatures(contenu)),
      () => actuelle && setGardees({}),
    );
    return () => {
      actuelle = false;
    };
  }, [ouvert]);

  function changerSorte(suivante: Sorte) {
    setSorte(suivante);
    setTraits([]);
    setRefaire(false);
  }

  const gardee = gardees[sorte];
  const dessiner = !gardee || refaire;
  const taille = PAVE[sorte];

  /** Le point du pavé sous le pointeur, en unités du dessin. */
  function ici(evenement: { clientX: number; clientY: number }): Point {
    const cadre = pave.current!.getBoundingClientRect();
    return { x: ((evenement.clientX - cadre.left) * taille.largeur) / cadre.width, y: ((evenement.clientY - cadre.top) * taille.hauteur) / cadre.height };
  }

  function bas(evenement: EvenementPointeur<SVGSVGElement>) {
    if (evenement.pointerType === "mouse" && evenement.button !== 0) return;
    evenement.preventDefault();
    evenement.currentTarget.setPointerCapture(evenement.pointerId);
    enCours.current = true;
    const point = ici(evenement);
    setTraits((avant) => [...avant, [point]]);
  }

  function mouvement(evenement: EvenementPointeur<SVGSVGElement>) {
    if (!enCours.current) return;
    const point = ici(evenement);
    setTraits((avant) => {
      const dernier = avant[avant.length - 1];
      const precedent = dernier?.[dernier.length - 1];
      if (!dernier || (precedent && Math.hypot(point.x - precedent.x, point.y - precedent.y) < ECART_MIN)) return avant;
      return [...avant.slice(0, -1), [...dernier, point]];
    });
  }

  const haut = () => {
    enCours.current = false;
  };

  async function poser() {
    const initiales = sorte === "initiales";
    if (!dessiner && gardee) {
      onChoisir({ trace: gardee.trace, teinte: gardee.teinte, initiales });
      return;
    }
    const trace = normaliser(traits, EPAISSEUR);
    if (!trace) return;
    if (garder) {
      const nouvelles = { ...gardees, [sorte]: { trace, teinte } };
      try {
        await garderSignatures(ecrireSignatures(nouvelles));
        setGardees(nouvelles);
      } catch (e) {
        signalerErreur(e);
      }
    }
    onChoisir({ trace, teinte, initiales });
  }

  async function oublier() {
    const restes: SignaturesGardees = { ...gardees };
    delete restes[sorte];
    try {
      await garderSignatures(restes.signature || restes.initiales ? ecrireSignatures(restes) : "");
      setGardees(restes);
      setRefaire(false);
      setTraits([]);
    } catch (e) {
      signalerErreur(e);
    }
  }

  return (
    <Modal opened={ouvert} onClose={onFermer} title={t.titre} size="lg" centered data-fenetre-signature>
      <Stack gap="md">
        <SegmentedControl
          value={sorte}
          onChange={(valeur) => changerSorte(valeur === "initiales" ? "initiales" : "signature")}
          data={[
            { value: "signature", label: t.signature },
            { value: "initiales", label: t.initiales },
          ]}
          data-sorte-signature
        />
        {dessiner || !gardee ? (
          <>
            <div className="pave-signature" data-sorte={sorte}>
              <svg
                ref={pave}
                viewBox={`0 0 ${taille.largeur} ${taille.hauteur}`}
                onPointerDown={bas}
                onPointerMove={mouvement}
                onPointerUp={haut}
                onPointerCancel={haut}
                role="img"
                aria-label={sorte === "signature" ? t.dessinez : t.dessinezInitiales}
                data-pave-signature
              >
                <line className="pave-signature__ligne" x1={24} x2={taille.largeur - 24} y1={taille.hauteur * 0.75} y2={taille.hauteur * 0.75} />
                {traits.length === 0 && (
                  <text x={taille.largeur / 2} y={taille.hauteur * 0.45} textAnchor="middle" className="pave-signature__aide">
                    {sorte === "signature" ? t.dessinez : t.dessinezInitiales}
                  </text>
                )}
                {traits.map((trait, rang) => (
                  <path key={rang} d={chemin(trait)} stroke={couleur(teinte)} strokeWidth={EPAISSEUR} strokeLinecap="round" strokeLinejoin="round" fill="none" />
                ))}
              </svg>
            </div>
            <Group justify="space-between" gap="sm">
              <Group gap="xs">
                <Text size="sm">{t.encre}</Text>
                {ENCRES.map((cle) => (
                  <Tooltip key={cle} label={t.encres[cle]}>
                    <ColorSwatch
                      component="button"
                      type="button"
                      size={22}
                      color={couleur(cle)}
                      onClick={() => setTeinte(cle)}
                      aria-label={t.encres[cle]}
                      aria-pressed={teinte === cle}
                      style={{ cursor: "pointer", color: "white" }}
                      data-encre={cle}
                    >
                      {teinte === cle && <IconCheck size={13} stroke={3} />}
                    </ColorSwatch>
                  </Tooltip>
                ))}
              </Group>
              <Button variant="default" size="xs" leftSection={<IconEraser size={15} />} onClick={() => setTraits([])} disabled={traits.length === 0}>
                {t.effacer}
              </Button>
            </Group>
            <div>
              <Checkbox label={sorte === "signature" ? t.garder : t.garderInitiales} checked={garder} onChange={(evenement) => setGarder(evenement.currentTarget.checked)} data-garder-signature />
              <Text size="xs" c="dimmed" ml={30}>
                {t.gardeeInfo}
              </Text>
            </div>
          </>
        ) : (
          <Stack gap="xs" className="apercu-signature" data-signature-gardee>
            <Text size="sm" c="dimmed">
              {sorte === "signature" ? t.gardee : t.gardees}
            </Text>
            <svg viewBox={`0 0 ${gardee.trace.proportion} 1`} preserveAspectRatio="xMidYMid meet" aria-hidden>
              {gardee.trace.traits.map((trait, rang) => (
                <path key={rang} d={chemin(trait, gardee.trace.proportion, 1)} stroke={couleur(gardee.teinte)} strokeWidth={gardee.trace.epaisseur} strokeLinecap="round" strokeLinejoin="round" fill="none" />
              ))}
            </svg>
            <Group gap="xs">
              <Button variant="default" size="xs" onClick={() => setRefaire(true)} data-refaire-signature>
                {t.refaire}
              </Button>
              <Button variant="subtle" color="red" size="xs" onClick={() => void oublier()} data-oublier-signature>
                {t.oublier}
              </Button>
            </Group>
          </Stack>
        )}
        <Group justify="space-between" gap="sm">
          <Anchor component="button" type="button" size="sm" onClick={onImage}>
            {t.image}
          </Anchor>
          <Button leftSection={<IconSignature size={17} />} onClick={() => void poser()} disabled={dessiner && traits.length === 0} data-poser-signature>
            {t.poser}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
