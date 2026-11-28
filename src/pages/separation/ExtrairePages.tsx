import { useState } from "react";
import { SegmentedControl, Stack, Text } from "@mantine/core";
import { IconFileExport } from "@tabler/icons-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { enregistrerPages, extrairePages, pagesPdf } from "../../coeur";
import { aCote, dossierDe, nomDe } from "../../donnees/chemins";
import { ecrirePlages, nomExtrait } from "../../donnees/plages";
import { textes } from "../../textes";
import Action from "../commun/Action";
import BarreAction from "../commun/BarreAction";
import ChoixPages from "../commun/ChoixPages";
import Etape from "../commun/Etape";
import { dela, pageConfiee, useFichiersRecus } from "../commun/reception";
import Resultat, { type Cree } from "../commun/Resultat";
import { signalerErreur } from "../commun/signalerErreur";
import { oublier } from "../commun/vignettes";
import { ZoneFichier } from "../commun/ZoneFichier";

const t = textes.extraction;

/**
 * Extraire des pages : on les voit en grand, on clique celles à garder, puis on les enregistre en
 * un nouveau PDF (ou un PDF par page). Ouvert depuis le lecteur, la page lue est déjà choisie.
 */
export default function ExtrairePages() {
  const [source, setSource] = useState<{ chemin: string; nom: string; pages: number } | null>(null);
  const [choisies, setChoisies] = useState<number[]>([]);
  const [vue, setVue] = useState(0);
  const [mode, setMode] = useState<"un" | "chacune">("un");
  const [enCours, setEnCours] = useState(false);
  const [cree, setCree] = useState<Cree | null>(null);
  const base = source?.nom.replace(/\.pdf$/i, "") ?? "";

  async function choisir() {
    const chemin = await open({ multiple: false, directory: false, filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (typeof chemin === "string") await ouvrir(chemin);
  }

  /** Ouvre un PDF ; `page` (à partir de 0) : celle que montrait le lecteur, choisie d'emblée. */
  async function ouvrir(chemin: string, page?: number) {
    try {
      const pages = (await pagesPdf(chemin)).length;
      const depart = page !== undefined && page < pages ? page : null;
      setSource({ chemin, nom: nomDe(chemin), pages });
      setChoisies(depart === null ? [] : [depart]);
      setVue(depart ?? 0);
      setCree(null);
    } catch (e) {
      signalerErreur(e);
    }
  }
  useFichiersRecus("extraire", (fichiers) => {
    const [pdf] = dela(fichiers, "pdf");
    // La page confiée par le lecteur se lit tout de suite : elle ne vaut que pour cette remise.
    if (pdf) void ouvrir(pdf, pageConfiee(pdf));
  });

  async function extraire() {
    if (!source || choisies.length === 0) return;
    if (mode === "chacune" && choisies.length > 1) {
      const dossier = await open({ directory: true, multiple: false, title: t.choisirDossier, defaultPath: dossierDe(source.chemin) || undefined });
      if (typeof dossier !== "string") return;
      setEnCours(true);
      setCree(null);
      try {
        const cible = await extrairePages(source.chemin, choisies, dossier, t.nomDossier(base));
        setCree({ titre: t.crees(choisies.length), chemin: cible, dossier: true });
      } catch (e) {
        signalerErreur(e);
      } finally {
        setEnCours(false);
      }
      return;
    }
    const sortie = await save({ defaultPath: aCote(source.chemin, nomExtrait(base, choisies)), filters: [{ name: textes.pdf.filtre, extensions: ["pdf"] }] });
    if (!sortie) return;
    setEnCours(true);
    setCree(null);
    try {
      await enregistrerPages(
        choisies.map((page) => ({ chemin: source.chemin, page, rotation: 0 })),
        sortie,
      );
      oublier(sortie);
      setCree({ titre: t.enregistre, chemin: sortie, details: [textes.pdf.pages(choisies.length)] });
    } catch (e) {
      signalerErreur(e);
    } finally {
      setEnCours(false);
    }
  }

  return (
    <Stack gap="md">
      <Etape numero={1} titre={t.etapes.pdf}>
        <ZoneFichier
          fichier={source ? { nom: source.nom, details: textes.pdf.pages(source.pages) } : null}
          invite={textes.depot.pdf}
          bouton={t.choisir}
          changer={t.autre}
          onChoisir={() => void choisir()}
        />
      </Etape>

      {source && (
        <>
          <Etape numero={2} titre={t.etapes.pages}>
            <ChoixPages
              chemin={source.chemin}
              total={source.pages}
              choisies={choisies}
              onChoisies={(pages) => {
                setChoisies(pages);
                setCree(null);
              }}
              vue={vue}
              onVue={setVue}
            />
          </Etape>

          <Etape numero={3} titre={t.etapes.extraire}>
            {choisies.length > 0 && (
              <Text size="sm" fw={500} data-resume-extraction>
                {t.resume(choisies.length, ecrirePlages(choisies))}
              </Text>
            )}
            {choisies.length > 1 && (
              <SegmentedControl
                value={mode}
                onChange={(valeur) => setMode(valeur as typeof mode)}
                data={(["un", "chacune"] as const).map((value) => ({ value, label: t.modes[value] }))}
                style={{ alignSelf: "flex-start" }}
              />
            )}
            <BarreAction>
              <Action icone={IconFileExport} enCours={enCours} raison={choisies.length === 0 ? t.raison : null} onClick={() => void extraire()}>
                {t.extraire}
              </Action>
            </BarreAction>
            <Resultat cree={cree} onFermer={() => setCree(null)} />
          </Etape>
        </>
      )}
    </Stack>
  );
}
