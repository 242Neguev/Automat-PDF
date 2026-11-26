import { useState } from "react";
import { Checkbox } from "@mantine/core";
import { pdfEnTexte } from "../../coeur";
import { textes } from "../../textes";
import ConversionFichiers from "./ConversionFichiers";

const t = textes.conversion.texte;

/** PDF en texte : le texte de PDF dans des fichiers texte ; celui des pages numérisées est reconnu. */
export default function PdfEnTexte() {
  const [reconnaitre, setReconnaitre] = useState(true);
  return (
    <ConversionFichiers
      outil="pdfEnTexte"
      filtre={{ nom: textes.pdf.filtre, extensions: ["pdf"] }}
      deposer={t.deposer}
      deposerDetail={t.deposerDetail}
      aide={t.aide}
      convertir={(fichiers, dossier) => pdfEnTexte(fichiers, dossier, reconnaitre)}
      options={
        <Checkbox
          label={t.reconnaitre}
          description={t.reconnaitreAide}
          checked={reconnaitre}
          onChange={(evenement) => setReconnaitre(evenement.currentTarget.checked)}
          data-reconnaitre-pages
        />
      }
      libelles={{ bouton: t.bouton, choisirDossier: t.choisirDossier, cree: t.cree, aucun: t.aucun, echecs: t.echecs, progressionPages: t.progression }}
    />
  );
}
