// Formulaires PDF (0.7.0) : les champs d'un PDF se remplissent dans le lecteur, sur la page. La page
// montre leur valeur telle que le PDF l'écrira : chaque champ rempli passe par le cœur, qui l'écrit
// (valeur et apparence) dans sa copie en mémoire, puis la page est redessinée. Pendant la saisie, le
// champ devient une case de saisie ; la valeur saisie reste affichée jusqu'à la page redessinée.
import { useEffect, useState, type CSSProperties, type KeyboardEvent } from "react";
import { Alert, Button, Checkbox, Group, Text, Tooltip } from "@mantine/core";
import { IconDeviceFloppy, IconForms, IconSignature } from "@tabler/icons-react";
import type { ChampFormulaire, FormulairePdf, WidgetChamp } from "../../coeur";
import { memeValeur, type ValeurChamp, type Zone } from "../../donnees/annotations";
import { apresClic, coche, libelleDe, remplissable, signable, tailleSaisie, texteDe } from "../../donnees/formulaire";
import { dansLeCadre } from "../../donnees/signature";
import { textes } from "../../textes";

const t = textes.lecteur.formulaire;

interface PropsCouche {
  page: number;
  echelle: number;
  champs: ChampFormulaire[];
  /** Valeur actuelle de chaque champ (reçue, ou remplie dans le lecteur). */
  valeurs: Record<string, ValeurChamp>;
  /** Version de l'image de la page affichée (−1 : pas encore là). */
  versionImage: number;
  onChanger: (champ: ChampFormulaire, valeur: ValeurChamp) => void;
  /** Signatures dessinées déjà posées sur cette page : un champ de signature qui en a une n'invite plus à signer. */
  signees?: Zone[];
  /** Signer dans un champ de signature. */
  onSigner?: (champ: ChampFormulaire, widget: WidgetChamp) => void;
}

/** Une valeur saisie, montrée par-dessus la page jusqu'à ce que la page redessinée l'affiche. */
interface Attente {
  valeur: ValeurChamp;
  /** Version de l'image affichée au moment de la saisie. */
  version: number;
}

/** Les champs d'une page, posés à leur place (en pixels de la page droite). */
export function CoucheFormulaire({ page, echelle, champs, valeurs, versionImage, onChanger, signees = [], onSigner }: PropsCouche) {
  const [edition, setEdition] = useState<string | null>(null);
  const [attentes, setAttentes] = useState<Record<string, Attente>>({});

  // La page redessinée montre les valeurs saisies avant : elles ne sont plus à montrer ici.
  useEffect(() => {
    setAttentes((avant) => {
      const restes = Object.entries(avant).filter(([, attente]) => attente.version >= versionImage);
      return restes.length === Object.keys(avant).length ? avant : Object.fromEntries(restes);
    });
  }, [versionImage]);

  function changer(champ: ChampFormulaire, valeur: ValeurChamp) {
    if (memeValeur(valeur, valeurs[champ.nom])) return;
    setAttentes((avant) => ({ ...avant, [champ.nom]: { valeur, version: versionImage } }));
    onChanger(champ, valeur);
  }

  const widgets = champs
    .filter((champ) => remplissable(champ) || (onSigner && signable(champ)))
    .flatMap((champ) => champ.widgets.map((widget, rang) => ({ champ, widget, cle: `${champ.nom}#${rang}` })));
  return (
    <div className="couche-formulaire" data-formulaire-page={page + 1}>
      {widgets
        .filter(({ widget }) => widget.page === page)
        .map(({ champ, widget, cle }) =>
          champ.sorte === "signature" ? (
            // Un champ de signature : on y signe d'un clic, tant qu'aucune signature n'y est posée.
            signees.some((zone) => dansLeCadre(zone, widget)) ? null : (
              <button
                key={cle}
                type="button"
                className="champ-pdf champ-pdf--signature"
                style={{ left: widget.x * echelle, top: widget.y * echelle, width: widget.largeur * echelle, height: widget.hauteur * echelle }}
                title={champ.libelle ?? textes.lecteur.signature.signerIci}
                onClick={() => onSigner?.(champ, widget)}
                data-champ={champ.nom}
                data-sorte="signature"
              >
                <IconSignature size={Math.max(12, Math.min(20, widget.hauteur * echelle * 0.45))} stroke={1.8} />
                <span>{textes.lecteur.signature.signerIci}</span>
              </button>
            )
          ) : (
          <ChampSurLaPage
            key={cle}
            champ={champ}
            widget={widget}
            echelle={echelle}
            valeur={attentes[champ.nom]?.valeur ?? valeurs[champ.nom]}
            enAttente={champ.nom in attentes}
            edition={edition === cle}
            onEdition={(ouvert) => setEdition(ouvert ? cle : (avant) => (avant === cle ? null : avant))}
            onChanger={(valeur) => changer(champ, valeur)}
          />
          ),
        )}
    </div>
  );
}

