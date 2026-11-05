//! Texte écrit par Automat-PDF sur les pages (numéros, tampons) : police standard Helvetica,
//! présente dans tous les lecteurs de PDF, en codage WinAnsi (accents du français compris). Les
//! largeurs servent à centrer ou à aligner le texte.

use lopdf::{Dictionary, dictionary};

/// Largeurs des caractères 32 à 126, en millièmes de la taille : Helvetica.
const LARGEURS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278,
    278, // espace à /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // chiffres
    278, 278, 584, 584, 584, 556, 1015, // : à @
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667,
    611, 722, 667, 944, 667, 667, 611, // A à Z
    278, 278, 278, 469, 556, 333, // [ à `
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500,
    278, 556, 500, 722, 500, 500, 500, // a à z
    334, 260, 334, 584, // { à ~
];

/// Largeurs des caractères 32 à 126 : Helvetica gras.
const LARGEURS_GRAS: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278,
    278, // espace à /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // chiffres
    333, 333, 584, 584, 584, 611, 975, // : à @
    722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667,
    611, 722, 667, 944, 667, 667, 611, // A à Z
    333, 278, 333, 584, 556, 333, // [ à `
    556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611, 611, 611, 389, 556,
    333, 611, 556, 778, 556, 556, 500, // a à z
    389, 280, 389, 584, // { à ~
];

/// Octet WinAnsi d'un caractère (« ? » s'il n'existe pas dans ce codage).
pub fn octet_winansi(c: char) -> u8 {
    match c {
        ' '..='~' | '\u{A0}'..='\u{FF}' => c as u8,
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8A,
        '‹' => 0x8B,
        'Œ' => 0x8C,
        'Ž' => 0x8E,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9A,
        '›' => 0x9B,
        'œ' => 0x9C,
        'ž' => 0x9E,
        'Ÿ' => 0x9F,
        _ => b'?',
    }
}

/// Lettre de base d'une lettre accentuée du latin (même largeur dans Helvetica).
fn sans_accent(c: char) -> char {
    match c {
        'À'..='Å' => 'A',
        'Ç' => 'C',
        'È'..='Ë' => 'E',
        'Ì'..='Ï' => 'I',
        'Ñ' => 'N',
        'Ò'..='Ö' | 'Ø' => 'O',
        'Ù'..='Ü' => 'U',
        'Ý' | 'Ÿ' => 'Y',
        'à'..='å' => 'a',
        'ç' => 'c',
        'è'..='ë' => 'e',
        'ì'..='ï' => 'i',
        'ñ' => 'n',
        'ò'..='ö' | 'ø' => 'o',
        'ù'..='ü' => 'u',
        'ý' | 'ÿ' => 'y',
        autre => autre,
    }
}

fn largeur_caractere(c: char, gras: bool) -> u16 {
    let c = sans_accent(c);
    let table = if gras { &LARGEURS_GRAS } else { &LARGEURS };
    match c {
        ' '..='~' => table[c as usize - 32],
        'Æ' | 'Œ' | '…' | '—' | '‰' => 1000,
        'æ' => 889,
        'œ' => 944,
        '‘' | '’' | '‚' => 222,
        '“' | '”' | '„' => 333,
        '°' => 400,
        _ => 556,
    }
}

/// Caractère d'un octet en codage WinAnsi.
fn caractere_winansi(octet: u8) -> char {
    const SPECIAUX: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž',
        '\u{8F}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}',
        'ž', 'Ÿ',
    ];
    match octet {
        0x80..=0x9F => SPECIAUX[usize::from(octet - 0x80)],
        autre => char::from(autre),
    }
}

/// Largeur (millièmes de la taille) d'un caractère d'une police standard sans largeurs déclarées
/// (Helvetica, Courier) ; `None` pour les autres (Times, Symbol…), dont on n'a pas les mesures.
pub fn largeur_standard(nom_police: &str, code: u8) -> Option<f64> {
    if nom_police.starts_with("Courier") {
        return Some(600.0);
    }
    if !(nom_police.starts_with("Helvetica") || nom_police.starts_with("Arial")) {
        return None;
    }
    let gras = nom_police.contains("Bold");
    Some(f64::from(largeur_caractere(caractere_winansi(code), gras)))
}

/// Largeur d'un texte en points, pour une taille de police donnée.
pub fn largeur(texte: &str, gras: bool, taille: f64) -> f64 {
    let milliemes: u32 = texte
        .chars()
        .map(|c| u32::from(largeur_caractere(c, gras)))
        .sum();
    f64::from(milliemes) * taille / 1000.0
}

/// Chaîne hexadécimale PDF du texte (codage WinAnsi), sans caractère à échapper.
pub fn chaine(texte: &str) -> String {
    let octets: String = texte
        .chars()
        .map(|c| format!("{:02X}", octet_winansi(c)))
        .collect();
    format!("<{octets}>")
}

/// Police Helvetica (ou Helvetica gras), codage WinAnsi.
pub fn police(gras: bool) -> Dictionary {
    dictionary! {
        "Type" => "Font", "Subtype" => "Type1",
        "BaseFont" => if gras { "Helvetica-Bold" } else { "Helvetica" },
        "Encoding" => "WinAnsiEncoding",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesure_et_code_les_textes_francais() {
        // P 667 + a 556 + g 556 + e 556 + espace 278 + 1 556 = 3 169 millièmes.
        assert!((largeur("Page 1", false, 10.0) - 31.69).abs() < 1e-9);
        // Accents : même largeur que la lettre de base.
        assert_eq!(largeur("PAYÉ", true, 10.0), largeur("PAYE", true, 10.0));
        assert_eq!(chaine("Été 2€"), "<C974E9203280>");
        assert_eq!(octet_winansi('漢'), b'?');
        assert_eq!(largeur_standard("Helvetica", b'P'), Some(667.0));
        assert_eq!(largeur_standard("Helvetica-Bold", 0xC9), Some(667.0));
        assert_eq!(largeur_standard("Courier-Oblique", b'i'), Some(600.0));
        assert_eq!(largeur_standard("Times-Roman", b'a'), None);
    }
}
