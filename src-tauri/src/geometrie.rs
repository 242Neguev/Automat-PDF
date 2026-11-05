//! Géométrie des pages : matrices du PDF, rectangles, et repère de la page telle qu'elle s'affiche
//! (zone visible, rotation comprise). Partagée par la mise en page et les surimpressions.

use lopdf::{Dictionary, Document, Object};

/// Feuille A4 en portrait, en points.
pub const A4: (f64, f64) = (595.28, 841.89);

/// Transformation affine du PDF : (x, y) ↦ (a x + c y + e, b x + d y + f).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrice(pub [f64; 6]);

impl Matrice {
    pub const IDENTITE: Matrice = Matrice([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    pub fn translation(x: f64, y: f64) -> Self {
        Matrice([1.0, 0.0, 0.0, 1.0, x, y])
    }

    pub fn echelle(sx: f64, sy: f64) -> Self {
        Matrice([sx, 0.0, 0.0, sy, 0.0, 0.0])
    }

    /// Rotation d'un angle en degrés (sens inverse des aiguilles d'une montre, comme en PDF).
    pub fn rotation(degres: f64) -> Self {
        let (sinus, cosinus) = degres.to_radians().sin_cos();
        Matrice([cosinus, sinus, -sinus, cosinus, 0.0, 0.0])
    }

    /// Applique `self`, puis `ensuite`.
    pub fn puis(self, ensuite: Matrice) -> Matrice {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = ensuite.0;
        Matrice([
            a * a2 + b * c2,
            a * b2 + b * d2,
            c * a2 + d * c2,
            c * b2 + d * d2,
            e * a2 + f * c2 + e2,
            e * b2 + f * d2 + f2,
        ])
    }

    /// Transformation inverse (la matrice d'une page est toujours inversible).
    pub fn inverse(self) -> Matrice {
        let [a, b, c, d, e, f] = self.0;
        let det = a * d - b * c;
        Matrice([
            d / det,
            -b / det,
            -c / det,
            a / det,
            (c * f - d * e) / det,
            (b * e - a * f) / det,
        ])
    }

    pub fn point(self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }

    /// Opérateur `cm` du contenu PDF.
    pub fn operateur(self) -> String {
        let [a, b, c, d, e, f] = self.0;
        format!("{a:.5} {b:.5} {c:.5} {d:.5} {e:.3} {f:.3} cm")
    }

    /// Matrice écrite en six nombres (clé /Matrix d'un formulaire, opérandes de `cm`) ; `None`
    /// s'il n'y en a pas exactement six.
    pub fn de_nombres(valeurs: &[Object]) -> Option<Matrice> {
        let valeurs: Option<Vec<f64>> = valeurs
            .iter()
            .map(|valeur| valeur.as_float().ok().map(f64::from))
            .collect();
        <[f64; 6]>::try_from(valeurs?).ok().map(Matrice)
    }
}

/// Boîte englobante de points (x0, y0, x1, y1), une fois la matrice appliquée.
pub fn boite(matrice: Matrice, points: &[(f64, f64)]) -> [f64; 4] {
    points
        .iter()
        .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, &(x, y)| {
            let (x, y) = matrice.point(x, y);
            [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]
        })
}

/// Objet désigné par une référence, ou l'objet lui-même.
pub fn resoudre<'a>(document: &'a Document, objet: &'a Object) -> &'a Object {
    match objet {
        Object::Reference(id) => document.get_object(*id).unwrap_or(objet),
        autre => autre,
    }
}

/// Rectangle (x0, y0, x1, y1) normalisé ; `None` s'il n'est pas lisible.
pub fn rectangle(document: &Document, objet: &Object) -> Option<[f64; 4]> {
    let valeurs: Vec<f64> = resoudre(document, objet)
        .as_array()
        .ok()?
        .iter()
        .map(|valeur| resoudre(document, valeur).as_float().ok().map(f64::from))
        .collect::<Option<_>>()?;
    let [x0, y0, x1, y1] = <[f64; 4]>::try_from(valeurs).ok()?;
    Some([x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)])
}

