/**
 * Automat-PDF : réception des statistiques anonymes d'utilisation (Google Apps Script).
 *
 * Attaché à un classeur Google Sheets, ce script reçoit le signal qu'Automat-PDF envoie à chaque
 * ouverture, seulement si la personne l'a accepté. Il tient trois feuilles :
 *   - « Signaux » : un signal par ligne ;
 *   - « Installations » : une ligne par installation (premier et dernier signal, ouvertures,
 *     jours d'utilisation, version) ;
 *   - « Synthèse » : les chiffres clés, recalculés par Google Sheets.
 * Il prévient par e-mail à chaque nouvelle installation, et envoie un résumé chaque semaine.
 *
 * Le signal ne contient qu'un numéro tiré au hasard pour l'installation, la version
 * d'Automat-PDF, celle de Windows, la langue et l'édition : ni nom, ni adresse, ni fichier.
 * Google ne transmet pas l'adresse IP de l'expéditeur au script.
 *
 * Mise en place et mise à jour : voir README.md, dans le même dossier.
 */

/** Réglages, à adapter avant de lancer « installer ». */
const REGLAGES = {
  // Adresse qui reçoit les alertes et le résumé ; vide : celle du compte Google propriétaire du script.
  destinataire: "",
  // Alertes « nouvelle installation » envoyées au plus par jour ; les suivantes attendent le résumé
  // (un script Gmail gratuit envoie au plus 100 e-mails par jour).
  alertesParJour: 20,
  // Dépôt GitHub dont le résumé compte les téléchargements ; vide : pas de comptage.
  depot: "242Neguev/Automat-PDF",
  // Résumé : jour de la semaine (MONDAY, TUESDAY…) et heure, au fuseau horaire du script.
  jourDuResume: "MONDAY",
  heureDuResume: 8,
};

const FEUILLES = { signaux: "Signaux", installations: "Installations", synthese: "Synthèse" };

const EN_TETES = {
  signaux: ["Date", "Installation", "Événement", "Version", "Système", "Langue", "Édition"],
  installations: ["N°", "Installation", "Première ouverture", "Dernière ouverture", "Ouvertures", "Jours d'utilisation", "Version", "Système", "Langue", "Édition"],
};

// Colonnes de la feuille « Installations » (à partir de 0).
const N = { numero: 0, installation: 1, premiere: 2, derniere: 3, ouvertures: 4, jours: 5, version: 6, systeme: 7, langue: 8, edition: 9 };

const EVENEMENTS = ["installation", "ouverture"];

/** Forme attendue de chaque champ : tout autre signal est écarté (rien n'est écrit). */
const FORMATS = {
  installation: /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
  version: /^\d{1,4}\.\d{1,4}\.\d{1,4}(?:[-+][0-9A-Za-z.-]{1,20})?$/,
  systeme: /^[A-Za-z0-9][A-Za-z0-9 ().,_-]{0,59}$/,
  langue: /^(?:[A-Za-z]{2,3}(?:-[A-Za-z0-9]{1,8}){0,3}|inconnue)$/,
  edition: /^[a-z]{1,12}$/,
};

const JOUR = 24 * 60 * 60 * 1000;

/** Signal envoyé par Automat-PDF. */
function doPost(e) {
  const signal = lireSignal(e);
  if (!signal) return repondre({ ok: false, erreur: "signal non valable" });
  const verrou = LockService.getScriptLock();
  if (!verrou.tryLock(20000)) return repondre({ ok: false, erreur: "service occupé" });
  let nouvelle;
  try {
    nouvelle = enregistrer(signal, new Date());
  } finally {
    verrou.releaseLock();
  }
  if (nouvelle) alerter(nouvelle);
  return repondre({ ok: true });
}

/** Adresse ouverte dans un navigateur : pour vérifier que le service répond. */
function doGet() {
  return ContentService.createTextOutput("Automat-PDF : le service des statistiques est en marche.");
}

function repondre(objet) {
  return ContentService.createTextOutput(JSON.stringify(objet)).setMimeType(ContentService.MimeType.JSON);
}

