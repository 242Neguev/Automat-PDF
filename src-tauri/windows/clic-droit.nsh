; Clic droit dans l'Explorateur : sous-menu « Automat-PDF » sur les PDF, les images et les documents
; Office, pour le compte de l'utilisateur (SHCTX), retiré à la désinstallation. Chaque action lance
; Automat-PDF sur l'outil voulu : « Automat-PDF.exe --outil compresser "fichier.pdf" » ; plusieurs
; fichiers choisis lancent plusieurs fois l'application, qui les réunit dans sa seule fenêtre.

!define AUTOMATPDF_CLASSES "Software\Classes\SystemFileAssociations"

!macro AutomatPdfSousMenu EXT
  WriteRegStr SHCTX "${AUTOMATPDF_CLASSES}\${EXT}\shell\AutomatPDF" "MUIVerb" "Automat-PDF"
  WriteRegStr SHCTX "${AUTOMATPDF_CLASSES}\${EXT}\shell\AutomatPDF" "Icon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr SHCTX "${AUTOMATPDF_CLASSES}\${EXT}\shell\AutomatPDF" "SubCommands" ""
!macroend

; RANG range les actions dans le sous-menu, qui suit l'ordre des noms de clés.
!macro AutomatPdfAction EXT RANG OUTIL LIBELLE
  WriteRegStr SHCTX "${AUTOMATPDF_CLASSES}\${EXT}\shell\AutomatPDF\shell\${RANG}${OUTIL}" "MUIVerb" "${LIBELLE}"
  WriteRegStr SHCTX "${AUTOMATPDF_CLASSES}\${EXT}\shell\AutomatPDF\shell\${RANG}${OUTIL}\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --outil ${OUTIL} $\"%1$\""
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro AutomatPdfSousMenu ".pdf"
  !insertmacro AutomatPdfAction ".pdf" "1" "organiser" "Organiser les pages"
  !insertmacro AutomatPdfAction ".pdf" "2" "fusionner" "Fusionner"
  !insertmacro AutomatPdfAction ".pdf" "3" "compresser" "Compresser"
  !insertmacro AutomatPdfAction ".pdf" "4" "motDePasse" "Protéger par un mot de passe"
  !insertmacro AutomatPdfAction ".pdf" "5" "parPersonne" "Séparer par personne"
  !insertmacro AutomatPdfAction ".pdf" "6" "pdfEnWord" "Convertir en Word"
  !insertmacro AutomatPdfAction ".pdf" "7" "enImages" "Convertir en images"
  !insertmacro AutomatPdfSousMenu ".jpg"
  !insertmacro AutomatPdfAction ".jpg" "1" "images" "Mettre en PDF"
  !insertmacro AutomatPdfSousMenu ".jpeg"
  !insertmacro AutomatPdfAction ".jpeg" "1" "images" "Mettre en PDF"
  !insertmacro AutomatPdfSousMenu ".png"
  !insertmacro AutomatPdfAction ".png" "1" "images" "Mettre en PDF"
  !insertmacro AutomatPdfSousMenu ".webp"
  !insertmacro AutomatPdfAction ".webp" "1" "images" "Mettre en PDF"
  !insertmacro AutomatPdfSousMenu ".tif"
  !insertmacro AutomatPdfAction ".tif" "1" "images" "Mettre en PDF"
  !insertmacro AutomatPdfSousMenu ".tiff"
  !insertmacro AutomatPdfAction ".tiff" "1" "images" "Mettre en PDF"
  !insertmacro AutomatPdfSousMenu ".bmp"
  !insertmacro AutomatPdfAction ".bmp" "1" "images" "Mettre en PDF"
  !insertmacro AutomatPdfSousMenu ".gif"
  !insertmacro AutomatPdfAction ".gif" "1" "images" "Mettre en PDF"
  !insertmacro AutomatPdfSousMenu ".doc"
  !insertmacro AutomatPdfAction ".doc" "1" "officeEnPdf" "Convertir en PDF"
  !insertmacro AutomatPdfSousMenu ".docx"
  !insertmacro AutomatPdfAction ".docx" "1" "officeEnPdf" "Convertir en PDF"
  !insertmacro AutomatPdfSousMenu ".rtf"
  !insertmacro AutomatPdfAction ".rtf" "1" "officeEnPdf" "Convertir en PDF"
  !insertmacro AutomatPdfSousMenu ".odt"
  !insertmacro AutomatPdfAction ".odt" "1" "officeEnPdf" "Convertir en PDF"
  !insertmacro AutomatPdfSousMenu ".xls"
  !insertmacro AutomatPdfAction ".xls" "1" "officeEnPdf" "Convertir en PDF"
  !insertmacro AutomatPdfSousMenu ".xlsx"
  !insertmacro AutomatPdfAction ".xlsx" "1" "officeEnPdf" "Convertir en PDF"
  !insertmacro AutomatPdfSousMenu ".ods"
  !insertmacro AutomatPdfAction ".ods" "1" "officeEnPdf" "Convertir en PDF"
  !insertmacro AutomatPdfSousMenu ".ppt"
  !insertmacro AutomatPdfAction ".ppt" "1" "officeEnPdf" "Convertir en PDF"
  !insertmacro AutomatPdfSousMenu ".pptx"
  !insertmacro AutomatPdfAction ".pptx" "1" "officeEnPdf" "Convertir en PDF"
  !insertmacro AutomatPdfSousMenu ".odp"
  !insertmacro AutomatPdfAction ".odp" "1" "officeEnPdf" "Convertir en PDF"
  ; L'Explorateur relit les associations.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.pdf\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.jpg\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.jpeg\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.png\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.webp\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.tif\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.tiff\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.bmp\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.gif\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.doc\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.docx\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.rtf\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.odt\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.xls\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.xlsx\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.ods\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.ppt\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.pptx\shell\AutomatPDF"
  DeleteRegKey SHCTX "${AUTOMATPDF_CLASSES}\.odp\shell\AutomatPDF"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
