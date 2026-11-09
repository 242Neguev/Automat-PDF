import { useCallback, useEffect, useRef, useState } from "react";
import { Alert, Box, Button, Group, Loader, Progress, Stack, Text } from "@mantine/core";
import { IconAlertTriangle, IconDownload, IconExternalLink, IconRefresh } from "@tabler/icons-react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { enErreur, ouvrirPageDesVersions } from "./coeur";
import { avancer, pourcentage, signatureRefusee, TELECHARGEMENT_VIDE } from "./donnees/telechargement";
import { signalerErreur } from "./pages/commun/signalerErreur";
import { textes } from "./textes";

const t = textes.miseAJour;

/** Où en est la mise à jour de l'application. */
export type EtatMiseAJour =
  | { etape: "inconnue" }
  | { etape: "recherche" }
  | { etape: "aJour" }
  | { etape: "disponible"; version: string }
  | { etape: "telechargement"; version: string; avancement: number | null }
  | { etape: "installation"; version: string }
  /** Installée sans que l'application se ferme (hors Windows) : il faut la rouvrir. */
  | { etape: "installee"; version: string }
  /**
   * `version` : celle qui n'a pas pu être installée ; null si c'est la recherche qui a échoué.
   * `signatureRefusee` : le fichier reçu n'est pas celui que le projet a signé, réessayer n'y
   * changerait rien.
   */
  | { etape: "echec"; version: string | null; message: string; detail?: string; signatureRefusee?: boolean };

export interface MiseAJour {
  etat: EtatMiseAJour;
  /** Cherche une nouvelle version ; `manuelle` : demandée dans « À propos », qui montre alors le résultat. */
  chercher: (manuelle: boolean) => Promise<void>;
  /** Télécharge la nouvelle version, vérifie sa signature et l'installe ; sous Windows, Automat-PDF se ferme puis se rouvre. */
  installer: () => Promise<void>;
}

/** La fenêtre s'ouvre d'abord : la première recherche attend quelques secondes. */
const PREMIERE_RECHERCHE = 5_000;
/** Une application laissée ouverte cherche de nouveau chaque jour. */
const RECHERCHES_SUIVANTES = 24 * 60 * 60 * 1000;

/** Recherche (au démarrage puis chaque jour, si `automatique`) et installation des mises à jour. */
export function useMiseAJour(automatique: boolean): MiseAJour {
  const [etat, setEtat] = useState<EtatMiseAJour>({ etape: "inconnue" });
  const trouvee = useRef<Update | null>(null);
  // Une seule recherche ou installation à la fois.
  const occupe = useRef(false);

  const chercher = useCallback(async (manuelle: boolean) => {
    if (occupe.current) return;
    occupe.current = true;
    if (manuelle) setEtat({ etape: "recherche" });
    try {
      const mise = await check({ timeout: 30_000 });
      if (mise) {
        await trouvee.current?.close().catch(() => {});
        trouvee.current = mise;
        setEtat({ etape: "disponible", version: mise.version });
      } else if (manuelle) {
        setEtat({ etape: "aJour" });
      }
    } catch (erreur) {
      // Sans internet, la recherche automatique échoue sans rien dire : elle reprendra plus tard.
      console.warn("Recherche de mise à jour :", erreur);
      if (manuelle) setEtat({ etape: "echec", version: null, message: t.rechercheEchouee, detail: enErreur(erreur).message });
    } finally {
      occupe.current = false;
    }
  }, []);

  const installer = useCallback(async () => {
    const mise = trouvee.current;
    if (!mise || occupe.current) return;
    occupe.current = true;
    const version = mise.version;
    let suivi = TELECHARGEMENT_VIDE;
    let affiche: number | null = null;
    setEtat({ etape: "telechargement", version, avancement: null });
    try {
      await mise.download((evenement) => {
        suivi = avancer(suivi, evenement);
        const avancement = pourcentage(suivi);
        if (avancement !== affiche) {
          affiche = avancement;
          setEtat({ etape: "telechargement", version, avancement });
        }
      });
      setEtat({ etape: "installation", version });
      // Sous Windows, l'installateur prend le relais : Automat-PDF se ferme ici, puis se rouvre.
      await mise.install();
      setEtat({ etape: "installee", version });
    } catch (erreur) {
      console.warn("Installation de la mise à jour :", erreur);
      const detail = enErreur(erreur).message;
      const refusee = signatureRefusee(detail);
      setEtat({ etape: "echec", version, message: refusee ? t.echecSignature : t.echecDetail, detail, signatureRefusee: refusee });
    } finally {
      occupe.current = false;
    }
  }, []);

  useEffect(() => {
    if (!automatique) return;
    const premiere = window.setTimeout(() => void chercher(false), PREMIERE_RECHERCHE);
    const suivantes = window.setInterval(() => void chercher(false), RECHERCHES_SUIVANTES);
    return () => {
      window.clearTimeout(premiere);
      window.clearInterval(suivantes);
    };
  }, [automatique, chercher]);

  return { etat, chercher, installer };
}