/** Le signal reçu, vérifié champ par champ ; null s'il n'a pas la forme attendue. */
function lireSignal(e) {
  let brut;
  try {
    brut = JSON.parse(e && e.postData ? e.postData.contents : "");
  } catch (erreur) {
    return null;
  }
  if (!brut || typeof brut !== "object" || EVENEMENTS.indexOf(brut.evenement) < 0) return null;
  const signal = { evenement: brut.evenement };
  for (const champ of Object.keys(FORMATS)) {
    if (typeof brut[champ] !== "string" || !FORMATS[champ].test(brut[champ])) return null;
    signal[champ] = brut[champ];
  }
  return signal;
}

/** Range le signal ; renvoie la nouvelle installation s'il en annonce une, sinon null. */
function enregistrer(signal, maintenant) {
  const classeur = classeurDesStatistiques();
  feuille(classeur, "signaux").appendRow([maintenant, signal.installation, signal.evenement, signal.version, signal.systeme, signal.langue, signal.edition]);
  const installations = feuille(classeur, "installations");
  const trouvee = installations.getRange("B:B").createTextFinder(signal.installation).matchEntireCell(true).findNext();
  if (trouvee) {
    const plage = installations.getRange(trouvee.getRow(), 1, 1, EN_TETES.installations.length);
    const ligne = plage.getValues()[0];
    const autreJour = jour(ligne[N.derniere]) !== jour(maintenant);
    ligne[N.derniere] = maintenant;
    ligne[N.ouvertures] = Number(ligne[N.ouvertures] || 0) + 1;
    ligne[N.jours] = Number(ligne[N.jours] || 0) + (autreJour ? 1 : 0);
    ligne[N.version] = signal.version;
    ligne[N.systeme] = signal.systeme;
    ligne[N.langue] = signal.langue;
    ligne[N.edition] = signal.edition;
    plage.setValues([ligne]);
    return null;
  }
  // Numéro d'ordre : la ligne d'en-tête compte pour zéro.
  const numero = installations.getLastRow();
  installations.appendRow([numero, signal.installation, maintenant, maintenant, 1, 1, signal.version, signal.systeme, signal.langue, signal.edition]);
  return { numero: numero, signal: signal, date: maintenant };
}

/** E-mail « nouvelle installation », dans la limite du jour. */
function alerter(nouvelle) {
  const proprietes = PropertiesService.getScriptProperties();
  const compteur = "alertes " + jour(nouvelle.date);
  const envoyees = Number(proprietes.getProperty(compteur) || 0);
  if (envoyees >= REGLAGES.alertesParJour) return;
  proprietes.setProperty(compteur, String(envoyees + 1));
  const s = nouvelle.signal;
  MailApp.sendEmail({
    to: destinataire(),
    subject: "Automat-PDF : nouvelle installation (n° " + nouvelle.numero + ")",
    body: [
      "Une nouvelle installation d'Automat-PDF vient d'envoyer son premier signal.",
      "",
      "Numéro d'ordre : " + nouvelle.numero,
      "Version : " + s.version,
      "Système : " + s.systeme,
      "Langue : " + s.langue,
      "Édition : " + s.edition,
      "",
      "Tableau des statistiques : " + classeurDesStatistiques().getUrl(),
    ].join("\n"),
  });
}

