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

; En tête du sous-menu des PDF : le PDF s'ouvre dans le lecteur, sans outil demandé
; (« Automat-PDF.exe "fichier.pdf" »), comme par « Ouvrir avec » sur Mac.
!macro AutomatPdfLire EXT
  WriteRegStr SHCTX "${AUTOMATPDF_CLASSES}\${EXT}\shell\AutomatPDF\shell\0lire" "MUIVerb" "Ouvrir dans le lecteur"
  WriteRegStr SHCTX "${AUTOMATPDF_CLASSES}\${EXT}\shell\AutomatPDF\shell\0lire\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
!macroend

; Lecteur de PDF : Automat-PDF s'inscrit parmi les applications qui ouvrent les PDF (« Ouvrir avec »,
; Applications par défaut de Windows), sans prendre la place du lecteur actuel : Windows ne le permet
; qu'avec l'accord de l'utilisateur, que l'application demande ensuite (src-tauri/src/lecteur_defaut.rs).
!define AUTOMATPDF_CLASSE "AutomatPDF.Document"
!define AUTOMATPDF_CAPACITES "Software\Automat-PDF\Capabilities"

!macro AutomatPdfLecteur
  WriteRegStr SHCTX "Software\Classes\${AUTOMATPDF_CLASSE}" "" "Document PDF"
  WriteRegStr SHCTX "Software\Classes\${AUTOMATPDF_CLASSE}\DefaultIcon" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr SHCTX "Software\Classes\${AUTOMATPDF_CLASSE}\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
  WriteRegStr SHCTX "Software\Classes\.pdf\OpenWithProgids" "${AUTOMATPDF_CLASSE}" ""
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe" "FriendlyAppName" "Automat-PDF"
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe\SupportedTypes" ".pdf" ""
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe\shell\open\command" "" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\""
  WriteRegStr SHCTX "${AUTOMATPDF_CAPACITES}" "ApplicationName" "Automat-PDF"
  WriteRegStr SHCTX "${AUTOMATPDF_CAPACITES}" "ApplicationDescription" "Lire, annoter, imprimer et transformer vos PDF"
  WriteRegStr SHCTX "${AUTOMATPDF_CAPACITES}" "ApplicationIcon" "$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0"
  WriteRegStr SHCTX "${AUTOMATPDF_CAPACITES}\FileAssociations" ".pdf" "${AUTOMATPDF_CLASSE}"
  WriteRegStr SHCTX "Software\RegisteredApplications" "Automat-PDF" "${AUTOMATPDF_CAPACITES}"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro AutomatPdfLecteur
  !insertmacro AutomatPdfSousMenu ".pdf"
  !insertmacro AutomatPdfLire ".pdf"
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
  DeleteRegKey SHCTX "Software\Classes\${AUTOMATPDF_CLASSE}"
  DeleteRegValue SHCTX "Software\Classes\.pdf\OpenWithProgids" "${AUTOMATPDF_CLASSE}"
  DeleteRegKey SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe"
  DeleteRegKey SHCTX "${AUTOMATPDF_CAPACITES}"
  DeleteRegKey /ifempty SHCTX "Software\Automat-PDF"
  DeleteRegValue SHCTX "Software\RegisteredApplications" "Automat-PDF"
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
