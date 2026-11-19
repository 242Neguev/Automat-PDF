// Le système de l'ordinateur, pour adapter les textes (Windows ou macOS). La vue web de macOS
// (WebKit) se présente comme « Macintosh », celle de Windows (WebView2) comme « Windows » : lu une
// fois, au chargement de l'interface.

/** L'interface tourne sur un Mac. */
export const MAC = estMac(typeof navigator === "undefined" ? "" : navigator.userAgent);

/** D'après la présentation du navigateur (user agent). */
export function estMac(presentation: string): boolean {
  return /Macintosh|Mac OS X/.test(presentation) && !/iPhone|iPad/.test(presentation);
}

/** L'ordinateur, tel qu'on le nomme dans les textes : « ce Mac » ou « ce PC ». */
export const CET_ORDINATEUR = MAC ? "ce Mac" : "ce PC";
