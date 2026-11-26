import { reconnaitreTexte } from "../../coeur";
import { textes } from "../../textes";
import ConversionFichiers from "./ConversionFichiers";

const t = textes.conversion.ocr;

/** Reconnaître le texte : des PDF numérisés deviennent cherchables, par le moteur du système (OCR). */
export default function ReconnaitreTexte() {
  return (
    <ConversionFichiers
      outil="ocr"
      filtre={{ nom: textes.pdf.filtre, extensions: ["pdf"] }}
      deposer={t.deposer}
      deposerDetail={t.deposerDetail}
      aide={t.aide}
      convertir={reconnaitreTexte}
      libelles={{ bouton: t.bouton, choisirDossier: t.choisirDossier, cree: t.cree, aucun: t.aucun, echecs: t.echecs, progressionPages: t.progression }}
    />
  );
}
