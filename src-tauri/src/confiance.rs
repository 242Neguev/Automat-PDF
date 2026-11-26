//! La confiance dans le certificat d'une signature numérique : le système juge s'il remonte à une
//! autorité qu'il reconnaît (magasin de certificats de Windows, trousseau de macOS), à la date de
//! la signature.
//!
//! Sans Internet : ni certificat intermédiaire téléchargé, ni révocation demandée en ligne. Un
//! PDF reçu ne peut donc pas savoir quand on l'ouvre (une adresse dans un certificat piégé), et le
//! verdict est le même hors ligne. Les certificats joints à la signature complètent la chaîne.

use serde::Serialize;

/// Ce que le système dit du certificat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Confiance {
    /// Il remonte à une autorité reconnue par le système.
    Reconnue,
    /// Il ne remonte à aucune autorité reconnue (certificat fait soi-même, autorité inconnue).
    NonReconnue,
    /// Il n'était pas valable à la date de la signature (expiré, ou pas encore valable).
    HorsValidite,
    /// Le système le sait révoqué.
    Revoquee,
    /// Pas de jugement (système qui ne sait pas le faire, certificat absent).
    Inconnue,
}

/// Juge le certificat `certificats[0]`, les suivants complétant sa chaîne, à la date `moment`
/// (secondes depuis 1970 ; sans date : maintenant).
pub fn juger(certificats: &[&[u8]], moment: Option<i64>) -> Confiance {
    if certificats.is_empty() {
        return Confiance::Inconnue;
    }
    systeme(certificats, moment)
}

#[cfg(target_os = "macos")]
fn systeme(certificats: &[&[u8]], moment: Option<i64>) -> Confiance {
    use core_foundation::date::CFDate;
    use security_framework::certificate::SecCertificate;
    use security_framework::policy::SecPolicy;
    use security_framework::trust::SecTrust;

    /// Écart entre l'origine des dates de macOS (1er janvier 2001) et celle d'Unix (1970).
    const DEPUIS_2001: f64 = 978_307_200.0;
    // Codes d'erreur de Security.framework (SecBase.h).
    const EXPIRE: isize = -67818;
    const PAS_ENCORE_VALABLE: isize = -67819;
    const REVOQUE: isize = -67820;

    let Ok(lus) = certificats
        .iter()
        .map(|der| SecCertificate::from_der(der))
        .collect::<Result<Vec<_>, _>>()
    else {
        return Confiance::NonReconnue;
    };
    let Ok(mut evaluation) = SecTrust::create_with_certificates(&lus, &[SecPolicy::create_x509()])
    else {
        return Confiance::Inconnue;
    };
    if evaluation.set_network_fetch_allowed(false).is_err() {
        return Confiance::Inconnue;
    }
    if let Some(moment) = moment {
        let date = CFDate::new(moment as f64 - DEPUIS_2001);
        if evaluation.set_trust_verify_date(&date).is_err() {
            return Confiance::Inconnue;
        }
    }
    match evaluation.evaluate_with_error() {
        Ok(()) => Confiance::Reconnue,
        Err(erreur) => match erreur.code() {
            EXPIRE | PAS_ENCORE_VALABLE => Confiance::HorsValidite,
            REVOQUE => Confiance::Revoquee,
            _ => Confiance::NonReconnue,
        },
    }
}

