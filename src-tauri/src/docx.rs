//! Écriture d'un document Word (.docx) : une section par page, faite de paragraphes (texte mis en
//! forme, tabulations, retraits, espacements) et d'images posées à leur place sur la page. Le
//! fichier est une archive ZIP de fichiers XML (Office Open XML), lue par Word comme par
//! LibreOffice.

use std::fmt::Write as _;
use std::io::{Cursor, Write as _};
use std::sync::Arc;

use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::erreur::Erreur;

/// Format d'une image du document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
}

impl Format {
    fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpeg",
        }
    }
}

/// Mise en forme d'un morceau de texte.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub famille: String,
    /// Taille, en points.
    pub taille: f64,
    pub gras: bool,
    pub italique: bool,
    pub couleur: [u8; 3],
}

/// Ce que contient un paragraphe.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Texte(String, Style),
    Tabulation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignement {
    #[default]
    Gauche,
    Centre,
    Droite,
    Justifie,
}

/// Une image posée sur la page ; positions et tailles en points, depuis le coin haut gauche.
#[derive(Debug, Clone, PartialEq)]
pub struct ImagePosee {
    /// Rang de l'image dans `Document::images`.
    pub image: usize,
    pub gauche: f64,
    pub haut: f64,
    pub largeur: f64,
    pub hauteur: f64,
    /// Derrière le texte (fond de page) plutôt que devant.
    pub derriere: bool,
}

/// Un paragraphe ; longueurs en points.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Paragraphe {
    pub alignement: Alignement,
    pub retrait_gauche: f64,
    /// Décalage de la première ligne par rapport au retrait gauche (négatif : vers la gauche).
    pub premiere_ligne: f64,
    pub retrait_droit: f64,
    pub espace_avant: f64,
    /// Hauteur exacte de chaque ligne.
    pub interligne: f64,
    /// Taquets de tabulation alignés à gauche, depuis la marge gauche.
    pub taquets: Vec<f64>,
    pub elements: Vec<Element>,
    /// Images posées sur la page de ce paragraphe.
    pub images: Vec<ImagePosee>,
}

/// Une section : une page du PDF, son format et ses marges (points).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Section {
    pub largeur: f64,
    pub hauteur: f64,
    pub marge_haut: f64,
    pub marge_droite: f64,
    pub marge_bas: f64,
    pub marge_gauche: f64,
    pub paragraphes: Vec<Paragraphe>,
}

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub titre: String,
    pub sections: Vec<Section>,
    /// Images du document, référencées par leur rang.
    pub images: Vec<(Arc<Vec<u8>>, Format)>,
}

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const RELATIONS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const TYPE_RELATION: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const EN_TETE: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;
/// Plus grand côté de page accepté par Word (22 pouces).
const COTE_MAX: f64 = 1584.0;

fn twips(points: f64) -> i64 {
    (points * 20.0).round() as i64
}

fn emu(points: f64) -> i64 {
    (points * 12700.0).round() as i64
}

/// Texte sûr dans un fichier XML : caractères réservés échappés, caractères de contrôle retirés.
fn echapper(texte: &str) -> String {
    let mut sur = String::with_capacity(texte.len());
    for c in texte.chars() {
        match c {
            '&' => sur.push_str("&amp;"),
            '<' => sur.push_str("&lt;"),
            '>' => sur.push_str("&gt;"),
            '"' => sur.push_str("&quot;"),
            '\t' | '\n' | '\r' => sur.push(' '),
            '\u{0}'..='\u{1f}' | '\u{fffe}' | '\u{ffff}' => {}
            _ => sur.push(c),
        }
    }
    sur
}

