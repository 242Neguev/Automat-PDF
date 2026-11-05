// Ce qu'il faut dire, sur les pages de conversion, de ce qui manque à ce PC pour convertir.
import type { Convertisseurs } from "../coeur";

/**
 * LibreOffice sur la page Office en PDF : il manque (ni Office ni LibreOffice, rien ne peut
 * convertir), il est conseillé (Office est là, LibreOffice prendrait le relais), ou rien à dire.
 */
export function besoinLibreoffice(convertisseurs: Convertisseurs | null): "manque" | "conseille" | null {
  if (!convertisseurs || convertisseurs.libreoffice) return null;
  return convertisseurs.office ? "conseille" : "manque";
}

/** Durée lisible : « 45 s », « 2 min 05 s ». */
export function dureeLisible(secondes: number): string {
  const entieres = Math.max(0, Math.floor(secondes));
  const minutes = Math.floor(entieres / 60);
  const reste = entieres % 60;
  return minutes === 0 ? `${reste} s` : `${minutes} min ${String(reste).padStart(2, "0")} s`;
}
