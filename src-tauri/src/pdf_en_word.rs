//! PDF en document Word modifiable, sans Microsoft Word : le texte de chaque page est remis en
//! paragraphes (police, taille, gras, italique, couleur, alignement, retraits, espacements), les
//! colonnes d'un tableau alignées par des taquets de tabulation, les images reprises à leur place ;
//! chaque page du PDF donne une page du document, au même format. Une page faite seulement de
//! dessins est reprise en image. La mise en page est plus simple qu'avec Word : à relire.

use std::cell::OnceCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use lopdf::content::{Content, Operation};
use lopdf::{Object, Stream, dictionary};

use crate::contenu::{self, Caractere, Page};
use crate::docx::{
    self, Alignement, Document, Element, Format, ImagePosee, Paragraphe, Section, Style,
};
use crate::erreur::Erreur;
use crate::pdf::charger;
use crate::rendu::{self, FormatImage};

/// Hauteur d'une ligne isolée, en tailles de police.
const INTERLIGNE_SIMPLE: f64 = 1.15;
/// Place de la ligne de base dans la hauteur d'une ligne, quand l'interligne est exact (Word).
const BASE_DANS_LA_LIGNE: f64 = 0.8;
/// Écart, en tailles de police, au-delà duquel deux morceaux d'une ligne sont deux colonnes.
const ECART_COLONNES: f64 = 1.5;
/// Écart, en tailles de police, qui sépare deux colonnes dans un tableau (une espace entre deux
/// mots d'une cellule en fait environ 0,3).
const ECART_COLONNES_TABLEAU: f64 = 0.6;
/// Écart, en points, entre les débuts de mots d'une même colonne d'un tableau.
const ALIGNEMENT: f64 = 2.0;
/// Résolution d'une page reprise en image.
const DPI_PAGE: u32 = 150;

/// Un morceau de ligne écrit d'un seul tenant, entre deux grands blancs (colonnes d'un tableau).
#[derive(Debug, Clone)]
struct Segment {
    debut: f64,
    fin: f64,
    morceaux: Vec<(String, Style)>,
}

impl Segment {
    fn nouveau(debut: f64) -> Self {
        Segment {
            debut,
            fin: debut,
            morceaux: Vec::new(),
        }
    }

    fn ajouter(&mut self, texte: &str, style: &Style) {
        if texte == " "
            && self
                .morceaux
                .last()
                .is_none_or(|(avant, _)| avant.ends_with(' '))
        {
            return;
        }
        match self.morceaux.last_mut() {
            Some((avant, son_style)) if son_style == style => avant.push_str(texte),
            _ => self.morceaux.push((texte.to_string(), style.clone())),
        }
    }

    /// Sans blanc à la fin.
    fn finir(&mut self) {
        while let Some((texte, _)) = self.morceaux.last_mut() {
            texte.truncate(texte.trim_end().len());
            if !texte.is_empty() {
                break;
            }
            self.morceaux.pop();
        }
    }

    fn texte(&self) -> String {
        self.morceaux
            .iter()
            .map(|(texte, _)| texte.as_str())
            .collect()
    }
}

#[derive(Debug, Clone)]
struct Ligne {
    /// Ligne de base, vers le bas.
    base: f64,
    /// Plus grande taille de police de la ligne.
    taille: f64,
    segments: Vec<Segment>,
    /// Fin du premier mot : aurait-il tenu au bout de la ligne précédente ?
    fin_premier_mot: f64,
}

impl Ligne {
    fn debut(&self) -> f64 {
        self.segments[0].debut
    }

    fn fin(&self) -> f64 {
        self.segments[self.segments.len() - 1].fin
    }

    /// Ligne d'une liste : elle commence par une puce ou un numéro (« 1. », « a) »).
    fn commence_par_une_puce(&self) -> bool {
        let texte = self.segments[0].texte();
        let premier = texte.split_whitespace().next().unwrap_or_default();
        if matches!(
            premier,
            "•" | "–" | "—" | "-" | "▪" | "◦" | "·" | "*" | "‣" | "■" | "□" | "➢" | "✓"
        ) {
            return true;
        }
        let Some(numero) = premier
            .strip_suffix('.')
            .or_else(|| premier.strip_suffix(')'))
        else {
            return false;
        };
        (1..=3).contains(&numero.len())
            && (numero.chars().all(|c| c.is_ascii_digit())
                || (numero.len() == 1 && numero.chars().all(|c| c.is_ascii_lowercase())))
    }
}

/// Mise en forme d'un caractère dans le document. Les fonds colorés ne sont pas repris : un texte
/// très clair (blanc sur un bandeau) passe en noir, comme le texte invisible d'un scan reconnu.
fn style_de(caractere: &Caractere) -> Style {
    let [rouge, vert, bleu] = caractere.couleur;
    let clair =
        0.299 * f64::from(rouge) + 0.587 * f64::from(vert) + 0.114 * f64::from(bleu) > 200.0;
    Style {
        famille: caractere.police.famille.clone(),
        taille: (caractere.taille * 2.0).round() / 2.0,
        gras: caractere.police.gras,
        italique: caractere.police.italique,
        couleur: if clair || caractere.invisible {
            [0, 0, 0]
        } else {
            caractere.couleur
        },
    }
}

/// Un caractère visible d'une ligne ; `mot` : il commence un mot (après une espace ou un écart).
struct Glyphe<'c> {
    caractere: &'c Caractere,
    mot: bool,
}

/// Les caractères visibles d'une ligne, de gauche à droite ; un caractère redessiné au même
/// endroit (faux gras, contour) ne compte qu'une fois.
fn glyphes_de(mut caracteres: Vec<&Caractere>) -> Vec<Glyphe<'_>> {
    caracteres.sort_by(|a, b| a.debut.total_cmp(&b.debut));
    let mut glyphes: Vec<Glyphe<'_>> = Vec::new();
    let mut blanc = false;
    for caractere in caracteres {
        if caractere.texte.trim().is_empty() {
            blanc = true;
            continue;
        }
        let mot = match glyphes.last() {
            Some(avant) => {
                let avant = avant.caractere;
                if caractere.texte == avant.texte
                    && (caractere.debut - avant.debut).abs() < 0.2 * caractere.taille
                {
                    continue;
                }
                blanc || caractere.debut - avant.fin > 0.2 * avant.taille.max(caractere.taille)
            }
            None => true,
        };
        glyphes.push(Glyphe { caractere, mot });
        blanc = false;
    }
    glyphes
}