fn ecrire_texte(xml: &mut String, texte: &str, style: &Style) {
    let famille = echapper(&style.famille);
    let _ = write!(
        xml,
        r#"<w:r><w:rPr><w:rFonts w:ascii="{famille}" w:hAnsi="{famille}" w:cs="{famille}"/>"#
    );
    if style.gras {
        xml.push_str("<w:b/><w:bCs/>");
    }
    if style.italique {
        xml.push_str("<w:i/><w:iCs/>");
    }
    let [rouge, vert, bleu] = style.couleur;
    if style.couleur != [0, 0, 0] {
        let _ = write!(xml, r#"<w:color w:val="{rouge:02X}{vert:02X}{bleu:02X}"/>"#);
    }
    let demi_points = (style.taille * 2.0).round().clamp(2.0, 3276.0) as i64;
    let _ = write!(
        xml,
        r#"<w:sz w:val="{demi_points}"/><w:szCs w:val="{demi_points}"/></w:rPr><w:t xml:space="preserve">{}</w:t></w:r>"#,
        echapper(texte)
    );
}

/// Image ancrée à la page, sans habillage : elle garde sa place, quel que soit le texte.
fn ecrire_image(xml: &mut String, image: &ImagePosee, numero: usize) {
    let (x, y) = (emu(image.gauche), emu(image.haut));
    let (cx, cy) = (emu(image.largeur).max(1), emu(image.hauteur).max(1));
    let derriere = u8::from(image.derriere);
    let plan = 251_658_240 + numero;
    let _ = write!(
        xml,
        r#"<w:r><w:drawing><wp:anchor distT="0" distB="0" distL="0" distR="0" simplePos="0" relativeHeight="{plan}" behindDoc="{derriere}" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/><wp:positionH relativeFrom="page"><wp:posOffset>{x}</wp:posOffset></wp:positionH><wp:positionV relativeFrom="page"><wp:posOffset>{y}</wp:posOffset></wp:positionV><wp:extent cx="{cx}" cy="{cy}"/><wp:effectExtent l="0" t="0" r="0" b="0"/><wp:wrapNone/><wp:docPr id="{numero}" name="Image {numero}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="{numero}" name="Image {numero}"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rIdImage{rang}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:anchor></w:drawing></w:r>"#,
        rang = image.image + 1
    );
}

/// Format et marges d'une section, rendus acceptables par Word (page de 22 pouces au plus, place
/// laissée au texte entre les marges).
fn ecrire_section(xml: &mut String, section: &Section) {
    let largeur = section.largeur.clamp(72.0, COTE_MAX);
    let hauteur = section.hauteur.clamp(72.0, COTE_MAX);
    let place = |debut: f64, fin: f64, cote: f64| {
        let (debut, fin) = (debut.max(0.0), fin.max(0.0));
        let exces = (debut + fin - (cote - 36.0)).max(0.0);
        (debut - exces / 2.0, fin - exces / 2.0)
    };
    let (gauche, droite) = place(section.marge_gauche, section.marge_droite, largeur);
    let (haut, bas) = place(section.marge_haut, section.marge_bas, hauteur);
    let paysage = if largeur > hauteur {
        r#" w:orient="landscape""#
    } else {
        ""
    };
    let _ = write!(
        xml,
        r#"<w:sectPr><w:type w:val="nextPage"/><w:pgSz w:w="{}" w:h="{}"{paysage}/><w:pgMar w:top="{}" w:right="{}" w:bottom="{}" w:left="{}" w:header="0" w:footer="0" w:gutter="0"/></w:sectPr>"#,
        twips(largeur),
        twips(hauteur),
        twips(haut),
        twips(droite),
        twips(bas),
        twips(gauche)
    );
}

fn ecrire_paragraphe(
    xml: &mut String,
    paragraphe: &Paragraphe,
    section: Option<&Section>,
    numero: &mut usize,
) {
    xml.push_str("<w:p><w:pPr>");
    let mut taquets: Vec<i64> = paragraphe
        .taquets
        .iter()
        .map(|&taquet| twips(taquet).max(0))
        .collect();
    taquets.sort_unstable();
    taquets.dedup();
    if !taquets.is_empty() {
        xml.push_str("<w:tabs>");
        // Word n'en garde que 64 par paragraphe.
        for taquet in taquets.iter().take(64) {
            let _ = write!(xml, r#"<w:tab w:val="left" w:pos="{taquet}"/>"#);
        }
        xml.push_str("</w:tabs>");
    }
    let _ = write!(
        xml,
        r#"<w:spacing w:before="{}" w:after="0" w:line="{}" w:lineRule="exact"/>"#,
        twips(paragraphe.espace_avant).max(0),
        twips(paragraphe.interligne).max(20)
    );
    let premiere = twips(paragraphe.premiere_ligne);
    let decalage = if premiere < 0 {
        format!(r#" w:hanging="{}""#, -premiere)
    } else {
        format!(r#" w:firstLine="{premiere}""#)
    };
    let _ = write!(
        xml,
        r#"<w:ind w:left="{}" w:right="{}"{decalage}/>"#,
        twips(paragraphe.retrait_gauche),
        twips(paragraphe.retrait_droit)
    );
    let alignement = match paragraphe.alignement {
        Alignement::Gauche => "left",
        Alignement::Centre => "center",
        Alignement::Droite => "right",
        Alignement::Justifie => "both",
    };
    let _ = write!(xml, r#"<w:jc w:val="{alignement}"/>"#);
    if let Some(section) = section {
        ecrire_section(xml, section);
    }
    xml.push_str("</w:pPr>");
    for image in &paragraphe.images {
        *numero += 1;
        ecrire_image(xml, image, *numero);
    }
    for element in &paragraphe.elements {
        match element {
            Element::Texte(texte, style) => ecrire_texte(xml, texte, style),
            Element::Tabulation => xml.push_str("<w:r><w:tab/></w:r>"),
        }
    }
    xml.push_str("</w:p>");
}

/// Le corps du document : les paragraphes de chaque section, les propriétés de chacune dans son
/// dernier paragraphe (celles de la dernière, à la fin du corps).
fn document_xml(document: &Document) -> String {
    let mut xml = format!(
        r#"{EN_TETE}<w:document xmlns:w="{W}" xmlns:r="{TYPE_RELATION}" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>"#
    );
    let mut numero = 0;
    let vide = Paragraphe {
        interligne: 12.0,
        ..Paragraphe::default()
    };
    for (rang, section) in document.sections.iter().enumerate() {
        let derniere_section = rang + 1 == document.sections.len();
        let paragraphes: Vec<&Paragraphe> = if section.paragraphes.is_empty() {
            vec![&vide]
        } else {
            section.paragraphes.iter().collect()
        };
        for (position, paragraphe) in paragraphes.iter().enumerate() {
            let fin_de_section = position + 1 == paragraphes.len() && !derniere_section;
            ecrire_paragraphe(
                &mut xml,
                paragraphe,
                fin_de_section.then_some(section),
                &mut numero,
            );
        }
        if derniere_section {
            ecrire_section(&mut xml, section);
        }
    }
    if document.sections.is_empty() {
        ecrire_paragraphe(&mut xml, &vide, None, &mut numero);
    }
    xml.push_str("</w:body></w:document>");
    xml
}

fn types_xml() -> String {
    let principal = "application/vnd.openxmlformats-officedocument.wordprocessingml";
    format!(
        r#"{EN_TETE}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="jpeg" ContentType="image/jpeg"/><Override PartName="/word/document.xml" ContentType="{principal}.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="{principal}.styles+xml"/><Override PartName="/word/settings.xml" ContentType="{principal}.settings+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>"#
    )
}

fn relations_paquet() -> String {
    format!(
        r#"{EN_TETE}<Relationships xmlns="{RELATIONS}"><Relationship Id="rId1" Type="{TYPE_RELATION}/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="{TYPE_RELATION}/extended-properties" Target="docProps/app.xml"/></Relationships>"#
    )
}

fn relations_document(document: &Document) -> String {
    let mut xml = format!(
        r#"{EN_TETE}<Relationships xmlns="{RELATIONS}"><Relationship Id="rIdStyles" Type="{TYPE_RELATION}/styles" Target="styles.xml"/><Relationship Id="rIdSettings" Type="{TYPE_RELATION}/settings" Target="settings.xml"/>"#
    );
    for (rang, (_, format)) in document.images.iter().enumerate() {
        let _ = write!(
            xml,
            r#"<Relationship Id="rIdImage{n}" Type="{TYPE_RELATION}/image" Target="media/image{n}.{}"/>"#,
            format.extension(),
            n = rang + 1
        );
    }
    xml.push_str("</Relationships>");
    xml
}

/// Styles : Arial 11 par défaut, aucun espacement automatique, ni lignes veuves et orphelines
/// (elles déplaceraient des lignes d'une page à l'autre).
fn styles_xml() -> String {
    format!(
        r#"{EN_TETE}<w:styles xmlns:w="{W}"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial" w:eastAsia="Arial" w:cs="Arial"/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="0" w:line="240" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/><w:pPr><w:widowControl w:val="0"/></w:pPr></w:style></w:styles>"#
    )
}

/// Réglages : document au format de Word 2013 et suivants (pas de « mode de compatibilité »).
fn reglages_xml() -> String {
    format!(
        r#"{EN_TETE}<w:settings xmlns:w="{W}"><w:defaultTabStop w:val="709"/><w:compat><w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/></w:compat></w:settings>"#
    )
}

fn proprietes_xml(titre: &str) -> String {
    format!(
        r#"{EN_TETE}<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{}</dc:title></cp:coreProperties>"#,
        echapper(titre)
    )
}

fn application_xml() -> String {
    format!(
        r#"{EN_TETE}<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Automat-PDF</Application></Properties>"#
    )
}

/// Le document Word, prêt à être enregistré.
pub fn ecrire(document: &Document) -> Result<Vec<u8>, Erreur> {
    let echec = |erreur: zip::result::ZipError| {
        Erreur::simple(format!("Le document Word n'a pas pu être écrit : {erreur}"))
    };
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let compresse = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    // Les images sont déjà compressées : elles sont rangées telles quelles.
    let telles_quelles = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let parties = [
        ("[Content_Types].xml", types_xml()),
        ("_rels/.rels", relations_paquet()),
        ("docProps/core.xml", proprietes_xml(&document.titre)),
        ("docProps/app.xml", application_xml()),
        ("word/document.xml", document_xml(document)),
        ("word/_rels/document.xml.rels", relations_document(document)),
        ("word/styles.xml", styles_xml()),
        ("word/settings.xml", reglages_xml()),
    ];
    for (nom, contenu) in parties {
        archive.start_file(nom, compresse).map_err(echec)?;
        archive
            .write_all(contenu.as_bytes())
            .map_err(|erreur| echec(erreur.into()))?;
    }
    for (rang, (octets, format)) in document.images.iter().enumerate() {
        let nom = format!("word/media/image{}.{}", rang + 1, format.extension());
        archive.start_file(nom, telles_quelles).map_err(echec)?;
        archive
            .write_all(octets)
            .map_err(|erreur| echec(erreur.into()))?;
    }
    Ok(archive.finish().map_err(echec)?.into_inner())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    fn style(gras: bool) -> Style {
        Style {
            famille: "Arial".into(),
            taille: 11.0,
            gras,
            italique: false,
            couleur: [0, 0, 0],
        }
    }

    fn lire_octets(archive: &[u8], nom: &str) -> Vec<u8> {
        let mut zip = zip::ZipArchive::new(Cursor::new(archive)).unwrap();
        let mut contenu = Vec::new();
        zip.by_name(nom).unwrap().read_to_end(&mut contenu).unwrap();
        contenu
    }

    fn lire(archive: &[u8], nom: &str) -> String {
        String::from_utf8(lire_octets(archive, nom)).unwrap()
    }

    #[test]
    fn ecrit_un_document_lisible_par_word() {
        let paragraphe = Paragraphe {
            alignement: Alignement::Centre,
            retrait_gauche: 36.0,
            premiere_ligne: -18.0,
            espace_avant: 6.0,
            interligne: 13.5,
            taquets: vec![300.0, 150.0, 150.0],
            elements: vec![
                Element::Texte("Matricule :".into(), style(true)),
                Element::Tabulation,
                Element::Texte("0012 & Fils <Été>".into(), style(false)),
            ],
            images: vec![ImagePosee {
                image: 0,
                gauche: 40.0,
                haut: 30.0,
                largeur: 100.0,
                hauteur: 50.0,
                derriere: false,
            }],
            ..Paragraphe::default()
        };
        let a4 = Section {
            largeur: 595.0,
            hauteur: 842.0,
            marge_haut: 40.0,
            marge_droite: 50.0,
            marge_bas: 20.0,
            marge_gauche: 60.0,
            paragraphes: vec![paragraphe],
        };
        let paysage = Section {
            largeur: 842.0,
            hauteur: 595.0,
            ..Section::default()
        };
        let document = Document {
            titre: "Bulletins d'octobre".into(),
            sections: vec![a4, paysage],
            images: vec![(Arc::new(b"\x89PNG essai".to_vec()), Format::Png)],
        };
        let archive = ecrire(&document).unwrap();
        let corps = lire(&archive, "word/document.xml");
        // Texte échappé, gras, tabulation, taquets triés et sans doublon, retraits en vingtièmes
        // de point.
        assert!(corps.contains(r#"<w:b/><w:bCs/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr><w:t xml:space="preserve">Matricule :</w:t>"#), "{corps}");
        assert!(corps.contains("<w:r><w:tab/></w:r>"));
        assert!(corps.contains("0012 &amp; Fils &lt;Été&gt;"));
        assert!(corps.contains(r#"<w:tabs><w:tab w:val="left" w:pos="3000"/><w:tab w:val="left" w:pos="6000"/></w:tabs>"#));
        assert!(corps.contains(r#"<w:spacing w:before="120" w:after="0" w:line="270" w:lineRule="exact"/><w:ind w:left="720" w:right="0" w:hanging="360"/><w:jc w:val="center"/>"#));
        // La première page dans son dernier paragraphe, la seconde (paysage) à la fin du corps.
        assert!(corps.contains(r#"<w:pgSz w:w="11900" w:h="16840"/><w:pgMar w:top="800" w:right="1000" w:bottom="400" w:left="1200""#));
        assert!(corps.ends_with(r#"<w:pgSz w:w="16840" w:h="11900" w:orient="landscape"/><w:pgMar w:top="0" w:right="0" w:bottom="0" w:left="0" w:header="0" w:footer="0" w:gutter="0"/></w:sectPr></w:body></w:document>"#), "{corps}");
        // L'image, ancrée à la page à 40 × 30 points, renvoie à son fichier.
        assert!(corps.contains(r#"<wp:posOffset>508000</wp:posOffset></wp:positionH><wp:positionV relativeFrom="page"><wp:posOffset>381000</wp:posOffset>"#));
        assert!(corps.contains(r#"<a:blip r:embed="rIdImage1"/>"#));
        assert!(lire(&archive, "word/_rels/document.xml.rels").contains(r#"Id="rIdImage1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png""#));
        assert_eq!(
            lire_octets(&archive, "word/media/image1.png"),
            b"\x89PNG essai"
        );
        assert!(
            lire(&archive, "docProps/core.xml")
                .contains("<dc:title>Bulletins d'octobre</dc:title>")
        );
        assert!(lire(&archive, "[Content_Types].xml").contains(r#"PartName="/word/document.xml""#));
    }

    #[test]
    fn retire_les_caracteres_interdits_en_xml() {
        assert_eq!(echapper("a\u{0}b\u{1b}c\td\"e"), "abc d&quot;e");
    }

    #[test]
    fn garde_de_la_place_au_texte_entre_les_marges() {
        let mut xml = String::new();
        ecrire_section(
            &mut xml,
            &Section {
                largeur: 200.0,
                hauteur: 3000.0,
                marge_gauche: 150.0,
                marge_droite: 150.0,
                ..Section::default()
            },
        );
        // Page ramenée à 22 pouces de haut ; marges de 150 points réduites à 82 pour laisser 36
        // points au texte sur 200.
        assert!(xml.contains(r#"w:h="31680""#), "{xml}");
        assert!(
            xml.contains(r#"w:right="1640""#) && xml.contains(r#"w:left="1640""#),
            "{xml}"
        );
    }
}