interface PropsChamp {
  champ: ChampFormulaire;
  widget: WidgetChamp;
  echelle: number;
  valeur: ValeurChamp | undefined;
  /** Saisie pas encore redessinée sur la page : la montrer ici. */
  enAttente: boolean;
  edition: boolean;
  onEdition: (ouvert: boolean) => void;
  onChanger: (valeur: ValeurChamp) => void;
}

function ChampSurLaPage({ champ, widget, echelle, valeur, enAttente, edition, onEdition, onChanger }: PropsChamp) {
  const place: CSSProperties = {
    left: widget.x * echelle,
    top: widget.y * echelle,
    width: widget.largeur * echelle,
    height: widget.hauteur * echelle,
  };
  const libelle = champ.libelle ?? champ.nom;
  const titre = champ.obligatoire ? t.obligatoire(libelle) : libelle;
  const commun = {
    title: titre,
    "aria-label": titre,
    "data-champ": champ.nom,
    "data-sorte": champ.sorte,
    "data-obligatoire": champ.obligatoire || undefined,
  };

  if (champ.sorte === "case" || champ.sorte === "radio") {
    const estCoche = coche(widget, valeur);
    return (
      <button
        type="button"
        className="champ-pdf champ-pdf--case"
        style={place}
        role={champ.sorte === "radio" ? "radio" : "checkbox"}
        aria-checked={estCoche}
        onClick={() => {
          const suivante = apresClic(champ, widget, valeur);
          if (suivante !== null) onChanger(suivante);
        }}
        data-coche={estCoche || undefined}
        data-etat={widget.etat ?? undefined}
        {...commun}
      >
        {enAttente && estCoche && <span className="champ-pdf__coche" style={{ fontSize: Math.min(widget.largeur, widget.hauteur) * echelle * 0.8 }}>{champ.sorte === "radio" ? "●" : "✔"}</span>}
      </button>
    );
  }

  const taille = tailleSaisie(champ, widget) * echelle;
  const alignement: CSSProperties["textAlign"] = widget.alignement === 1 ? "center" : widget.alignement === 2 ? "right" : "left";
  const styleSaisie: CSSProperties = {
    fontSize: taille,
    textAlign: alignement,
    justifyContent: widget.alignement === 1 ? "center" : widget.alignement === 2 ? "flex-end" : "flex-start",
  };

  if (champ.sorte === "combo" && !champ.saisieLibre) {
    // Un menu invisible sur le champ : un clic l'ouvre ; la page montre l'option choisie.
    return (
      <div className="champ-pdf" style={place} data-edition={edition || undefined} {...commun}>
        {enAttente && <span className="champ-pdf__attente" style={styleSaisie}>{libelleDe(champ, valeur)}</span>}
        <select
          className="champ-pdf__menu"
          value={texteDe(valeur)}
          aria-label={titre}
          onFocus={() => onEdition(true)}
          onBlur={() => onEdition(false)}
          onChange={(evenement) => onChanger(evenement.currentTarget.value)}
        >
          {!champ.options.some((o) => o.valeur === texteDe(valeur)) && <option value={texteDe(valeur)}>{texteDe(valeur) || t.choisir}</option>}
          {champ.options.map((option) => (
            <option key={option.valeur} value={option.valeur}>
              {option.libelle}
            </option>
          ))}
        </select>
      </div>
    );
  }

  if (champ.sorte === "liste") {
    const choisies = valeur === undefined ? [] : Array.isArray(valeur) ? valeur : valeur === "" ? [] : [valeur];
    return (
      <div className="champ-pdf" style={place} data-edition={edition || undefined} {...commun}>
        {edition ? (
          <select
            className="champ-pdf__liste"
            multiple={champ.multiple}
            size={Math.max(2, Math.min(champ.options.length, 8))}
            value={champ.multiple ? choisies : (choisies[0] ?? "")}
            autoFocus
            style={{ fontSize: taille }}
            onBlur={() => onEdition(false)}
            onChange={(evenement) => {
              const choix = [...evenement.currentTarget.selectedOptions].map((o) => o.value);
              onChanger(champ.multiple ? choix : (choix[0] ?? ""));
              if (!champ.multiple) onEdition(false);
            }}
          >
            {champ.options.map((option) => (
              <option key={option.valeur} value={option.valeur}>
                {option.libelle}
              </option>
            ))}
          </select>
        ) : (
          <button type="button" className="champ-pdf__ouvrir" aria-label={titre} onClick={() => onEdition(true)} onFocus={() => onEdition(true)}>
            {enAttente && <span className="champ-pdf__attente" style={styleSaisie}>{libelleDe(champ, valeur)}</span>}
          </button>
        )}
      </div>
    );
  }

  // Texte (et liste déroulante où l'on écrit sa valeur) : une case de saisie pendant l'édition.
  const valider = (texte: string) => {
    onEdition(false);
    onChanger(texte);
  };
  const touche = (evenement: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    if (evenement.key === "Escape") {
      evenement.preventDefault();
      onEdition(false);
    } else if (evenement.key === "Enter" && !(champ.multiligne && !evenement.ctrlKey && !evenement.metaKey)) {
      evenement.preventDefault();
      valider(evenement.currentTarget.value);
    }
  };
  const texte = texteDe(valeur);
  const listeId = champ.sorte === "combo" ? `options-${champ.nom}` : undefined;
  return (
    <div className="champ-pdf" style={place} data-edition={edition || undefined} {...commun}>
      {edition ? (
        champ.multiligne ? (
          <textarea className="champ-pdf__saisie" defaultValue={texte} autoFocus maxLength={champ.longueurMax ?? undefined} style={styleSaisie} onKeyDown={touche} onBlur={(evenement) => valider(evenement.currentTarget.value)} />
        ) : (
          <>
            <input
              className="champ-pdf__saisie"
              type={champ.secret ? "password" : "text"}
              defaultValue={texte}
              autoFocus
              maxLength={champ.longueurMax ?? undefined}
              list={listeId}
              style={{ ...styleSaisie, letterSpacing: champ.peigne && champ.longueurMax ? `${Math.max(0, (widget.largeur / champ.longueurMax - tailleSaisie(champ, widget) * 0.56) * echelle)}px` : undefined }}
              onKeyDown={touche}
              onBlur={(evenement) => valider(evenement.currentTarget.value)}
            />
            {listeId && (
              <datalist id={listeId}>
                {champ.options.map((option) => (
                  <option key={option.valeur} value={option.valeur}>
                    {option.libelle}
                  </option>
                ))}
              </datalist>
            )}
          </>
        )
      ) : (
        <button type="button" className="champ-pdf__ouvrir" aria-label={titre} onClick={() => onEdition(true)} onFocus={() => onEdition(true)}>
          {enAttente && (
            <span className="champ-pdf__attente" style={{ ...styleSaisie, whiteSpace: champ.multiligne ? "pre-wrap" : "nowrap" }}>
              {champ.secret ? "•".repeat(texte.length) : texte}
            </span>
          )}
        </button>
      )}
    </div>
  );
}

