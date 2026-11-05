//! Texte des pages, tel qu'on le lit : pour reconnaître à qui appartient chaque page d'un PDF de
//! bulletins de paie (matricule, nom). hayro donne chaque caractère avec sa position, y compris le
//! texte invisible des scans passés à la reconnaissance de caractères ; les mots, les colonnes et
//! les lignes sont reconstitués d'après les écarts, quel que soit l'ordre d'écriture du PDF.

use std::sync::Arc;

use hayro::hayro_interpret::font::Glyph;
use hayro::hayro_interpret::hayro_cmap::BfString;
use hayro::hayro_interpret::{
    BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, InterpreterCache,
    InterpreterSettings, Paint, PathDrawMode, SoftMask, TransformExt, interpret_page,
};
use hayro::vello_cpu::kurbo::{Affine, BezPath, Point, Rect};

use crate::erreur::Erreur;
use crate::rendu::ouvrir;

/// Un caractère lu, dans un repère tourné pour que son sens d'écriture aille vers la droite.
struct Caractere {
    texte: String,
    /// Sens d'écriture sur la page affichée, en quarts de tour.
    sens: u8,
    debut: f64,
    fin: f64,
    /// Position de la ligne (vers le bas).
    ligne: f64,
    taille: f64,
}

/// Appareil qui, au lieu de dessiner, relève les caractères.
#[derive(Default)]
struct Releve {
    caracteres: Vec<Caractere>,
}

impl<'a> Device<'a> for Releve {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'a>>) {}
    fn set_blend_mode(&mut self, _: BlendMode) {}
    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'a>, _: &PathDrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn draw_image(&mut self, _: Image<'a, '_>, _: Affine) {}
    fn pop_clip_path(&mut self) {}
    fn pop_transparency_group(&mut self) {}

    fn draw_glyph(
        &mut self,
        glyphe: &Glyph<'a>,
        transform: Affine,
        glyph_transform: Affine,
        _: &Paint<'a>,
        _: &GlyphDrawMode,
    ) {
        let texte = match glyphe.as_unicode() {
            Some(BfString::Char(caractere)) => caractere.to_string(),
            Some(BfString::String(texte)) => texte,
            None => return,
        };
        // Le glyphe compte 1 000 unités par cadratin ; avance inconnue : un demi-cadratin.
        let matrice = transform * glyph_transform;
        let avance = match glyphe {
            Glyph::Outline(contour) => contour.advance_width().map(f64::from),
            Glyph::Type3(_) => None,
        }
        .filter(|avance| *avance > 0.0)
        .unwrap_or(500.0);
        let origine = matrice * Point::ORIGIN;
        let arrivee = matrice * Point::new(avance, 0.0);
        let taille = (matrice * Point::new(0.0, 1000.0) - origine).hypot();
        if !taille.is_finite() || taille < 0.5 {
            return;
        }
        // Quart de tour le plus proche du sens d'écriture, puis rotation qui le ramène vers la
        // droite (repère de la page affichée : y vers le bas).
        let angle = (arrivee.y - origine.y).atan2(arrivee.x - origine.x);
        let sens = (angle / std::f64::consts::FRAC_PI_2)
            .round()
            .rem_euclid(4.0) as u8;
        let tourner = |p: Point| match sens {
            0 => (p.x, p.y),
            1 => (p.y, -p.x),
            2 => (-p.x, -p.y),
            _ => (-p.y, p.x),
        };
        let (debut, ligne) = tourner(origine);
        let (fin, _) = tourner(arrivee);
        self.caracteres.push(Caractere {
            texte,
            sens,
            debut,
            fin: fin.max(debut),
            ligne,
            taille,
        });
    }
}

/// Texte d'une page : une ligne par ligne de texte, une espace entre les mots, une tabulation
/// entre deux blocs éloignés (colonnes d'un tableau).
fn mettre_en_lignes(mut caracteres: Vec<Caractere>) -> String {
    caracteres.sort_by(|a, b| a.sens.cmp(&b.sens).then(a.ligne.total_cmp(&b.ligne)));
    let mut lignes: Vec<Vec<Caractere>> = Vec::new();
    for caractere in caracteres {
        match lignes.last_mut() {
            Some(ligne)
                if ligne[0].sens == caractere.sens
                    && (caractere.ligne - ligne[0].ligne).abs()
                        <= 0.4 * ligne[0].taille.min(caractere.taille) =>
            {
                ligne.push(caractere);
            }
            _ => lignes.push(vec![caractere]),
        }
    }
    let mut texte = String::new();
    for mut ligne in lignes {
        ligne.sort_by(|a, b| a.debut.total_cmp(&b.debut));
        let mut brut = String::new();
        let mut precedent: Option<&Caractere> = None;
        for caractere in &ligne {
            if let Some(avant) = precedent {
                // Un caractère redessiné au même endroit (faux gras, contour) ne compte qu'une fois.
                if caractere.texte == avant.texte
                    && (caractere.debut - avant.debut).abs() < 0.2 * caractere.taille
                {
                    continue;
                }
                let ecart = caractere.debut - avant.fin;
                let taille = avant.taille.max(caractere.taille);
                if ecart > 1.5 * taille {
                    brut.push('\t');
                } else if ecart > 0.2 * taille {
                    brut.push(' ');
                }
            }
            brut.push_str(&caractere.texte);
            precedent = Some(caractere);
        }
        // Une suite de blancs devient une espace, ou une tabulation si elle en contient une.
        let mut propre = String::new();
        let mut blancs: Option<bool> = None;
        for c in brut.chars() {
            if c.is_whitespace() {
                blancs = Some(blancs.unwrap_or(false) || c == '\t');
            } else {
                if let Some(tabulation) = blancs.take()
                    && !propre.is_empty()
                {
                    propre.push(if tabulation { '\t' } else { ' ' });
                }
                propre.push(c);
            }
        }
        if !propre.is_empty() {
            texte.push_str(&propre);
            texte.push('\n');
        }
    }
    texte
}

