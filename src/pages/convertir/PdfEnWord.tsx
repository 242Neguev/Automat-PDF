import { useEffect, useState } from "react";
import { Alert, Stack } from "@mantine/core";
import { IconInfoCircle } from "@tabler/icons-react";
import { convertisseursOffice, pdfEnWord, type Convertisseurs } from "../../coeur";
import { textes } from "../../textes";
import ConversionFichiers from "./ConversionFichiers";

const t = textes.conversion.word;

/** PDF en Word : chaque PDF devient un document Word modifiable, par Word, ou à défaut par Automat-PDF. */
export default function PdfEnWord() {
  const [convertisseurs, setConvertisseurs] = useState<Convertisseurs | null>(null);
  // Si la vérification échoue, rien n'est affiché : la conversion dira ce qui manque.
  useEffect(() => void convertisseursOffice().then(setConvertisseurs, () => setConvertisseurs(null)), []);

  return (
    <Stack gap="md" maw={720}>
      {convertisseurs && !convertisseurs.word && (
        <Alert color="blue" icon={<IconInfoCircle size={18} />} data-sans-word>
          {t.sansWord}
        </Alert>
      )}
      <ConversionFichiers outil="pdfEnWord" filtre={{ nom: textes.pdf.filtre, extensions: ["pdf"] }} deposer={t.deposer} deposerDetail={t.deposerDetail} aide={t.aide} convertir={pdfEnWord} />
    </Stack>
  );
}
