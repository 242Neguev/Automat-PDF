// Formulaires PDF (0.7.0) : ce que l'interface décide seule, sans le cœur. Quels champs se
// remplissent, ce que donne un clic sur une case ou un bouton radio, la taille du texte saisi.
import type { ChampFormulaire, FormulairePdf, WidgetChamp } from "../coeur";
import type { ValeurChamp } from "./annotations";

/** Les sortes de champs que l'on remplit dans le lecteur (ni boutons poussoirs, ni signatures). */
const REMPLISSABLES = new Set(["texte", "case", "radio", "liste", "combo"]);

/** Un champ que l'on peut remplir : de ces sortes, pas en lecture seule, et affiché sur une page. */
export const remplissable = (champ: ChampFormulaire) => REMPLISSABLES.has(champ.sorte) && !champ.lectureSeule && champ.widgets.length > 0;

/** Un champ de signature où poser une signature dessinée : pas encore signé, affiché sur une page. */
export const signable = (champ: ChampFormulaire) => champ.sorte === "signature" && !champ.signe && !champ.lectureSeule && champ.widgets.length > 0;

/** Les valeurs reçues avec le PDF, par nom de champ. */
export function initialesDe(formulaire: FormulairePdf | null): Record<string, ValeurChamp> {
  const initiales: Record<string, ValeurChamp> = {};
  for (const champ of formulaire?.champs ?? []) initiales[champ.nom] = champ.valeur;
  return initiales;
}

/** Le premier texte d'une valeur (celle d'une liste à choix unique, d'un champ de texte). */
export const texteDe = (valeur: ValeurChamp | undefined) => (valeur === undefined ? "" : Array.isArray(valeur) ? (valeur[0] ?? "") : valeur);

/** Cette case, ou ce bouton radio, est-il coché ? */
export const coche = (widget: WidgetChamp, valeur: ValeurChamp | undefined) => widget.etat !== null && texteDe(valeur) === widget.etat;

/**
 * La valeur après un clic sur une case ou un bouton radio : cochée ou décochée (« Off ») ; un bouton
 * radio coché ne se décoche que si le formulaire le permet. `null` : rien ne change.
 */
export function apresClic(champ: ChampFormulaire, widget: WidgetChamp, valeur: ValeurChamp | undefined): ValeurChamp | null {
  if (!widget.etat) return null;
  if (!coche(widget, valeur)) return widget.etat;
  if (champ.sorte === "radio" && champ.sansDecocher) return null;
  return "Off";
}

/**
 * Taille du texte saisi, en points : celle que demande le PDF, sinon celle que le cœur choisira
 * (d'après la hauteur du champ, comme dans les lecteurs de PDF courants ; 12 sur plusieurs lignes).
 */
export function tailleSaisie(champ: ChampFormulaire, widget: WidgetChamp): number {
  if (widget.taille > 0) return widget.taille;
  if (champ.multiligne) return 12;
  return Math.max(4, Math.min(24, (widget.hauteur - 4) * 0.8));
}

/** Le texte affiché d'une valeur de liste : le libellé de l'option, ou la valeur écrite à la main. */
export function libelleDe(champ: ChampFormulaire, valeur: ValeurChamp | undefined): string {
  const valeurs = valeur === undefined ? [] : Array.isArray(valeur) ? valeur : [valeur];
  return valeurs.map((v) => champ.options.find((o) => o.valeur === v)?.libelle ?? v).join(", ");
}