/// Texte de chaque page (vide pour une page numérisée sans texte).
pub fn textes(octets: Arc<Vec<u8>>, nom: &str) -> Result<Vec<String>, Erreur> {
    let pdf = ouvrir(octets, nom)?;
    let cache = InterpreterCache::new();
    let reglages = InterpreterSettings::default();
    Ok(pdf
        .pages()
        .iter()
        .map(|page| {
            let (largeur, hauteur) = page.render_dimensions();
            let mut contexte = Context::new(
                page.initial_transform(true).to_kurbo(),
                Rect::new(0.0, 0.0, f64::from(largeur), f64::from(hauteur)),
                &cache,
                page.xref(),
                reglages.clone(),
            );
            let mut releve = Releve::default();
            interpret_page(page, &mut contexte, &mut releve);
            mettre_en_lignes(releve.caracteres)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use lopdf::Stream;

    use super::*;
    use crate::essais::pdf_herite;
    use crate::pdf::charger;

    /// PDF d'une page A4 en Helvetica, dont le contenu est donné.
    fn pdf_contenu(contenu: &str) -> Vec<u8> {
        let mut document = charger(&pdf_herite(1, "x"), "p.pdf").unwrap();
        let flux = document.add_object(Stream::new(
            lopdf::Dictionary::new(),
            contenu.as_bytes().to_vec(),
        ));
        let page = document.get_pages()[&1];
        document
            .get_dictionary_mut(page)
            .unwrap()
            .set("Contents", flux);
        let mut octets = Vec::new();
        document.save_to(&mut octets).unwrap();
        octets
    }

    #[test]
    fn lit_le_texte_de_chaque_page() {
        let textes = textes(Arc::new(pdf_herite(2, "Confidentiel")), "c.pdf").unwrap();
        assert_eq!(textes, ["Confidentiel 1\n", "Confidentiel 2\n"]);
    }

    #[test]
    fn reconstitue_lignes_mots_et_colonnes_dans_l_ordre_de_lecture() {
        // Écrit dans le désordre : la valeur avant l'intitulé, la deuxième ligne avant la
        // première, chaque chiffre du matricule placé à part, un mot en faux gras (deux fois).
        let pdf = pdf_contenu(
            "BT /F1 12 Tf 1 0 0 1 300 700 Tm (DUPONT) Tj ET \
             BT /F1 12 Tf 1 0 0 1 72 700 Tm (Nom :) Tj ET \
             BT /F1 12 Tf 1 0 0 1 72 720 Tm (Matricule :) Tj ET \
             BT /F1 12 Tf 1 0 0 1 135 720 Tm (0) Tj 1 0 0 1 141.67 720 Tm (0) Tj \
             1 0 0 1 148.34 720 Tm (1) Tj 1 0 0 1 155.01 720 Tm (2) Tj ET \
             BT /F1 12 Tf 1 0 0 1 72 680 Tm (Net) Tj 1 0 0 1 72.3 680 Tm (Net) Tj ET",
        );
        let textes = textes(Arc::new(pdf), "b.pdf").unwrap();
        assert_eq!(textes, ["Matricule : 0012\nNom :\tDUPONT\nNet\n"]);
    }

    #[test]
    fn lit_le_texte_tourne_et_le_texte_invisible() {
        // Ligne tournée d'un quart de tour (marge), et texte invisible d'un scan (mode 3).
        let pdf = pdf_contenu(
            "BT /F1 10 Tf 0 1 -1 0 40 300 Tm (Bulletin confidentiel) Tj ET \
             BT 3 Tr /F1 12 Tf 1 0 0 1 72 720 Tm (Matricule A001) Tj ET",
        );
        let textes_lus = textes(Arc::new(pdf), "s.pdf").unwrap();
        assert!(textes_lus[0].contains("Matricule A001\n"), "{textes_lus:?}");
        assert!(
            textes_lus[0].contains("Bulletin confidentiel\n"),
            "{textes_lus:?}"
        );
        // Une page sans texte (numérisée) donne un texte vide.
        let vide = pdf_contenu("q 0 0 1 rg 0 0 100 100 re f Q");
        assert_eq!(textes(Arc::new(vide), "v.pdf").unwrap(), [""]);
    }
}
