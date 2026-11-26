// Reconnaître, en mémoire, le texte d'un PDF numérisé (0.8.0, OCR), avec l'avancement page par page :
// le lecteur (bandeau « Ce PDF semble numérisé ») et Séparer par personne.
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { reconnaitreLecture, type BilanReconnaissance } from "../../coeur";

/** Pages lues, pages à lire. */
export type AvancementReconnaissance = [number, number];

export function useReconnaissance() {
  const [enCours, setEnCours] = useState(false);
  const [avancement, setAvancement] = useState<AvancementReconnaissance | null>(null);

  useEffect(() => {
    const arret = listen<AvancementReconnaissance>("progression-reconnaissance", (evenement) => setAvancement(evenement.payload)).catch(() => () => {});
    return () => void arret.then((arreter) => arreter());
  }, []);

  /** Reconnaît le texte du PDF, en mémoire : le texte se lit ensuite comme celui d'un PDF ordinaire. */
  async function lancer(chemin: string): Promise<BilanReconnaissance> {
    setEnCours(true);
    setAvancement(null);
    try {
      return await reconnaitreLecture(chemin);
    } finally {
      setEnCours(false);
      setAvancement(null);
    }
  }

  return { enCours, avancement, lancer };
}