/** Résumé de la semaine, envoyé par le déclencheur créé par « installer ». */
function envoyerLeResume() {
  const classeur = classeurDesStatistiques();
  const maintenant = new Date();
  const semaine = new Date(maintenant.getTime() - 7 * JOUR);
  const mois = new Date(maintenant.getTime() - 30 * JOUR);
  const installations = lignes(feuille(classeur, "installations"));
  const signaux = lignes(feuille(classeur, "signaux"));
  const nouvelles = installations.filter((l) => l[N.premiere] >= semaine).length;
  const actives = installations.filter((l) => l[N.derniere] >= semaine).length;
  const activesMois = installations.filter((l) => l[N.derniere] >= mois).length;
  const ouvertures = signaux.filter((l) => l[0] >= semaine).length;
  const versions = compter(installations.filter((l) => l[N.derniere] >= mois).map((l) => l[N.version]));
  const github = telechargementsGithub();

  const corps = [
    "Résumé d'Automat-PDF, sur les 7 derniers jours.",
    "",
    "Statistiques anonymes (seulement les personnes qui les ont acceptées) :",
    "- nouvelles installations : " + nouvelles,
    "- installations actives : " + actives + " (sur 30 jours : " + activesMois + ")",
    "- ouvertures : " + ouvertures + (actives > 0 ? ", soit " + arrondi(ouvertures / actives) + " par installation active" : ""),
    "- installations depuis le début : " + installations.length,
    "- versions utilisées (30 jours) : " + (versions.length ? versions.map((v) => v[0] + " : " + v[1]).join(" ; ") : "aucune"),
  ];
  if (github) {
    corps.push(
      "",
      "Sur GitHub (tout le monde, y compris les personnes qui ont refusé les statistiques) :",
      "- téléchargements de l'installateur : " + github.installateurs + " au total" + depuis(github.installateursDepuis) + ", mises à jour comprises",
      "- recherches de mise à jour : " + github.recherches + " au total" + depuis(github.recherchesDepuis) + " (environ une par ouverture d'Automat-PDF, quand la recherche des mises à jour est activée)",
    );
  }
  corps.push("", "Tableau complet : " + classeur.getUrl());
  MailApp.sendEmail({ to: destinataire(), subject: "Automat-PDF : résumé de la semaine", body: corps.join("\n") });
  oublierLesAnciensCompteurs(jour(maintenant));
}

/** Téléchargements comptés par GitHub, et ce qui a changé depuis le résumé précédent ; null si GitHub ne répond pas. */
function telechargementsGithub() {
  if (!REGLAGES.depot) return null;
  try {
    const reponse = UrlFetchApp.fetch("https://api.github.com/repos/" + REGLAGES.depot + "/releases?per_page=100", {
      muteHttpExceptions: true,
      headers: { Accept: "application/vnd.github+json" },
    });
    if (reponse.getResponseCode() !== 200) return null;
    let installateurs = 0;
    let recherches = 0;
    JSON.parse(reponse.getContentText()).forEach((publication) =>
      (publication.assets || []).forEach((fichier) => {
        if (/\.(exe|msi)$/i.test(fichier.name)) installateurs += fichier.download_count;
        if (fichier.name === "latest.json") recherches += fichier.download_count;
      }),
    );
    const proprietes = PropertiesService.getScriptProperties();
    const avant = JSON.parse(proprietes.getProperty("github") || "null");
    proprietes.setProperty("github", JSON.stringify({ installateurs: installateurs, recherches: recherches }));
    return {
      installateurs: installateurs,
      recherches: recherches,
      installateursDepuis: avant ? installateurs - avant.installateurs : null,
      recherchesDepuis: avant ? recherches - avant.recherches : null,
    };
  } catch (erreur) {
    return null;
  }
}

/**
 * À lancer une fois depuis l'éditeur (bouton « Exécuter ») : crée les feuilles, la synthèse et le
 * résumé hebdomadaire. Sans risque si on le relance : rien n'est effacé.
 */
function installer() {
  const classeur = SpreadsheetApp.getActiveSpreadsheet();
  PropertiesService.getScriptProperties().setProperty("classeur", classeur.getId());
  classeur.setSpreadsheetTimeZone(Session.getScriptTimeZone());
  feuille(classeur, "signaux");
  feuille(classeur, "installations");
  if (!classeur.getSheetByName(FEUILLES.synthese)) creerLaSynthese(classeur);
  // La feuille vide créée avec le classeur ne sert à rien.
  classeur.getSheets().forEach((f) => {
    if (Object.values(FEUILLES).indexOf(f.getName()) < 0 && f.getLastRow() === 0) classeur.deleteSheet(f);
  });
  ScriptApp.getProjectTriggers()
    .filter((d) => d.getHandlerFunction() === "envoyerLeResume")
    .forEach((d) => ScriptApp.deleteTrigger(d));
  ScriptApp.newTrigger("envoyerLeResume").timeBased().onWeekDay(ScriptApp.WeekDay[REGLAGES.jourDuResume]).atHour(REGLAGES.heureDuResume).create();
  Logger.log("Installé. Les alertes iront à " + destinataire() + ". Déployez maintenant le script comme application Web (voir README.md).");
}