interface PropsBandeau {
  formulaire: FormulairePdf;
  /** Des champs à remplir (sinon, seulement des champs où signer). */
  aRemplir: boolean;
  /** Champs remplis dans le lecteur (pas encore enregistrés). */
  remplis: number;
  /** Une signature dessinée est posée (pas encore enregistrée). */
  signe: boolean;
  aplatir: boolean;
  onAplatir: (aplatir: boolean) => void;
  enregistrement: boolean;
  onEnregistrer: () => void;
}

/** Le bandeau d'un formulaire : comment le remplir, et l'enregistrer (aplati, au choix). */
export function BandeauFormulaire({ formulaire, aRemplir, remplis, signe, aplatir, onAplatir, enregistrement, onEnregistrer }: PropsBandeau) {
  if (formulaire.xfaSeul) {
    return (
      <Alert color="orange" variant="light" icon={<IconForms size={20} />} py={8} data-bandeau-formulaire="xfa">
        <Text size="sm">{t.xfaSeul}</Text>
      </Alert>
    );
  }
  return (
    <Alert color="blue" variant="light" icon={<IconForms size={20} />} py={8} data-bandeau-formulaire>
      <Group justify="space-between" gap="sm" wrap="wrap">
        <div style={{ minWidth: 0, flex: 1 }}>
          <Text size="sm">{aRemplir ? t.bandeau : t.bandeauSignature}</Text>
          {formulaire.calculs && (
            <Text size="xs" c="dimmed">
              {t.calculs}
            </Text>
          )}
        </div>
        <Group gap="sm" wrap="nowrap">
          <Text size="xs" c="dimmed" data-champs-remplis={remplis}>
            {remplis > 0 || !signe ? [t.modifies(remplis), signe ? t.signe : null].filter(Boolean).join(" · ") : t.signeSeul}
          </Text>
          <Tooltip label={t.aplatirAide} multiline w={300}>
            <Checkbox size="xs" label={t.aplatir} checked={aplatir} onChange={(evenement) => onAplatir(evenement.currentTarget.checked)} data-aplatir />
          </Tooltip>
          <Tooltip label={t.enregistrerAide}>
            <Button size="xs" radius="xl" leftSection={<IconDeviceFloppy size={15} />} loading={enregistrement} disabled={remplis === 0 && !signe && !aplatir} onClick={onEnregistrer} data-enregistrer-formulaire>
              {remplis === 0 && signe ? t.enregistrerSigne : t.enregistrer}
            </Button>
          </Tooltip>
        </Group>
      </Group>
    </Alert>
  );
}
