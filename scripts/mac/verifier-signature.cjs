// Vérifie une signature de mise à jour d'Automat-PDF (format minisign de Tauri), comme le fera
// l'application installée : même clé publique (celle de src-tauri/tauri.conf.json), signature du
// fichier, puis signature du « commentaire de confiance » (horodatage, nom du fichier, version).
//
//   node verifier-signature.cjs <fichier> <fichier.sig> <clé publique en base64>
//
// Affiche le commentaire de confiance et sort avec 0 si tout est juste ; sinon, un message et un
// code non nul (2 : autre clé, 3 : fichier modifié, 4 : commentaire modifié, 1 : fichier illisible).
"use strict";
const crypto = require("crypto");
const fs = require("fs");

const [fichier, fichierSig, clePublique] = process.argv.slice(2);
if (!fichier || !fichierSig || !clePublique) {
  console.error("usage : node verifier-signature.cjs <fichier> <fichier.sig> <clé publique en base64>");
  process.exit(1);
}

// La clé publique et la signature de Tauri sont le texte de minisign, encodé en base64.
const lignes = (base64) => Buffer.from(base64.trim(), "base64").toString("utf8").split("\n");
try {
  const pub = Buffer.from(lignes(clePublique)[1], "base64"); // « Ed » + numéro de clé (8) + clé (32)
  const sig = lignes(fs.readFileSync(fichierSig, "utf8"));
  const signature = Buffer.from(sig[1], "base64"); // algorithme (2) + numéro de clé (8) + signature (64)
  if (!pub.subarray(2, 10).equals(signature.subarray(2, 10))) {
    const hex = (b) => Buffer.from(b).reverse().toString("hex").toUpperCase();
    console.error(`signée par une autre clé (${hex(signature.subarray(2, 10))}) que celle de l'application (${hex(pub.subarray(2, 10))})`);
    process.exit(2);
  }
  const cle = crypto.createPublicKey({
    key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), pub.subarray(10, 42)]),
    format: "der",
    type: "spki",
  });
  // « ED » : signature de l'empreinte BLAKE2b-512 du fichier (ce que fait Tauri) ; « Ed » : du fichier.
  const donnees = fs.readFileSync(fichier);
  const message = signature.subarray(0, 2).toString("latin1") === "ED" ? crypto.createHash("blake2b512").update(donnees).digest() : donnees;
  if (!crypto.verify(null, message, cle, signature.subarray(10, 74))) {
    console.error("signature fausse : le fichier a changé depuis sa signature");
    process.exit(3);
  }
  const confiance = sig[2].replace(/^trusted comment: /, "");
  const globale = Buffer.from(sig[3], "base64");
  if (!crypto.verify(null, Buffer.concat([signature.subarray(10, 74), Buffer.from(confiance, "utf8")]), cle, globale)) {
    console.error("commentaire de confiance modifié");
    process.exit(4);
  }
  console.log(confiance);
} catch (erreur) {
  console.error(`signature illisible : ${erreur.message}`);
  process.exit(1);
}
