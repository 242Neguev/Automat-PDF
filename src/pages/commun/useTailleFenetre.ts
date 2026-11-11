import { useEffect, useState } from "react";

/** Taille de la fenêtre, en pixels de l'interface, tenue à jour quand on l'agrandit ou la redimensionne. */
export function useTailleFenetre(): { largeur: number; hauteur: number } {
  const [taille, setTaille] = useState(() => ({ largeur: window.innerWidth, hauteur: window.innerHeight }));
  useEffect(() => {
    const changer = () => setTaille({ largeur: window.innerWidth, hauteur: window.innerHeight });
    window.addEventListener("resize", changer);
    return () => window.removeEventListener("resize", changer);
  }, []);
  return taille;
}
