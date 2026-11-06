import { notifications } from "@mantine/notifications";
import { IconAlertTriangle } from "@tabler/icons-react";
import { enErreur } from "../../coeur";
import { textes } from "../../textes";

/** Affiche une erreur (du cœur ou de l'interface) dans une notification, le temps de la lire. */
export function signalerErreur(e: unknown) {
  const { message, details } = enErreur(e);
  notifications.show({ color: "red", icon: <IconAlertTriangle size={18} />, title: textes.erreur, message: [message, ...details].join("\n"), autoClose: 15000 });
}
