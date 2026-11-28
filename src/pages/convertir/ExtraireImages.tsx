import { useState } from "react";
import { Checkbox } from "@mantine/core";
import { extraireImages } from "../../coeur";
import { textes } from "../../textes";
import ConversionFichiers from "./ConversionFichiers";

const t = textes.conversion.images;

/** Extraire les images : les photos et images de PDF, chacune dans son fichier, dans sa qualité d'origine. */
export default function ExtraireImages() {
  const [laisserPetites, setLaisserPetites] = useState(true);
  return (
    <ConversionFichiers
      outil="extraireImages"
      filtre={{ nom: textes.pdf.filtre, extensions: ["pdf"] }}
      deposer={t.deposer}
      deposerDetail={t.deposerDetail}
      aide={t.aide}
      convertir={(fichiers, dossier) => extraireImages(fichiers, dossier, laisserPetites)}
      options={
        <Checkbox
          label={t.petites}
          description={t.petitesAide}
          checked={laisserPetites}
          onChange={(evenement) => setLaisserPetites(evenement.currentTarget.checked)}
          data-laisser-petites
        />
      }
      libelles={{ bouton: t.bouton, choisirDossier: t.choisirDossier, cree: t.cree, aucun: t.aucun, echecs: t.echecs }}
    />
  );
}