/// Partie visible d'une page et sa rotation : de quoi passer de ce que l'on voit à l'écran aux
/// coordonnées de la page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cadre {
    pub boite: [f64; 4],
    /// 0, 90, 180 ou 270 degrés (sens horaire).
    pub rotation: i64,
}

impl Cadre {
    /// Cadre d'une page : sa zone de recadrage (dans les limites de la page) et sa rotation.
    pub fn de_page(document: &Document, page: &Dictionary) -> Cadre {
        let media = page
            .get(b"MediaBox")
            .ok()
            .and_then(|b| rectangle(document, b))
            .unwrap_or([0.0, 0.0, A4.0, A4.1]);
        let boite = match page
            .get(b"CropBox")
            .ok()
            .and_then(|b| rectangle(document, b))
        {
            Some([x0, y0, x1, y1]) => {
                let boite = [
                    x0.max(media[0]),
                    y0.max(media[1]),
                    x1.min(media[2]),
                    y1.min(media[3]),
                ];
                if boite[2] - boite[0] > 1.0 && boite[3] - boite[1] > 1.0 {
                    boite
                } else {
                    media
                }
            }
            None => media,
        };
        let rotation = page
            .get(b"Rotate")
            .map(|r| resoudre(document, r))
            .and_then(Object::as_i64)
            .unwrap_or(0)
            .rem_euclid(360)
            / 90
            * 90;
        Cadre { boite, rotation }
    }

    /// Taille telle qu'elle s'affiche (rotation comprise).
    pub fn taille_vue(&self) -> (f64, f64) {
        let [x0, y0, x1, y1] = self.boite;
        if self.rotation % 180 == 0 {
            (x1 - x0, y1 - y0)
        } else {
            (y1 - y0, x1 - x0)
        }
    }

    /// Des coordonnées de la page à celles de la page affichée : origine en bas à gauche de la
    /// partie visible, page redressée.
    pub fn vers_vue(&self) -> Matrice {
        let [x0, y0, x1, y1] = self.boite;
        let (largeur, hauteur) = (x1 - x0, y1 - y0);
        let redressement = match self.rotation {
            90 => Matrice([0.0, -1.0, 1.0, 0.0, 0.0, largeur]),
            180 => Matrice([-1.0, 0.0, 0.0, -1.0, largeur, hauteur]),
            270 => Matrice([0.0, 1.0, -1.0, 0.0, hauteur, 0.0]),
            _ => Matrice::IDENTITE,
        };
        Matrice::translation(-x0, -y0).puis(redressement)
    }

    /// Des coordonnées de la page affichée (origine en bas à gauche) à celles de la page : pour
    /// dessiner à l'endroit sur une page tournée.
    pub fn depuis_vue(&self) -> Matrice {
        self.vers_vue().inverse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proche(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn redresse_les_pages_tournees_dans_les_deux_sens() {
        for rotation in [0, 90, 180, 270] {
            let cadre = Cadre {
                boite: [10.0, 20.0, 410.0, 620.0],
                rotation,
            };
            let aller_retour = cadre.vers_vue().puis(cadre.depuis_vue());
            assert!(
                proche(aller_retour.point(123.0, 456.0), (123.0, 456.0)),
                "{rotation}"
            );
        }
        // Page tournée d'un quart de tour : le coin en haut à gauche de l'écran est le coin en bas à
        // gauche de la page d'origine ; celui en bas à gauche de l'écran, son coin en bas à droite.
        let cadre = Cadre {
            boite: [0.0, 0.0, 400.0, 600.0],
            rotation: 90,
        };
        assert_eq!(cadre.taille_vue(), (600.0, 400.0));
        assert!(proche(cadre.depuis_vue().point(0.0, 400.0), (0.0, 0.0)));
        assert!(proche(cadre.depuis_vue().point(0.0, 0.0), (400.0, 0.0)));
    }

    #[test]
    fn compose_et_inverse_les_matrices() {
        let m = Matrice::rotation(30.0)
            .puis(Matrice::echelle(2.0, 3.0))
            .puis(Matrice::translation(5.0, -7.0));
        assert!(proche(m.puis(m.inverse()).point(3.0, 4.0), (3.0, 4.0)));
        assert!(proche(Matrice::rotation(90.0).point(1.0, 0.0), (0.0, 1.0)));
    }
}
