// Automat-PDF, lecteur de PDF par défaut : quand le proposer.

/** Ce que dit le système : le proposer est-il possible, est-ce déjà fait ? (`EtatLecteurPdf` du cœur) */
interface Etat {
  possible: boolean;
  parDefaut: boolean;
}

/**
 * La proposition se montre-t-elle ? Seulement si le système le permet (Windows avec Automat-PDF
 * installé, macOS 12 et plus), si ce n'est pas déjà fait, ni refusé (« Ne plus proposer »), ni remis
 * à plus tard pendant cette session.
 */
export const aProposer = (etat: Etat | null, refus: boolean, masquee: boolean) => !!etat && etat.possible && !etat.parDefaut && !refus && !masquee;
