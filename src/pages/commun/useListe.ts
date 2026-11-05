import { useEffect, useMemo, useState } from "react";
import { notifications } from "@mantine/notifications";
import { open } from "@tauri-apps/plugin-dialog";
import type { WorkBook } from "xlsx";
import urlListeExemple from "../../assets/exemple-agents.xlsx?url";
import { controlerListe } from "../../donnees/controle";
import { fusionner, type Correspondances } from "../../donnees/correspondances";
import { lireFeuille, ouvrirClasseur } from "../../donnees/liste";
import { lireListe } from "../../coeur";
import { textes } from "../../textes";
import { signalerErreur } from "./signalerErreur";

const EXTENSIONS_LISTE = ["xlsx", "xlsm", "xls", "ods", "csv", "tsv", "txt"];

/**
 * Liste des personnes (fichier Excel, CSV ou ODS) et association de ses colonnes aux balises
 * (noms de fichiers…) : automatique, en gardant les choix faits à la main.
 */
export function useListe(balises: string[]) {
  const [classeur, setClasseur] = useState<{ nomFichier: string; chemin: string | null; contenu: WorkBook } | null>(null);
  const [feuille, setFeuille] = useState<string | null>(null);
  const [correspondances, setCorrespondances] = useState<Correspondances>({});
  const cleBalises = balises.join("\u0000");
  const liste = useMemo(
    () => (classeur && feuille ? lireFeuille(classeur.contenu, classeur.nomFichier, feuille) : null),
    [classeur, feuille],
  );
  useEffect(() => {
    if (liste) setCorrespondances((precedentes) => fusionner(balises, liste.colonnes, precedentes));
  }, [liste, cleBalises]); // `cleBalises` résume `balises`
  const anomalies = useMemo(() => (liste ? controlerListe(liste, balises, correspondances) : []), [liste, cleBalises, correspondances]);

  function ouvrir(contenu: ArrayBuffer, nomFichier: string, chemin: string | null, feuilleVoulue: string | null = null) {
    try {
      const nouveau = ouvrirClasseur(contenu, nomFichier);
      setClasseur({ nomFichier, chemin, contenu: nouveau });
      setFeuille(feuilleVoulue && nouveau.SheetNames.includes(feuilleVoulue) ? feuilleVoulue : (nouveau.SheetNames[0] ?? null));
      return true;
    } catch (e) {
      signalerErreur(e);
      return false;
    }
  }

  /** Ouvre la liste enregistrée à `chemin` (feuille voulue si elle existe encore) ; `false` si elle n'a pas pu l'être. */
  async function importerDepuis(chemin: string, feuilleVoulue: string | null = null) {
    try {
      return ouvrir(await lireListe(chemin), chemin.split(/[\\/]/).pop() ?? chemin, chemin, feuilleVoulue);
    } catch (e) {
      signalerErreur(e);
      return false;
    }
  }

  /** Liste reçue de l'Explorateur : importée, et annoncée (son étape peut être encore masquée). */
  async function recevoir(chemin: string) {
    if (await importerDepuis(chemin)) notifications.show({ color: "teal", title: textes.liste.importee(chemin.split(/[\\/]/).pop() ?? chemin), message: "" });
  }

  async function importer() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.liste.filtre, extensions: EXTENSIONS_LISTE }] });
    if (typeof chemin === "string") await importerDepuis(chemin);
  }

  async function utiliserExemple() {
    try {
      const reponse = await fetch(urlListeExemple);
      ouvrir(await reponse.arrayBuffer(), "exemple-agents.xlsx", null);
    } catch (e) {
      signalerErreur(e);
    }
  }

  return {
    liste,
    /** Chemin du fichier de la liste (null pour la liste d'exemple). */
    chemin: classeur?.chemin ?? null,
    feuilles: classeur?.contenu.SheetNames ?? [],
    setFeuille,
    correspondances,
    relier: (balise: string, colonne: string | null) => setCorrespondances((actuelles) => ({ ...actuelles, [balise]: colonne })),
    /** Correspondances d'un projet rouvert, accordées à la liste déjà chargée. */
    restaurer: (enregistrees: Correspondances, balisesDuProjet: string[]) =>
      setCorrespondances(liste ? fusionner(balisesDuProjet, liste.colonnes, enregistrees) : enregistrees),
    anomalies,
    importer,
    importerDepuis,
    recevoir,
    utiliserExemple,
  };
}
