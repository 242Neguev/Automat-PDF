// Avancement du téléchargement d'une mise à jour, d'après les événements du greffon updater de Tauri.
import type { DownloadEvent } from "@tauri-apps/plugin-updater";

export interface Telechargement {
  /** Octets reçus. */
  recus: number;
  /** Taille annoncée par le serveur ; null si elle ne l'est pas. */
  total: number | null;
}

export const TELECHARGEMENT_VIDE: Telechargement = { recus: 0, total: null };

/** Le téléchargement après un événement du greffon. */
export function avancer(suivi: Telechargement, evenement: DownloadEvent): Telechargement {
  switch (evenement.event) {
    case "Started": {
      const total = evenement.data.contentLength;
      return { recus: 0, total: total !== undefined && total > 0 ? total : null };
    }
    case "Progress":
      return { ...suivi, recus: suivi.recus + evenement.data.chunkLength };
    case "Finished":
      return suivi.total === null ? suivi : { ...suivi, recus: suivi.total };
  }
}

/** Pour cent reçus, de 0 à 100 ; null tant que la taille n'est pas connue. */
export function pourcentage({ recus, total }: Telechargement): number | null {
  if (total === null) return null;
  return Math.min(100, Math.floor((recus / total) * 100));
}

/**
 * Le fichier téléchargé a été refusé au contrôle de sa signature : ce n'est pas celui que le projet
 * a signé pour cette version. Réessayer n'y changerait rien. Reconnu d'après les messages (en
 * anglais) du greffon updater et de minisign, qui vérifie la signature.
 */
export function signatureRefusee(message: string): boolean {
  return /signature|signed for version|minisign/i.test(message);
}