const ouvrirPage = () => void ouvrirPageDesVersions().catch(signalerErreur);

/**
 * Bandeau en haut de chaque écran : une nouvelle version est disponible, elle se télécharge,
 * s'installe, ou n'a pas pu l'être. « Plus tard » le cache jusqu'au prochain lancement.
 */
export function BandeauMiseAJour({ miseAJour, versionActuelle }: { miseAJour: MiseAJour; versionActuelle: string }) {
  // Ce que « Plus tard » a écarté : une version proposée, ou l'échec de son installation.
  const [ecarte, setEcarte] = useState<string | null>(null);
  const { etat } = miseAJour;
  const cle = "version" in etat ? `${etat.etape} ${etat.version}` : etat.etape;
  const alerte = cle === ecarte ? null : alerteMiseAJour(miseAJour, versionActuelle, () => setEcarte(cle));
  return (
    alerte && (
      <Box maw={1240} mb="lg">
        {alerte}
      </Box>
    )
  );
}

function alerteMiseAJour({ etat, installer }: MiseAJour, versionActuelle: string, repousser: () => void) {
  const lancer = () => void installer();
  const plusTard = (
    <Button variant="subtle" color="gray" onClick={repousser}>
      {t.plusTard}
    </Button>
  );

  switch (etat.etape) {
    case "disponible":
      return (
        <Alert color="eau" icon={<IconDownload size={20} />} title={t.disponible(etat.version)} data-mise-a-jour="disponible">
          <Stack gap="xs">
            <Text size="sm">{t.explication(versionActuelle)}</Text>
            <Group gap="xs">
              <Button leftSection={<IconDownload size={18} />} onClick={lancer}>
                {t.mettreAJour}
              </Button>
              {plusTard}
            </Group>
          </Stack>
        </Alert>
      );
    case "telechargement":
      return (
        <Alert color="eau" icon={<IconDownload size={20} />} title={t.telechargement(etat.version)} data-mise-a-jour="telechargement">
          <Group gap="sm" wrap="nowrap">
            <Progress value={etat.avancement ?? 100} animated={etat.avancement === null} striped={etat.avancement === null} style={{ flex: 1 }} />
            {etat.avancement !== null && (
              <Text size="sm" w={48} ta="right">
                {t.pourcentage(etat.avancement)}
              </Text>
            )}
          </Group>
        </Alert>
      );
    case "installation":
      return (
        <Alert color="eau" icon={<Loader size={18} />} title={t.installation(etat.version)} data-mise-a-jour="installation">
          <Text size="sm">{t.installationDetail}</Text>
        </Alert>
      );
    case "installee":
      return (
        <Alert color="teal" title={t.installee(etat.version)} data-mise-a-jour="installee" />
      );
    case "echec":
      // Une recherche qui échoue ne se signale que dans « À propos ».
      if (etat.version === null) return null;
      return (
        <Alert color="orange" icon={<IconAlertTriangle size={20} />} title={t.echec} data-mise-a-jour="echec">
          <Stack gap="xs">
            <Text size="sm">{etat.message}</Text>
            {etat.detail && (
              <Text size="xs" c="dimmed" style={{ overflowWrap: "anywhere" }}>
                {t.detailTechnique(etat.detail)}
              </Text>
            )}
            {/* Un seul chemin évident : après une signature refusée, la page de téléchargement. */}
            <Group gap="xs">
              {!etat.signatureRefusee && (
                <Button leftSection={<IconRefresh size={18} />} onClick={lancer}>
                  {t.reessayer}
                </Button>
              )}
              <Button variant={etat.signatureRefusee ? "filled" : "default"} leftSection={<IconExternalLink size={18} />} onClick={ouvrirPage}>
                {t.pageDesVersions}
              </Button>
              {plusTard}
            </Group>
          </Stack>
        </Alert>
      );
    default:
      return null;
  }
}

/** Où en est la mise à jour, en une ligne, pour « À propos ». */
export function etatEnClair(etat: EtatMiseAJour): string | null {
  switch (etat.etape) {
    case "recherche":
      return t.recherche;
    case "aJour":
      return t.aJour;
    case "disponible":
      return t.disponibleCourt(etat.version);
    case "telechargement":
      return etat.avancement === null ? t.telechargement(etat.version) : `${t.telechargement(etat.version)} ${t.pourcentage(etat.avancement)}`;
    case "installation":
      return t.installation(etat.version);
    case "installee":
      return t.installee(etat.version);
    case "echec":
      return etat.version === null ? etat.message : t.echec;
    default:
      return null;
  }
}
