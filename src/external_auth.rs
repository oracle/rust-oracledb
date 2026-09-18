//-----------------------------------------------------------------------------
// Copyright (c) 2026, Oracle and/or its affiliates.
//
// This software is dual-licensed to you under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
// 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
// either license.
//
// If you elect to accept the software under the Apache License, Version 2.0,
// the following applies:
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//-----------------------------------------------------------------------------

//-----------------------------------------------------------------------------
// external_auth.rs
//
// Defines the structure used for external authentication as well as the
// calculation of the signature required for Oracle Cloud Infrastructure IAM
// token based authentication.
//-----------------------------------------------------------------------------

use base64ct::Encoding;
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::pem::PemObject;

use crate::error::Error;

// the format of the date that is included in the signed header
const IAM_DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";

/// Method used to authenticate to the database instead of supplying a user
/// name and a password. `Debug` is deliberately not implemented so that no
/// credential contained in it can be printed.
pub enum ExternalAuth {
    /// OAuth 2.0 access token.
    AccessToken(String),

    /// Oracle Cloud Infrastructure IAM database token, along with the PEM
    /// encoded RSA private key that is used to sign the request sent to the
    /// database.
    IamToken { token: String, private_key: String },
}

/// Returns the header that is signed for OCI IAM token based authentication.
/// The header identifies the service and the address of the database to which
/// the connection was made.
fn get_auth_header(service_name: &str, host_info: &str) -> String {
    format!(
        "date: {}\n(request-target): {}\nhost: {}",
        chrono::Utc::now().format(IAM_DATE_FORMAT),
        service_name,
        host_info
    )
}

/// Returns the signature of the given text in base64 encoding. The private
/// key is expected to be a PEM encoded RSA private key and the signature is
/// calculated using RSA PKCS#1 v1.5 with SHA-256, as required by OCI IAM.
fn get_signature(private_key: &[u8], text: &str) -> Result<String, Error> {
    let key = PrivateKeyDer::from_pem_slice(private_key)
        .map_err(|e| Error::iam_private_key_invalid(e.to_string()))?;
    let builder = rustls::ClientConfig::builder();
    let signing_key = builder
        .crypto_provider()
        .key_provider
        .load_private_key(key)
        .map_err(|e| Error::iam_private_key_invalid(e.to_string()))?;
    let signer = signing_key
        .choose_scheme(&[rustls::SignatureScheme::RSA_PKCS1_SHA256])
        .ok_or_else(|| {
            Error::iam_private_key_invalid(String::from(
                "an RSA private key is required",
            ))
        })?;
    let signature = signer
        .sign(text.as_bytes())
        .map_err(|e| Error::iam_private_key_invalid(e.to_string()))?;
    Ok(base64ct::Base64::encode_string(&signature))
}

/// Returns the header required for OCI IAM token based authentication along
/// with the signature of that header.
pub(crate) fn get_iam_auth(
    private_key: &[u8],
    service_name: &str,
    host_info: &str,
) -> Result<(String, String), Error> {
    let header = get_auth_header(service_name, host_info);
    let signature = get_signature(private_key, &header)?;
    Ok((header, signature))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies the layout of the header that is signed for OCI IAM token
    /// based authentication.
    #[test]
    fn auth_header() {
        let header = get_auth_header("my_service", "192.168.1.1:1521");
        let lines: Vec<&str> = header.split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("date: "), "{}", lines[0]);
        assert!(lines[0].ends_with(" GMT"), "{}", lines[0]);
        assert_eq!(lines[1], "(request-target): my_service");
        assert_eq!(lines[2], "host: 192.168.1.1:1521");
    }

    /// Verifies that a private key which cannot be used for signing is
    /// reported as such.
    #[test]
    fn invalid_private_key() {
        let err = get_signature(b"not a private key", "text")
            .expect_err("expected failure");
        assert!(matches!(
            err.kind(),
            crate::ErrorKind::IamPrivateKeyInvalid(_)
        ));
    }
}