#[cfg(windows)]
fn systeme(certificats: &[&[u8]], moment: Option<i64>) -> Confiance {
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::Security::Cryptography::{
        CERT_CHAIN_CACHE_ONLY_URL_RETRIEVAL, CERT_CHAIN_CONTEXT, CERT_CHAIN_PARA,
        CERT_STORE_ADD_ALWAYS, CERT_STORE_PROV_MEMORY, CERT_TRUST_IS_EXPLICIT_DISTRUST,
        CERT_TRUST_IS_NOT_SIGNATURE_VALID, CERT_TRUST_IS_NOT_TIME_VALID,
        CERT_TRUST_IS_OFFLINE_REVOCATION, CERT_TRUST_IS_PARTIAL_CHAIN, CERT_TRUST_IS_REVOKED,
        CERT_TRUST_IS_UNTRUSTED_ROOT, CERT_TRUST_REVOCATION_STATUS_UNKNOWN,
        CertAddEncodedCertificateToStore, CertCloseStore, CertCreateCertificateContext,
        CertFreeCertificateChain, CertFreeCertificateContext, CertGetCertificateChain,
        CertOpenStore, PKCS_7_ASN_ENCODING, X509_ASN_ENCODING,
    };

    /// Écart entre l'origine des dates de Windows (1601) et celle d'Unix (1970), en secondes.
    const DEPUIS_1601: i64 = 11_644_473_600;
    let encodage = X509_ASN_ENCODING | PKCS_7_ASN_ENCODING;
    let Ok(taille) = u32::try_from(certificats[0].len()) else {
        return Confiance::Inconnue;
    };
    // SAFETY : chaque objet créé ici est libéré avant de sortir, et les pointeurs passés
    // désignent des données vivantes (certificats, paramètres, date) pendant chaque appel.
    unsafe {
        let contexte = CertCreateCertificateContext(encodage, certificats[0].as_ptr(), taille);
        if contexte.is_null() {
            return Confiance::NonReconnue;
        }
        // Les autres certificats joints à la signature, pour compléter la chaîne.
        let magasin = CertOpenStore(CERT_STORE_PROV_MEMORY, 0, 0, 0, null());
        if !magasin.is_null() {
            for der in &certificats[1..] {
                if let Ok(taille) = u32::try_from(der.len()) {
                    CertAddEncodedCertificateToStore(
                        magasin,
                        encodage,
                        der.as_ptr(),
                        taille,
                        CERT_STORE_ADD_ALWAYS,
                        null_mut(),
                    );
                }
            }
        }
        let date = moment.and_then(|secondes| {
            let centaines_de_ns = u64::try_from(secondes + DEPUIS_1601).ok()? * 10_000_000;
            Some(FILETIME {
                dwLowDateTime: centaines_de_ns as u32,
                dwHighDateTime: (centaines_de_ns >> 32) as u32,
            })
        });
        let mut parametres: CERT_CHAIN_PARA = std::mem::zeroed();
        parametres.cbSize = size_of::<CERT_CHAIN_PARA>() as u32;
        let mut chaine: *mut CERT_CHAIN_CONTEXT = null_mut();
        let construite = CertGetCertificateChain(
            null_mut(),
            contexte,
            date.as_ref().map_or(null(), |d| d as *const FILETIME),
            magasin,
            &parametres,
            CERT_CHAIN_CACHE_ONLY_URL_RETRIEVAL,
            null(),
            &mut chaine,
        );
        let confiance = if construite == 0 || chaine.is_null() {
            Confiance::Inconnue
        } else {
            // La révocation n'est pas demandée en ligne : son état inconnu n'est pas une erreur.
            let erreurs = (*chaine).TrustStatus.dwErrorStatus
                & !(CERT_TRUST_REVOCATION_STATUS_UNKNOWN | CERT_TRUST_IS_OFFLINE_REVOCATION);
            if erreurs & CERT_TRUST_IS_REVOKED != 0 {
                Confiance::Revoquee
            } else if erreurs
                & (CERT_TRUST_IS_UNTRUSTED_ROOT
                    | CERT_TRUST_IS_PARTIAL_CHAIN
                    | CERT_TRUST_IS_NOT_SIGNATURE_VALID
                    | CERT_TRUST_IS_EXPLICIT_DISTRUST)
                != 0
            {
                Confiance::NonReconnue
            } else if erreurs & CERT_TRUST_IS_NOT_TIME_VALID != 0 {
                Confiance::HorsValidite
            } else if erreurs == 0 {
                Confiance::Reconnue
            } else {
                Confiance::NonReconnue
            }
        };
        if !chaine.is_null() {
            CertFreeCertificateChain(chaine);
        }
        if !magasin.is_null() {
            CertCloseStore(magasin, 0);
        }
        CertFreeCertificateContext(contexte);
        confiance
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
fn systeme(_certificats: &[&[u8]], _moment: Option<i64>) -> Confiance {
    Confiance::Inconnue
}
