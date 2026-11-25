// Images des pages (et des fichiers image), demandées au cœur une seule fois : quelques rendus à la
// fois, les dernières demandes d'abord (ce sont les vignettes à l'écran), puis gardées pour la suite.
import { vignetteImage, vignettePdf } from "../../coeur";

const EN_MEME_TEMPS = 3;
const images = new Map<string, Promise<string>>();
const enAttente: (() => void)[] = [];
let enCours = 0;

function suivante() {
  while (enCours < EN_MEME_TEMPS && enAttente.length > 0) enAttente.pop()?.();
}

function planifier<T>(travail: () => Promise<T>): Promise<T> {
  return new Promise<T>((resoudre, rejeter) => {
    enAttente.push(() => {
      enCours++;
      travail()
        .then(resoudre, rejeter)
        .finally(() => {
          enCours--;
          suivante();
        });
    });
    suivante();
  });
}

function charger(cle: string, travail: () => Promise<ArrayBuffer>): Promise<string> {
  let image = images.get(cle);
  if (!image) {
    image = planifier(travail).then((octets) => URL.createObjectURL(new Blob([octets], { type: "image/jpeg" })));
    images.set(cle, image);
    image.catch(() => images.delete(cle));
  }
  return image;
}

/** Adresse (blob:) de l'image d'une page, large de `largeur` pixels. */
export const imagePage = (chemin: string, page: number, largeur: number) =>
  charger(`${chemin}\u0000${page}\u0000${largeur}`, () => vignettePdf(chemin, page, largeur));

/** Adresse (blob:) de la vignette d'un fichier image. */
export const imageFichier = (chemin: string, largeur: number) => charger(`${chemin}\u0000image\u0000${largeur}`, () => vignetteImage(chemin, largeur));

/** Événement : un fichier se relit autrement (PDF ouvert avec son mot de passe) ; ses vignettes se redemandent. */
export const FICHIER_RELU = "automat-pdf:fichier-relu";

/** Oublie les images d'un fichier, et fait redemander les siennes aux vignettes affichées (un échec compris). */
export function relire(chemin: string) {
  oublier(chemin);
  window.dispatchEvent(new CustomEvent<string>(FICHIER_RELU, { detail: chemin }));
}

/** Oublie les images d'un fichier qui vient d'être réécrit. */
export function oublier(chemin: string) {
  for (const [cle, image] of images) {
    if (cle.startsWith(`${chemin}\u0000`)) {
      images.delete(cle);
      image.then(URL.revokeObjectURL, () => undefined);
    }
  }
}
