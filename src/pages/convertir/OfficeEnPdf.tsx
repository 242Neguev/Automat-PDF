import { useCallback, useEffect, useState } from "react";
import { Stack } from "@mantine/core";
import { convertisseursOffice, officeEnPdf, type Convertisseurs } from "../../coeur";
import { textes } from "../../textes";
import ConversionFichiers from "./ConversionFichiers";
import LibreOffice from "./LibreOffice";

const t = textes.conversion.office;
const EXTENSIONS = ["doc", "docx", "docm", "dot", "dotx", "rtf", "odt", "xls", "xlsx", "xlsm", "xlsb", "ods", "ppt", "pptx", "pptm", "pps", "ppsx", "odp"];

/** Office en PDF : documents Word, classeurs Excel et présentations PowerPoint convertis par Office, ou LibreOffice. */
export default function OfficeEnPdf() {
  const [convertisseurs, setConvertisseurs] = useState<Convertisseurs | null>(null);
  // Si la vérification échoue, rien n'est affiché : la conversion dira ce qui manque.
  const verifier = useCallback(() => void convertisseursOffice().then(setConvertisseurs, () => setConvertisseurs(null)), []);
  useEffect(verifier, [verifier]);

  return (
    <Stack gap="md" maw={720}>
      <LibreOffice convertisseurs={convertisseurs} onVerifier={verifier} />
      <ConversionFichiers outil="officeEnPdf" filtre={{ nom: t.filtre, extensions: EXTENSIONS }} vide={t.vide} aide={t.aide} convertir={officeEnPdf} />
    </Stack>
  );
}
