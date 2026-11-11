import { useEffect, useState, type ReactNode } from "react";

/**
 * Vrai quand l'élément, collé au bas de ce qui défile, cache du contenu : il dépasse alors d'un
 * pixel (`bottom: -1px`), coupé par le bas de la fenêtre ou de la fenêtre de dialogue.
 */
function useCollee(element: HTMLElement | null): boolean {
  const [collee, setCollee] = useState(false);
  useEffect(() => {
    if (!element || typeof IntersectionObserver === "undefined") return;
    const observateur = new IntersectionObserver(
      ([entree]) => setCollee(entree.isIntersecting && entree.intersectionRect.bottom < entree.boundingClientRect.bottom - 0.5),
      { threshold: [0, 1] },
    );
    observateur.observe(element);
    return () => observateur.disconnect();
  }, [element]);
  return collee;
}

interface Props {
  children: ReactNode;
  /** Boutons de validation d'une fenêtre de dialogue (« Oui, j'accepte », « Enregistrer »). */
  pied?: boolean;
}

/**
 * Le bouton qui lance le travail reste à l'écran : quand la page (ou la fenêtre de dialogue) est
 * plus haute que la fenêtre, sur un portable ou un écran agrandi à 150 %, il reste collé en bas, et
 * une ombre le détache du contenu qui défile dessous. Sinon, il est à sa place, sans rien de plus.
 */
export default function BarreAction({ children, pied = false }: Props) {
  // L'élément est gardé dans l'état : l'observation commence dès qu'il existe.
  const [element, setElement] = useState<HTMLDivElement | null>(null);
  const collee = useCollee(element);
  return (
    <div ref={setElement} className={pied ? "barre-action pied-fenetre" : "barre-action"} data-barre-action data-collee={collee || undefined}>
      {children}
    </div>
  );
}
