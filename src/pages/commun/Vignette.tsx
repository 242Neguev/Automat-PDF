import { useEffect, useRef, useState } from "react";
import { Box, Loader, Text } from "@mantine/core";
import { IconFileOff } from "@tabler/icons-react";
import { imageFichier, imagePage } from "./vignettes";

interface Props {
  chemin: string;
  /** Page à afficher, à partir de 0 ; absente pour un fichier image. */
  page?: number;
  /** Rotation ajoutée par l'utilisateur, en degrés (sens horaire). */
  rotation?: number;
  /** Côté du carré où l'image s'inscrit, en pixels. */
  taille: number;
  /** Largeur de l'image demandée au cœur (plus grande que `taille` sur un écran haute définition). */
  largeur: number;
  alt: string;
}

/**
 * Image d'une page de PDF (ou d'un fichier image), chargée quand elle approche de l'écran. Elle
 * s'inscrit dans un carré : tournée d'un quart de tour, elle y tient toujours.
 */
export default function Vignette({ chemin, page, rotation = 0, taille, largeur, alt }: Props) {
  const cadre = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  const [image, setImage] = useState<string | null>(null);
  const [echec, setEchec] = useState(false);

  useEffect(() => {
    const element = cadre.current;
    if (!element) return;
    const observateur = new IntersectionObserver(
      ([entree]) => {
        if (entree.isIntersecting) {
          setVisible(true);
          observateur.disconnect();
        }
      },
      { rootMargin: "400px" },
    );
    observateur.observe(element);
    return () => observateur.disconnect();
  }, []);

  useEffect(() => {
    if (!visible) return;
    let actuelle = true;
    setEchec(false);
    (page === undefined ? imageFichier(chemin, largeur) : imagePage(chemin, page, largeur)).then(
      (adresse) => actuelle && setImage(adresse),
      () => actuelle && setEchec(true),
    );
    return () => {
      actuelle = false;
    };
  }, [visible, chemin, page, largeur]);

  return (
    <Box ref={cadre} w={taille} h={taille} style={{ display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
      {image ? (
        <img
          src={image}
          alt={alt}
          draggable={false}
          style={{
            maxWidth: "100%",
            maxHeight: "100%",
            transform: rotation ? `rotate(${rotation}deg)` : undefined,
            boxShadow: "0 1px 4px rgba(34, 37, 42, 0.25)",
            background: "white",
          }}
        />
      ) : echec ? (
        <Text c="dimmed" component="div" aria-label={alt}>
          <IconFileOff size={28} stroke={1.4} />
        </Text>
      ) : (
        <Loader size="sm" color="gray" />
      )}
    </Box>
  );
}