/// Rangs des caractères qui ouvrent une colonne après un grand blanc.
fn grands_blancs(glyphes: &[Glyphe<'_>]) -> BTreeSet<usize> {
    (1..glyphes.len())
        .filter(|&rang| {
            let (avant, apres) = (glyphes[rang - 1].caractere, glyphes[rang].caractere);
            apres.debut - avant.fin > ECART_COLONNES * avant.taille.max(apres.taille)
        })
        .collect()
}

/// Ce qu'une ligne d'un tableau dit d'une colonne possible.
#[derive(Clone, Copy, PartialEq)]
enum Appui {
    /// La ligne y commence, un mot y commence après un vrai blanc, ou bien une cellule y commence
    /// sous le texte trop long de la précédente.
    Fort,
    /// Un mot y commence après une simple espace.
    Faible,
    /// Le milieu d'un mot : deux cellules qui se touchent, ou un mot qui passe là par hasard.
    Touche,
}

/// Coupe les lignes `rangs` à la colonne `(min, max)` si elle est sûre (voir `colonnes_alignees`).
fn couper_a_la_colonne(
    lignes: &[Vec<Glyphe<'_>>],
    rangs: std::ops::Range<usize>,
    (min, max): (f64, f64),
    colonnes: &[(f64, f64)],
    coupures: &mut [BTreeSet<usize>],
) {
    let zone = |(min, max): (f64, f64)| min - 0.5..=max + 0.5;
    let ici = zone((min, max));
    let mut appuis: Vec<(usize, usize, Appui)> = Vec::new();
    let mut traversees = 0;
    for rang in rangs {
        let ligne = &lignes[rang];
        let fin_avant = |position: usize| {
            ligne[..position]
                .iter()
                .map(|glyphe| glyphe.caractere.fin)
                .fold(f64::NEG_INFINITY, f64::max)
        };
        let dedans: Vec<usize> = (0..ligne.len())
            .filter(|&position| ici.contains(&ligne[position].caractere.debut))
            .collect();
        let Some(&premier) = dedans.first() else {
            // Rien ne commence dans la colonne : un mot la traverse-t-il ?
            if ligne.iter().any(|glyphe| {
                glyphe.caractere.debut < *ici.start() && glyphe.caractere.fin > *ici.end()
            }) {
                traversees += 1;
            }
            continue;
        };
        let appui = dedans.into_iter().find(|&position| {
            position == 0
                || ligne[position].mot
                || fin_avant(position) > ligne[position].caractere.debut + 0.5
        });
        appuis.push(match appui {
            Some(0) => (rang, 0, Appui::Fort),
            Some(position) => {
                let caractere = ligne[position].caractere;
                let ecart = caractere.debut - fin_avant(position);
                let taille = caractere.taille.max(ligne[position - 1].caractere.taille);
                if ecart < -0.5 || ecart > ECART_COLONNES_TABLEAU * taille {
                    (rang, position, Appui::Fort)
                } else {
                    (rang, position, Appui::Faible)
                }
            }
            None => (rang, premier, Appui::Touche),
        });
    }
    let debut =
        |&(rang, position, _): &(usize, usize, Appui)| lignes[rang][position].caractere.debut;
    let reperes: Vec<f64> = appuis
        .iter()
        .filter(|appui| appui.2 == Appui::Fort)
        .map(debut)
        .collect();
    let faibles = appuis
        .iter()
        .filter(|appui| appui.2 == Appui::Faible)
        .count();
    if reperes.is_empty() || reperes.len() < faibles {
        return;
    }
    // Sans vrai blanc avant elle, une coupure doit tomber pile sur les autres débuts de la colonne.
    let bas = reperes.iter().copied().fold(f64::INFINITY, f64::min);
    let haut = reperes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let pile = |x: f64| haut - bas <= 0.3 && (bas - 0.15..=haut + 0.15).contains(&x);
    let mut coupes = Vec::new();
    let (mut soutiens, mut contraires) = (0, traversees);
    for appui in &appuis {
        let (rang, position, sorte) = *appui;
        let coupe = match sorte {
            Appui::Fort => true,
            Appui::Faible => pile(debut(appui)),
            Appui::Touche => {
                // Deux cellules qui se touchent : la première commence elle-même à une colonne.
                let depart = coupures[rang]
                    .range(..position)
                    .next_back()
                    .copied()
                    .unwrap_or(0);
                let cellule = lignes[rang][depart].caractere.debut;
                pile(debut(appui))
                    && colonnes
                        .iter()
                        .any(|&colonne| zone(colonne).contains(&cellule))
            }
        };
        if sorte == Appui::Touche && !coupe {
            contraires += 1;
        } else {
            soutiens += 1;
        }
        if coupe && position > 0 {
            coupes.push((rang, position));
        }
    }
    if 2 * contraires < soutiens {
        for (rang, position) in coupes {
            coupures[rang].insert(position);
        }
    }
}

/// Colonnes d'un tableau serré : dans une suite d'au moins trois lignes qui ont déjà un grand
/// blanc, une abscisse où au moins trois lignes commencent un mot est peut-être une colonne. Elle
/// l'est si la plupart de ces lignes y ont un vrai blanc avant (pas une simple espace, comme entre
/// les chiffres d'un montant), et si peu de lignes ont un mot qui la traverse : dans un texte
/// aligné à droite, des débuts de mots se rencontrent par hasard. Les lignes y sont alors coupées
/// où un mot commence après un vrai blanc, où commence une cellule que le texte trop long de la
/// précédente recouvre, ou, pile à la colonne, entre deux cellules qui se touchent (texte tronqué
/// par Excel) si la première commence elle-même à une colonne ; jamais au milieu d'un mot.
fn colonnes_alignees(lignes: &[Vec<Glyphe<'_>>], coupures: &mut [BTreeSet<usize>]) {
    let base = |rang: usize| lignes[rang][0].caractere.ligne;
    let taille = |rang: usize| {
        lignes[rang]
            .iter()
            .map(|glyphe| glyphe.caractere.taille)
            .fold(0.0, f64::max)
    };
    let mut debut = 0;
    while debut < lignes.len() {
        let mut fin = debut;
        while fin < lignes.len()
            && !coupures[fin].is_empty()
            && (fin == debut || base(fin) - base(fin - 1) <= 2.5 * taille(fin).max(taille(fin - 1)))
        {
            fin += 1;
        }
        if fin - debut >= 3 {
            // Dans un tableau, un écart moindre suffit à séparer deux cellules.
            for rang in debut..fin {
                for position in 1..lignes[rang].len() {
                    let (avant, apres) = (
                        lignes[rang][position - 1].caractere,
                        lignes[rang][position].caractere,
                    );
                    if apres.debut - avant.fin
                        > ECART_COLONNES_TABLEAU * avant.taille.max(apres.taille)
                    {
                        coupures[rang].insert(position);
                    }
                }
            }
            let mut departs: Vec<(f64, usize)> = (debut..fin)
                .flat_map(|rang| {
                    lignes[rang]
                        .iter()
                        .filter(|glyphe| glyphe.mot)
                        .map(move |glyphe| (glyphe.caractere.debut, rang))
                })
                .collect();
            departs.sort_by(|a, b| a.0.total_cmp(&b.0));
            // Départs proches les uns des autres : une colonne, de `min` à `max`.
            let mut colonnes: Vec<(f64, f64)> = Vec::new();
            let mut groupe: Vec<(f64, usize)> = Vec::new();
            for depart in departs.into_iter().chain([(f64::INFINITY, usize::MAX)]) {
                if let Some(&(precedent, _)) = groupe.last()
                    && depart.0 - precedent > ALIGNEMENT
                {
                    let lignes_du_groupe: HashSet<usize> =
                        groupe.iter().map(|&(_, rang)| rang).collect();
                    if lignes_du_groupe.len() >= 3 {
                        colonnes.push((groupe[0].0, precedent));
                    }
                    groupe.clear();
                }
                groupe.push(depart);
            }
            // De gauche à droite : une cellule coupée à une colonne peut en toucher une autre.
            for &colonne in &colonnes {
                couper_a_la_colonne(lignes, debut..fin, colonne, &colonnes, coupures);
            }
        }
        debut = fin.max(debut + 1);
    }
}

/// Une ligne : ses mots, et ses colonnes séparées aux `coupures`.
fn ligne_de(glyphes: &[Glyphe<'_>], coupures: &BTreeSet<usize>) -> Option<Ligne> {
    // Les caractères qui débordent sur la colonne suivante étaient cachés par elle (cellule
    // tronquée) : Word, lui, sauterait au taquet d'après et décalerait toute la ligne.
    let mut caches = HashSet::new();
    let debuts: Vec<usize> = std::iter::once(0).chain(coupures.iter().copied()).collect();
    for paire in debuts.windows(2) {
        let limite = glyphes[paire[1]].caractere.debut + 0.5;
        caches
            .extend((paire[0] + 1..paire[1]).filter(|&rang| glyphes[rang].caractere.fin > limite));
    }
    let mut segments: Vec<Segment> = Vec::new();
    let mut fin_premier_mot: Option<f64> = None;
    for (rang, glyphe) in glyphes.iter().enumerate() {
        if caches.contains(&rang) {
            continue;
        }
        let caractere = glyphe.caractere;
        if rang == 0 || coupures.contains(&rang) {
            if rang > 0 {
                fin_premier_mot.get_or_insert(glyphes[rang - 1].caractere.fin);
            }
            segments.push(Segment::nouveau(caractere.debut));
        } else if glyphe.mot {
            let avant = glyphes[rang - 1].caractere;
            fin_premier_mot.get_or_insert(avant.fin);
            segments.last_mut()?.ajouter(" ", &style_de(avant));
        }
        let segment = segments.last_mut()?;
        segment.ajouter(&caractere.texte, &style_de(caractere));
        segment.fin = segment.fin.max(caractere.fin);
    }
    for segment in &mut segments {
        segment.finir();
    }
    segments.retain(|segment| !segment.morceaux.is_empty());
    let dernier = glyphes.last()?.caractere;
    (!segments.is_empty()).then(|| Ligne {
        base: glyphes
            .iter()
            .map(|glyphe| glyphe.caractere.ligne)
            .sum::<f64>()
            / glyphes.len() as f64,
        taille: glyphes
            .iter()
            .map(|glyphe| glyphe.caractere.taille)
            .fold(0.0, f64::max),
        segments,
        fin_premier_mot: fin_premier_mot.unwrap_or(dernier.fin),
    })
}

/// Les lignes de texte, de haut en bas : les caractères dont la ligne de base est la même, coupés
/// en colonnes aux grands blancs et aux colonnes alignées d'un tableau.
fn lignes(mut caracteres: Vec<&Caractere>) -> Vec<Ligne> {
    caracteres.sort_by(|a, b| a.ligne.total_cmp(&b.ligne));
    let mut groupes: Vec<Vec<&Caractere>> = Vec::new();
    for caractere in caracteres {
        match groupes.last_mut() {
            Some(groupe)
                if (caractere.ligne - groupe[0].ligne).abs()
                    <= 0.4 * groupe[0].taille.min(caractere.taille) =>
            {
                groupe.push(caractere);
            }
            _ => groupes.push(vec![caractere]),
        }
    }
    let glyphes: Vec<Vec<Glyphe<'_>>> = groupes
        .into_iter()
        .map(glyphes_de)
        .filter(|glyphes| !glyphes.is_empty())
        .collect();
    let mut coupures: Vec<BTreeSet<usize>> = glyphes
        .iter()
        .map(|glyphes| grands_blancs(glyphes))
        .collect();
    colonnes_alignees(&glyphes, &mut coupures);
    glyphes
        .iter()
        .zip(&coupures)
        .filter_map(|(glyphes, coupures)| ligne_de(glyphes, coupures))
        .collect()
}

/// La ligne continue-t-elle le paragraphe ? Même taille, écart régulier, même bord gauche (la
/// première ligne peut être en retrait), pas de puce, et la ligne précédente était pleine : le
/// premier mot de celle-ci n'y aurait pas tenu. C'est un retour à la ligne automatique. Tant que
/// le paragraphe n'a qu'une ligne, « pleine » veut dire jusqu'au bord droit du texte de la page :
/// deux lignes courtes (« Cordialement, », « Le service RH ») restent deux paragraphes.
fn continue_le_paragraphe(bloc: &[Ligne], ligne: &Ligne, droite_page: f64) -> bool {
    let derniere = &bloc[bloc.len() - 1];
    if derniere.segments.len() > 1 || ligne.segments.len() > 1 || ligne.commence_par_une_puce() {
        return false;
    }
    let taille = derniere.taille.max(ligne.taille);
    if (derniere.taille - ligne.taille).abs() > 1.0 {
        return false;
    }
    let ecart = ligne.base - derniere.base;
    if !(0.9 * taille..=1.75 * taille).contains(&ecart) {
        return false;
    }
    if bloc.len() >= 2 {
        let interligne = bloc[1].base - bloc[0].base;
        if (ecart - interligne).abs() > 0.2 * taille
            || (ligne.debut() - bloc[1].debut()).abs() > 0.6 * taille
        {
            return false;
        }
    } else if (derniere.debut() - ligne.debut()).abs() > 6.0 * taille {
        return false;
    }
    let droite = if bloc.len() == 1 {
        droite_page
    } else {
        bloc.iter()
            .map(Ligne::fin)
            .fold(f64::NEG_INFINITY, f64::max)
    }
    .max(ligne.fin());
    let premier_mot = ligne.fin_premier_mot - ligne.debut() + 0.3 * taille;
    droite - derniere.fin() < premier_mot
}

/// Les paragraphes de la page : des lignes groupées.
fn blocs(lignes: Vec<Ligne>, droite_page: f64) -> Vec<Vec<Ligne>> {
    let mut blocs: Vec<Vec<Ligne>> = Vec::new();
    for ligne in lignes {
        match blocs.last_mut() {
            Some(bloc) if continue_le_paragraphe(bloc, &ligne, droite_page) => bloc.push(ligne),
            _ => blocs.push(vec![ligne]),
        }
    }
    blocs
}

fn ajouter_texte(elements: &mut Vec<Element>, texte: &str, style: &Style) {
    match elements.last_mut() {
        Some(Element::Texte(avant, son_style)) if son_style == style => avant.push_str(texte),
        _ => elements.push(Element::Texte(texte.to_string(), style.clone())),
    }
}

/// Le texte d'un paragraphe : ses lignes jointes par une espace (sans espace après un trait
/// d'union en fin de ligne), ses colonnes séparées par des tabulations.
fn elements_de(bloc: &[Ligne]) -> Vec<Element> {
    let mut elements = Vec::new();
    for (rang, ligne) in bloc.iter().enumerate() {
        if rang > 0
            && let Some(Element::Texte(avant, style)) = elements.last()
            && !avant.ends_with('-')
        {
            let style = style.clone();
            ajouter_texte(&mut elements, " ", &style);
        }
        for (colonne, segment) in ligne.segments.iter().enumerate() {
            if colonne > 0 {
                elements.push(Element::Tabulation);
            }
            for (texte, style) in &segment.morceaux {
                ajouter_texte(&mut elements, texte, style);
            }
        }
    }
    elements
}

/// Médiane d'une liste non vide.
fn mediane(mut valeurs: Vec<f64>) -> f64 {
    valeurs.sort_by(f64::total_cmp);
    valeurs[valeurs.len() / 2]
}

/// Les images du document, chacune une seule fois.
#[derive(Default)]
struct Images {
    rangs: HashMap<u128, usize>,
    liste: Vec<(Arc<Vec<u8>>, Format)>,
}

impl Images {
    fn rang(&mut self, cle: u128, octets: &Arc<Vec<u8>>, format: Format) -> usize {
        *self.rangs.entry(cle).or_insert_with(|| {
            self.liste.push((octets.clone(), format));
            self.liste.len() - 1
        })
    }
}

/// Bord gauche et bord droit du texte de la page : ce seront ses marges.
fn bords(lignes: &[Ligne], largeur: f64) -> (f64, f64) {
    let gauche = lignes
        .iter()
        .map(Ligne::debut)
        .fold(f64::INFINITY, f64::min);
    let droite = lignes
        .iter()
        .map(Ligne::fin)
        .fold(f64::NEG_INFINITY, f64::max);
    if gauche.is_finite() && droite > gauche {
        (gauche.clamp(0.0, largeur), droite.clamp(0.0, largeur))
    } else {
        (72.0, (largeur - 72.0).max(72.0))
    }
}

/// Alignement et retraits d'un paragraphe, d'après la place de ses lignes entre les bords du texte.
/// Une ligne isolée (titre, ligne de tableau) peut déborder jusqu'au bord de la page, où sa police
/// de remplacement, un peu plus large, la couperait sinon en deux.
fn disposition(bloc: &[Ligne], gauche_page: f64, droite_page: f64, largeur: f64) -> Paragraphe {
    let marge_droite = (largeur - droite_page).max(0.0);
    let premiere = &bloc[0];
    let taille = bloc.iter().map(|ligne| ligne.taille).fold(0.0, f64::max);
    if bloc.len() >= 2 {
        let gauche = bloc[1..]
            .iter()
            .map(Ligne::debut)
            .fold(f64::INFINITY, f64::min);
        let droite = bloc
            .iter()
            .map(Ligne::fin)
            .fold(f64::NEG_INFINITY, f64::max);
        let justifie = bloc.len() >= 3
            && bloc[..bloc.len() - 1]
                .iter()
                .all(|ligne| droite - ligne.fin() <= 0.6 * taille);
        return Paragraphe {
            alignement: if justifie {
                Alignement::Justifie
            } else {
                Alignement::Gauche
            },
            retrait_gauche: gauche - gauche_page,
            premiere_ligne: premiere.debut() - gauche,
            // Word coupe alors les lignes à la même largeur que le PDF.
            retrait_droit: (droite_page - droite).max(0.0),
            ..Paragraphe::default()
        };
    }
    if premiere.segments.len() > 1 {
        return Paragraphe {
            retrait_gauche: premiere.debut() - gauche_page,
            retrait_droit: -marge_droite,
            taquets: premiere.segments[1..]
                .iter()
                .map(|segment| segment.debut - gauche_page)
                .collect(),
            ..Paragraphe::default()
        };
    }
    let centre = (premiere.debut() + premiere.fin()) / 2.0;
    let centre_zone = (gauche_page + droite_page) / 2.0;
    let tolerance = taille.max(0.02 * (droite_page - gauche_page));
    if (centre - centre_zone).abs() <= tolerance && premiere.debut() - gauche_page > 2.0 * taille {
        // Même débord des deux côtés : le centre ne bouge pas.
        let debord = gauche_page.min(marge_droite);
        return Paragraphe {
            alignement: Alignement::Centre,
            retrait_gauche: -debord,
            retrait_droit: -debord,
            ..Paragraphe::default()
        };
    }
    if (droite_page - premiere.fin()).abs() <= taille && premiere.debut() > centre_zone {
        return Paragraphe {
            alignement: Alignement::Droite,
            retrait_gauche: -gauche_page,
            ..Paragraphe::default()
        };
    }
    Paragraphe {
        retrait_gauche: premiere.debut() - gauche_page,
        retrait_droit: -marge_droite,
        ..Paragraphe::default()
    }
}

/// La section d'une page : ses paragraphes, placés où ils étaient, et ses images.
fn section_de(page: &Page, images: &mut Images) -> Section {
    let (horizontaux, autres): (Vec<&Caractere>, Vec<&Caractere>) = page
        .caracteres
        .iter()
        .partition(|caractere| caractere.sens == 0);
    let lignes_page = lignes(horizontaux);
    let (gauche_page, droite_page) = bords(&lignes_page, page.largeur);
    let mut section = Section {
        largeur: page.largeur,
        hauteur: page.hauteur,
        marge_gauche: gauche_page,
        marge_droite: (page.largeur - droite_page).max(0.0),
        marge_haut: 36.0,
        marge_bas: 36.0,
        paragraphes: Vec::new(),
    };
    let blocs = blocs(lignes_page, droite_page);
    // Où Word aura fini le paragraphe précédent : chaque espace est compté depuis là, pour que
    // les écarts ne s'additionnent pas.
    let mut curseur: Option<f64> = None;
    for (rang, bloc) in blocs.iter().enumerate() {
        let taille = bloc.iter().map(|ligne| ligne.taille).fold(0.0, f64::max);
        let interligne = if bloc.len() >= 2 {
            mediane(
                bloc.windows(2)
                    .map(|deux| deux[1].base - deux[0].base)
                    .collect(),
            )
        } else {
            // Ligne isolée : sa hauteur ordinaire, sans dépasser la ligne suivante (lignes serrées
            // d'un tableau).
            let suivante = blocs
                .get(rang + 1)
                .map_or(f64::INFINITY, |suivant| suivant[0].base - bloc[0].base);
            (INTERLIGNE_SIMPLE * taille).min(suivante).max(taille)
        };
        let haut = bloc[0].base - BASE_DANS_LA_LIGNE * interligne;
        let depuis = *curseur.get_or_insert_with(|| {
            section.marge_haut = haut.max(0.0);
            section.marge_haut
        });
        let espace_avant = (haut - depuis).max(0.0);
        curseur = Some(depuis + espace_avant + bloc.len() as f64 * interligne);
        section.paragraphes.push(Paragraphe {
            espace_avant,
            interligne,
            elements: elements_de(bloc),
            ..disposition(bloc, gauche_page, droite_page, page.largeur)
        });
    }
    // Le texte écrit dans un autre sens (marges) suit, au fil du texte.
    for ligne in lignes(autres) {
        let interligne = INTERLIGNE_SIMPLE * ligne.taille;
        curseur = Some(curseur.unwrap_or(section.marge_haut) + interligne);
        section.paragraphes.push(Paragraphe {
            interligne,
            elements: elements_de(std::slice::from_ref(&ligne)),
            ..Paragraphe::default()
        });
    }
    if let Some(curseur) = curseur {
        section.marge_bas = (page.hauteur - curseur - 1.0).clamp(0.0, 36.0);
    }
    // L'image d'un scan reconnu est remplacée par son texte ; une grande image est un fond.
    let lisibles: Vec<&Caractere> = page
        .caracteres
        .iter()
        .filter(|caractere| !caractere.texte.trim().is_empty())
        .collect();
    let invisibles = lisibles
        .iter()
        .filter(|caractere| caractere.invisible)
        .count();
    let scan_reconnu = !lisibles.is_empty() && 2 * invisibles > lisibles.len();
    let surface = (page.largeur * page.hauteur).max(1.0);
    let mut posees = Vec::new();
    for image in &page.images {
        let couverture = image.largeur * image.hauteur / surface;
        if scan_reconnu && couverture > 0.5 {
            continue;
        }
        posees.push(ImagePosee {
            image: images.rang(image.cle, &image.octets, image.format),
            gauche: image.gauche,
            haut: image.haut,
            largeur: image.largeur,
            hauteur: image.hauteur,
            derriere: couverture > 0.5 && !lisibles.is_empty(),
        });
    }
    if !posees.is_empty() {
        if section.paragraphes.is_empty() {
            section.paragraphes.push(Paragraphe {
                interligne: 12.0,
                ..Paragraphe::default()
            });
        }
        section.paragraphes[0].images = posees;
    }
    section
}

/// Opérateurs de l'état graphique (couleur, épaisseur…) : gardés même dans un texte, ils valent
/// pour les tracés qui suivent.
const ETAT_GRAPHIQUE: [&str; 18] = [
    "rg", "RG", "g", "G", "k", "K", "cs", "CS", "sc", "SC", "scn", "SCN", "gs", "ri", "w", "J",
    "j", "d",
];
/// Taille au-delà de laquelle un contenu décompressé n'est pas traité (fichier piégé).
const CONTENU_MAX: usize = 64 * 1024 * 1024;

/// Un contenu sans son texte ni ses images en ligne ; `true` s'il peut y rester un tracé.
fn sans_texte(operations: Vec<Operation>) -> (Vec<Operation>, bool) {
    let mut gardees = Vec::new();
    let mut dans_un_texte = false;
    let mut trace = false;
    for operation in operations {
        match operation.operator.as_str() {
            "BT" => dans_un_texte = true,
            "ET" => dans_un_texte = false,
            "BI" | "ID" | "EI" => {}
            operateur if dans_un_texte => {
                if ETAT_GRAPHIQUE.contains(&operateur) {
                    gardees.push(operation);
                }
            }
            operateur => {
                trace |= matches!(
                    operateur,
                    "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "S" | "s" | "sh" | "Do"
                );
                gardees.push(operation);
            }
        }
    }
    (gardees, trace)
}

/// Le PDF sans son texte ni ses images, pour en rendre les tracés (traits, cadres des tableaux,
/// fonds de couleur, logos dessinés) : chaque image devient un formulaire vide, le texte est
/// retiré des pages et des formulaires. Rend aussi les pages qui peuvent garder des tracés.
fn sans_texte_ni_images(octets: &[u8], nom: &str) -> Option<(Arc<Vec<u8>>, HashSet<usize>)> {
    let mut document = charger(octets, nom).ok()?;
    for objet in document.objects.values_mut() {
        let Object::Stream(flux) = objet else {
            continue;
        };
        match flux.dict.get(b"Subtype").and_then(Object::as_name) {
            Ok(b"Image") => {
                *flux = Stream::new(
                    dictionary! {
                        "Type" => "XObject",
                        "Subtype" => "Form",
                        "BBox" => vec![0.into(), 0.into(), 0.into(), 0.into()],
                    },
                    Vec::new(),
                );
            }
            Ok(b"Form") => {
                if let Some(contenu) = flux
                    .get_plain_content_with_limit(CONTENU_MAX)
                    .ok()
                    .and_then(|octets| Content::decode(&octets).ok())
                    && let Ok(allege) = (Content {
                        operations: sans_texte(contenu.operations).0,
                    })
                    .encode()
                {
                    flux.set_plain_content(allege);
                }
            }
            _ => {}
        }
    }
    let mut avec_traces = HashSet::new();
    for (numero, page) in document.get_pages() {
        let Ok(contenu) = Content::decode(&document.get_page_content(page)) else {
            continue;
        };
        let (gardees, trace) = sans_texte(contenu.operations);
        if trace {
            avec_traces.insert(numero as usize - 1);
        }
        let allege = Content {
            operations: gardees,
        }
        .encode()
        .ok()?;
        document.change_page_content(page, allege).ok()?;
    }
    if avec_traces.is_empty() {
        return None;
    }
    let mut allege = Vec::new();
    document.save_to(&mut allege).ok()?;
    Some((Arc::new(allege), avec_traces))
}

/// Le document Word des pages lues. `page_entiere` donne l'image d'une page faite seulement de
/// dessins ; `traces`, celle des seuls tracés d'une page (traits, cadres, fonds), posée derrière
/// son texte : les tableaux gardent leurs bordures.
fn mettre_en_page(
    pages: &[Page],
    titre: &str,
    page_entiere: impl Fn(usize) -> Option<Vec<u8>>,
    traces: impl Fn(usize) -> Option<Vec<u8>>,
) -> Document {
    let mut images = Images::default();
    let mut sections = Vec::new();
    for (rang, page) in pages.iter().enumerate() {
        let mut section = section_de(page, &mut images);
        let pleine_page = |image, derriere| ImagePosee {
            image,
            gauche: 0.0,
            haut: 0.0,
            largeur: page.largeur,
            hauteur: page.hauteur,
            derriere,
        };
        // Des tracés (ou des caractères illisibles) : la page entière en image si elle n'a rien
        // d'autre, sinon ses seuls tracés, sous tout le reste (première image de la page).
        let vide = section.paragraphes.is_empty();
        if page.autres_traces > 0
            && let Some(octets) = if vide {
                page_entiere(rang)
            } else {
                traces(rang)
            }
        {
            let cle = if vide { u128::MAX } else { u128::MAX / 2 } - rang as u128;
            let image = images.rang(cle, &Arc::new(octets), Format::Png);
            if vide {
                section.paragraphes.push(Paragraphe {
                    interligne: 12.0,
                    ..Paragraphe::default()
                });
            }
            section.paragraphes[0]
                .images
                .insert(0, pleine_page(image, !vide));
        }
        sections.push(section);
    }
    Document {
        titre: titre.to_string(),
        sections,
        images: images.liste,
    }
}

/// Le document Word (.docx) d'un PDF.
pub fn convertir(octets: Arc<Vec<u8>>, nom: &str) -> Result<Vec<u8>, Erreur> {
    let pages = contenu::lire(octets.clone(), nom)?;
    let titre = nom
        .rsplit_once('.')
        .map_or(nom, |(debut, _)| debut)
        .to_string();
    // Le PDF sans texte ni images n'est préparé qu'à la première page qui a des tracés.
    let allege = OnceCell::new();
    let document = mettre_en_page(
        &pages,
        &titre,
        |rang| rendu::exporter_page(octets.clone(), nom, rang, DPI_PAGE, FormatImage::Png).ok(),
        |rang| {
            let (allege, avec_traces) = allege
                .get_or_init(|| sans_texte_ni_images(&octets, nom))
                .as_ref()?;
            if !avec_traces.contains(&rang) {
                return None;
            }
            rendu::rendre_sur_transparent(allege.clone(), nom, rang, DPI_PAGE)
                .ok()
                .flatten()
        },
    );
    docx::ecrire(&document)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use super::*;
    use crate::contenu::{ImagePlacee, Police};
    use crate::essais::pdf_herite;

    fn police(famille: &str, gras: bool) -> Arc<Police> {
        Arc::new(Police {
            famille: famille.into(),
            gras,
            italique: false,
        })
    }

    /// Les caractères d'un texte écrit à partir de `x` sur la ligne `base`, chacun large d'une
    /// demi-taille (espaces comprises).
    fn ecrire(texte: &str, x: f64, base: f64, taille: f64, police: &Arc<Police>) -> Vec<Caractere> {
        texte
            .chars()
            .enumerate()
            .map(|(rang, c)| {
                let debut = x + rang as f64 * taille / 2.0;
                Caractere {
                    texte: c.to_string(),
                    sens: 0,
                    debut,
                    fin: debut + taille / 2.0,
                    ligne: base,
                    taille,
                    police: police.clone(),
                    couleur: [0, 0, 0],
                    invisible: false,
                }
            })
            .collect()
    }

    fn page(caracteres: Vec<Caractere>) -> Page {
        Page {
            largeur: 595.0,
            hauteur: 842.0,
            caracteres,
            ..Page::default()
        }
    }

    fn texte(paragraphe: &Paragraphe) -> String {
        paragraphe
            .elements
            .iter()
            .map(|element| match element {
                Element::Texte(texte, _) => texte.as_str(),
                Element::Tabulation => "\t",
            })
            .collect()
    }

    fn document(pages: &[Page]) -> Document {
        mettre_en_page(pages, "essai", |_| None, |_| None)
    }

    #[test]
    fn remet_les_lignes_d_un_paragraphe_ensemble() {
        let arial = police("Arial", false);
        let mut caracteres = Vec::new();
        // Trois lignes pleines puis une courte, justifiées entre 72 et 522 (taille 12 : 75
        // caractères par ligne), interligne 14.
        let pleine = "Ceci est une ligne de texte assez longue pour remplir toute la largeur ok";
        for rang in 0..3 {
            caracteres.extend(ecrire(
                pleine,
                72.0,
                100.0 + 14.0 * f64::from(rang),
                12.0,
                &arial,
            ));
        }
        caracteres.extend(ecrire("et la fin.", 72.0, 142.0, 12.0, &arial));
        // Puis, après un blanc, une formule courte : un autre paragraphe.
        caracteres.extend(ecrire("Cordialement,", 72.0, 180.0, 12.0, &arial));
        caracteres.extend(ecrire("Le service RH", 72.0, 194.0, 12.0, &arial));
        let document = document(&[page(caracteres)]);
        let paragraphes = &document.sections[0].paragraphes;
        assert_eq!(paragraphes.len(), 3, "{paragraphes:#?}");
        assert_eq!(
            texte(&paragraphes[0]),
            format!("{pleine} {pleine} {pleine} et la fin.")
        );
        assert_eq!(paragraphes[0].alignement, Alignement::Justifie);
        assert_eq!(paragraphes[0].interligne, 14.0);
        // Une ligne courte n'est pas continuée par la suivante, même proche.
        assert_eq!(texte(&paragraphes[1]), "Cordialement,");
        assert_eq!(texte(&paragraphes[2]), "Le service RH");
        // Le blanc entre les deux paragraphes est gardé : 180 - 142 = 38 points d'une ligne de
        // base à l'autre, soit 24 points de plus qu'un interligne.
        assert!(
            (paragraphes[1].espace_avant - 24.0).abs() < 0.5,
            "{:?}",
            paragraphes[1]
        );
        let section = &document.sections[0];
        assert_eq!(section.marge_gauche, 72.0);
        assert!((section.marge_haut - (100.0 - 0.8 * 14.0)).abs() < 0.01);
    }

    #[test]
    fn aligne_les_colonnes_d_un_tableau_par_des_tabulations() {
        let arial = police("Arial", false);
        let gras = police("Arial", true);
        let mut caracteres = ecrire("Salaire de base", 72.0, 300.0, 10.0, &arial);
        caracteres.extend(ecrire("151,67", 300.0, 300.0, 10.0, &arial));
        caracteres.extend(ecrire("1 820,00", 450.0, 300.0, 10.0, &gras));
        caracteres.extend(ecrire("Prime", 72.0, 312.0, 10.0, &arial));
        caracteres.extend(ecrire("100,00", 450.0, 312.0, 10.0, &gras));
        let document = document(&[page(caracteres)]);
        let paragraphes = &document.sections[0].paragraphes;
        assert_eq!(paragraphes.len(), 2);
        assert_eq!(texte(&paragraphes[0]), "Salaire de base\t151,67\t1 820,00");
        assert_eq!(paragraphes[0].taquets, [228.0, 378.0]);
        assert_eq!(texte(&paragraphes[1]), "Prime\t100,00");
        // Le montant garde son gras ; les lignes serrées du tableau gardent leur écart (12).
        assert!(matches!(&paragraphes[0].elements[4], Element::Texte(_, style) if style.gras));
        assert!(matches!(&paragraphes[0].elements[2], Element::Texte(_, style) if !style.gras));
        assert_eq!(paragraphes[0].interligne, 11.5);
        assert!(paragraphes[1].espace_avant < 1.0, "{:?}", paragraphes[1]);
    }

    #[test]
    fn centre_un_titre_et_garde_les_puces_a_part() {
        let titre = police("Arial", true);
        let arial = police("Arial", false);
        // Centré sur 297, le milieu des bords du texte (72 et 522) : 16 caractères de 8 points.
        let mut caracteres = ecrire("BULLETIN DE PAIE", 233.0, 80.0, 16.0, &titre);
        caracteres.extend(ecrire("Bords", 72.0, 120.0, 10.0, &arial));
        caracteres.extend(ecrire("droite", 492.0, 120.0, 10.0, &arial));
        let longue = "- un premier point assez long pour aller jusqu'au bord droit de la page ici";
        caracteres.extend(ecrire(longue, 72.0, 140.0, 12.0, &arial));
        caracteres.extend(ecrire("- un second point", 72.0, 154.0, 12.0, &arial));
        let document = document(&[page(caracteres)]);
        let paragraphes = &document.sections[0].paragraphes;
        assert_eq!(paragraphes[0].alignement, Alignement::Centre);
        assert_eq!(texte(&paragraphes[0]), "BULLETIN DE PAIE");
        assert!(
            matches!(&paragraphes[0].elements[0], Element::Texte(_, style) if style.gras && style.taille == 16.0)
        );
        assert_eq!(texte(&paragraphes[2]), longue);
        assert_eq!(texte(&paragraphes[3]), "- un second point");
    }

    #[test]
    fn reprend_les_images_a_leur_place_mais_pas_le_scan_d_un_texte_reconnu() {
        let image = |gauche: f64, largeur: f64, hauteur: f64, cle: u128| ImagePlacee {
            gauche,
            haut: 40.0,
            largeur,
            hauteur,
            octets: Arc::new(vec![1, 2, 3]),
            format: Format::Jpeg,
            cle,
        };
        let arial = police("Arial", false);
        let mut logo = page(ecrire("Courrier", 72.0, 200.0, 12.0, &arial));
        logo.images = vec![image(400.0, 120.0, 60.0, 7)];
        let mut scan = page(
            ecrire("Texte reconnu", 72.0, 200.0, 12.0, &arial)
                .into_iter()
                .map(|caractere| Caractere {
                    invisible: true,
                    ..caractere
                })
                .collect(),
        );
        scan.images = vec![image(0.0, 595.0, 842.0, 8)];
        // Le même logo sur une troisième page : une seule image dans le document.
        let mut encore = page(Vec::new());
        encore.images = vec![image(400.0, 120.0, 60.0, 7)];
        let document = document(&[logo, scan, encore]);
        assert_eq!(document.images.len(), 1);
        let posee = &document.sections[0].paragraphes[0].images;
        assert_eq!(posee.len(), 1);
        assert_eq!(
            (posee[0].gauche, posee[0].haut, posee[0].largeur),
            (400.0, 40.0, 120.0)
        );
        assert!(!posee[0].derriere);
        assert!(document.sections[1].paragraphes[0].images.is_empty());
        assert_eq!(texte(&document.sections[1].paragraphes[0]), "Texte reconnu");
        assert_eq!(document.sections[2].paragraphes[0].images[0].image, 0);
    }

    #[test]
    fn reprend_en_image_une_page_faite_de_dessins_et_les_traits_d_une_autre() {
        let dessin = Page {
            autres_traces: 12,
            ..page(Vec::new())
        };
        let vide = page(Vec::new());
        let arial = police("Arial", false);
        let tableau = Page {
            autres_traces: 30,
            ..page(ecrire("Rubrique", 72.0, 200.0, 10.0, &arial))
        };
        let document = mettre_en_page(
            &[dessin, vide, tableau],
            "plan",
            |_| Some(b"page".to_vec()),
            |_| Some(b"traits".to_vec()),
        );
        let posee = &document.sections[0].paragraphes[0].images[0];
        assert_eq!((posee.largeur, posee.hauteur), (595.0, 842.0));
        assert!(!posee.derriere);
        assert_eq!(&**document.images[posee.image].0, b"page");
        // Une page vraiment vide reste une page vide.
        assert!(document.sections[1].paragraphes.is_empty());
        // Les traits du tableau, derrière son texte, sur toute la page.
        let traits = &document.sections[2].paragraphes[0].images[0];
        assert!(traits.derriere);
        assert_eq!(&**document.images[traits.image].0, b"traits");
        assert_eq!(texte(&document.sections[2].paragraphes[0]), "Rubrique");
    }

    #[test]
    fn coupe_les_colonnes_serrees_d_un_tableau() {
        // Comme un tableau d'Excel aux colonnes étroites : « Jean-Bap » touche « Nkoulou », dont
        // le « M » déborde sur la colonne suivante (caché dans le PDF), et l'en-tête « Mobile »
        // n'est qu'à 0,8 taille de police de « Fonction ».
        let arial = police("Calibri", false);
        let mut caracteres = Vec::new();
        for (base, cellules) in [
            (
                100.0,
                vec![
                    ("Mat", 40.0),
                    ("Prénom", 80.0),
                    ("NOM", 120.0),
                    ("Fonction", 163.0),
                    ("Mobile", 211.0),
                ],
            ),
            (
                112.0,
                vec![
                    ("A001", 40.0),
                    ("Jean-Bap", 80.0),
                    ("Nkoulou M", 120.0),
                    ("Directeur", 163.0),
                ],
            ),
            (
                124.0,
                vec![
                    ("A002", 40.0),
                    ("Aïssa", 80.0),
                    ("DIALLO", 120.0),
                    ("Respons", 163.0),
                ],
            ),
            (
                136.0,
                vec![
                    ("A003", 40.0),
                    ("Éloïse", 80.0),
                    ("martin", 120.0),
                    ("Chargée", 163.0),
                ],
            ),
        ] {
            for (cellule, x) in cellules {
                caracteres.extend(ecrire(cellule, x, base, 10.0, &arial));
            }
        }
        let document = document(&[page(caracteres)]);
        let paragraphes = &document.sections[0].paragraphes;
        let textes: Vec<String> = paragraphes.iter().map(texte).collect();
        assert_eq!(
            textes,
            [
                "Mat\tPrénom\tNOM\tFonction\tMobile",
                "A001\tJean-Bap\tNkoulou\tDirecteur",
                "A002\tAïssa\tDIALLO\tRespons",
                "A003\tÉloïse\tmartin\tChargée"
            ]
        );
        assert_eq!(paragraphes[1].taquets, [40.0, 80.0, 123.0]);
    }

    #[test]
    fn ne_coupe_ni_les_mots_ni_les_montants_d_une_colonne_alignee_a_droite() {
        // Comme le récapitulatif de la paie : noms alignés à droite (fin à 300), montants aussi
        // (fin à 400). « Jean », « Marc » et « Paul » commencent par hasard à 280, où les autres
        // noms ont le milieu d'un mot ; les secondes moitiés des montants sont alignées, après
        // une simple espace.
        let arial = police("Arial", false);
        let mut caracteres = Vec::new();
        let mut base = 88.0;
        for (gauche, nom, montant) in [
            ("Matricule", "Nom", "Net"),
            ("00123", "NKOULOU MBARGA Jean", "389 250"),
            ("00124", "DIALLO Aïssatou", "512 000"),
            ("00131", "MOUKOKO Éloïse", "245 600"),
            ("00140", "Marc", "131 000"),
            ("00152", "OKEMBA Grâce", "456 780"),
            ("00160", "Paul", "124 500"),
        ] {
            let largeur = |texte: &str| texte.chars().count() as f64 * 5.0;
            caracteres.extend(ecrire(gauche, 40.0, base, 10.0, &arial));
            caracteres.extend(ecrire(nom, 300.0 - largeur(nom), base, 10.0, &arial));
            caracteres.extend(ecrire(
                montant,
                400.0 - largeur(montant),
                base,
                10.0,
                &arial,
            ));
            base += 12.0;
        }
        let document = document(&[page(caracteres)]);
        let textes: Vec<String> = document.sections[0].paragraphes.iter().map(texte).collect();
        assert_eq!(
            textes,
            [
                "Matricule\tNom\tNet",
                "00123\tNKOULOU MBARGA Jean\t389 250",
                "00124\tDIALLO Aïssatou\t512 000",
                "00131\tMOUKOKO Éloïse\t245 600",
                "00140\tMarc\t131 000",
                "00152\tOKEMBA Grâce\t456 780",
                "00160\tPaul\t124 500"
            ]
        );
    }

    #[test]
    fn retire_le_texte_et_les_images_pour_rendre_les_traces() {
        let mut document = lopdf::Document::load_mem(&pdf_herite(1, "x")).unwrap();
        let police = document.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
        });
        let image = document.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => 1, "Height" => 1,
                "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
            },
            vec![0],
        ));
        let page = document.get_pages()[&1];
        let contenu = document.add_object(Stream::new(
            dictionary! {},
            b"q 0 0 1 rg 72 600 200 20 re f Q BT /F1 12 Tf 1 0 0 1 72 700 Tm 1 0 0 rg (Texte) Tj ET q 100 0 0 50 72 500 cm /Im1 Do Q".to_vec(),
        ));
        let page = document.get_dictionary_mut(page).unwrap();
        page.set("Contents", contenu);
        page.set(
            "Resources",
            dictionary! {
                "Font" => dictionary! { "F1" => police },
                "XObject" => dictionary! { "Im1" => image },
            },
        );
        let mut octets = Vec::new();
        document.save_to(&mut octets).unwrap();
        let (allege, avec_traces) = sans_texte_ni_images(&octets, "t.pdf").unwrap();
        assert_eq!(avec_traces, HashSet::from([0]));
        let allege = lopdf::Document::load_mem(&allege).unwrap();
        let page = allege.get_pages()[&1];
        let operateurs: Vec<String> = Content::decode(&allege.get_page_content(page))
            .unwrap()
            .operations
            .into_iter()
            .map(|operation| operation.operator)
            .collect();
        // Le rectangle bleu et l'appel de l'image restent ; le texte part, sa couleur reste.
        assert_eq!(
            operateurs,
            ["q", "rg", "re", "f", "Q", "rg", "q", "cm", "Do", "Q"]
        );
        // L'image, elle, n'est plus qu'un formulaire vide.
        let image = allege.get_object(image).unwrap().as_stream().unwrap();
        assert_eq!(
            image.dict.get(b"Subtype").unwrap().as_name().unwrap(),
            b"Form"
        );
        // Sans aucun tracé, rien à rendre.
        assert!(sans_texte_ni_images(&pdf_herite(1, "x"), "x.pdf").is_none());
    }

    #[test]
    fn convertit_un_pdf_en_document_word() {
        let docx = convertir(Arc::new(pdf_herite(2, "Bulletin")), "Bulletins.pdf").unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(docx)).unwrap();
        let mut corps = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut corps)
            .unwrap();
        // Deux pages, deux sections ; Helvetica devient Arial.
        assert!(corps.contains(">Bulletin 1</w:t>"), "{corps}");
        assert!(corps.contains(">Bulletin 2</w:t>"));
        assert_eq!(corps.matches("<w:sectPr>").count(), 2);
        assert!(corps.contains(r#"w:ascii="Arial""#));
        assert!(convertir(Arc::new(b"pas un PDF".to_vec()), "x.pdf").is_err());
    }
}