/** Feuille « Synthèse » : chiffres clés, recalculés par Google Sheets à chaque ouverture. */
function creerLaSynthese(classeur) {
  const f = classeur.insertSheet(FEUILLES.synthese, 0);
  const lignesDeSynthese = [
    ["Chiffre", "Valeur"],
    ["Installations depuis le début", "=COUNTA(Installations!B2:B)"],
    // SUMPRODUCT plutôt que COUNTIF : aucune date écrite en texte, quelle que soit la langue du classeur.
    ["Nouvelles installations, 7 derniers jours", "=SUMPRODUCT(--(Installations!C2:C>=NOW()-7))"],
    ["Installations actives, 7 derniers jours", "=SUMPRODUCT(--(Installations!D2:D>=NOW()-7))"],
    ["Installations actives, 30 derniers jours", "=SUMPRODUCT(--(Installations!D2:D>=NOW()-30))"],
    ["Ouvertures, 7 derniers jours", "=SUMPRODUCT(--(Signaux!A2:A>=NOW()-7))"],
    ["Ouvertures, 30 derniers jours", "=SUMPRODUCT(--(Signaux!A2:A>=NOW()-30))"],
    ["Ouvertures par installation active, 7 derniers jours", "=IF(B4>0,ROUND(B6/B4,1),0)"],
  ];
  f.getRange(1, 1, lignesDeSynthese.length, 2).setValues(lignesDeSynthese);
  f.getRange("A10").setValue("Dernière version connue de chaque installation");
  f.getRange("A11").setFormula("=QUERY(Installations!B2:G,\"select G, count(B) where B is not null group by G label G 'Version', count(B) 'Installations'\",0)");
  f.getRange("A1:B1").setFontWeight("bold");
  f.getRange("A10").setFontWeight("bold");
  f.setColumnWidth(1, 380);
  f.setColumnWidth(2, 120);
}

/** La feuille demandée, créée avec ses en-têtes si elle manque. */
function feuille(classeur, cle) {
  let f = classeur.getSheetByName(FEUILLES[cle]);
  if (!f) {
    f = classeur.insertSheet(FEUILLES[cle]);
    const enTetes = EN_TETES[cle];
    f.getRange(1, 1, 1, enTetes.length).setValues([enTetes]).setFontWeight("bold");
    f.setFrozenRows(1);
    const dates = cle === "signaux" ? ["A2:A"] : ["C2:C", "D2:D"];
    dates.forEach((plage) => f.getRange(plage).setNumberFormat("dd/mm/yyyy hh:mm"));
  }
  return f;
}

/** Lignes d'une feuille, sans les en-têtes. */
function lignes(f) {
  const derniere = f.getLastRow();
  return derniere < 2 ? [] : f.getRange(2, 1, derniere - 1, f.getLastColumn()).getValues();
}

/** Valeurs comptées, de la plus fréquente à la moins fréquente : [[valeur, nombre], …]. */
function compter(valeurs) {
  const comptes = {};
  valeurs.forEach((v) => (comptes[v] = (comptes[v] || 0) + 1));
  return Object.keys(comptes)
    .map((v) => [v, comptes[v]])
    .sort((a, b) => b[1] - a[1]);
}

function classeurDesStatistiques() {
  const id = PropertiesService.getScriptProperties().getProperty("classeur");
  return id ? SpreadsheetApp.openById(id) : SpreadsheetApp.getActiveSpreadsheet();
}

function destinataire() {
  return REGLAGES.destinataire || Session.getEffectiveUser().getEmail();
}

/** Jour d'une date, au fuseau horaire du script (« 2026-11-09 ») ; vide si ce n'est pas une date. */
function jour(date) {
  return date instanceof Date ? Utilities.formatDate(date, Session.getScriptTimeZone(), "yyyy-MM-dd") : "";
}

function arrondi(nombre) {
  return String(Math.round(nombre * 10) / 10).replace(".", ",");
}

function depuis(ecart) {
  return ecart === null ? "" : ", dont " + ecart + " depuis le résumé précédent";
}

/** Les compteurs d'alertes des jours passés ne servent plus. */
function oublierLesAnciensCompteurs(aujourdhui) {
  const proprietes = PropertiesService.getScriptProperties();
  proprietes
    .getKeys()
    .filter((cle) => cle.indexOf("alertes ") === 0 && cle !== "alertes " + aujourdhui)
    .forEach((cle) => proprietes.deleteProperty(cle));
}
