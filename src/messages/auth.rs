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
// auth.rs
//
// Defines the structure used for sending and receiving the auth message.
// This is the fourth (and fifth) messages sent to the database while
// establishing a connection and can be combined with the protocol and data
// types messages when fast authentication is available.
// -----------------------------------------------------------------------------

use aes::cipher::block_padding::NoPadding;
use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};
use rand::RngExt;
use sha2::Digest;
use std::collections::HashMap;

use crate::client::Client;
use crate::constants;
use crate::error::Error;
use crate::messages::Message;
use crate::ora_version::OracleVersion;
use crate::response::Response;
use crate::write_buffer::WriteBuffer;

type Aes192CbcDec = cbc::Decryptor<aes::Aes192>;
type Aes192CbcEnc = cbc::Encryptor<aes::Aes192>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

struct Pair {
    key: String,
    value: String,
    flags: u32,
}

impl Pair {
    fn new(key: &str, value: &str, flags: u32) -> Pair {
        Pair { key: String::from(key), value: String::from(value), flags }
    }
}

fn decrypt_cbc(key: &[u8], encrypted_text: &[u8], plain_text: &mut [u8]) {
    let iv = [0u8; 16];
    if key.len() == 24 {
        Aes192CbcDec::new_from_slices(key, &iv)
            .unwrap()
            .decrypt_padded_b2b::<NoPadding>(
                encrypted_text,
                &mut plain_text[..encrypted_text.len()],
            )
            .unwrap();
    } else {
        Aes256CbcDec::new_from_slices(key, &iv)
            .unwrap()
            .decrypt_padded_b2b::<NoPadding>(encrypted_text, plain_text)
            .unwrap();
    }
}

fn encrypt_cbc(key: &[u8], plain_text: &[u8], encrypted_text: &mut [u8]) {
    let iv = [0u8; 16];
    if key.len() == 24 {
        Aes192CbcEnc::new_from_slices(key, &iv)
            .unwrap()
            .encrypt_padded_b2b::<NoPadding>(plain_text, encrypted_text)
            .unwrap();
    } else {
        Aes256CbcEnc::new_from_slices(key, &iv)
            .unwrap()
            .encrypt_padded_b2b::<NoPadding>(plain_text, encrypted_text)
            .unwrap();
    }
}

fn get_derived_key(
    key: &[u8],
    salt: &[u8],
    iterations: u32,
    derived_key: &mut [u8],
) {
    pbkdf2::pbkdf2_hmac::<sha2::Sha512>(key, salt, iterations, derived_key);
}

pub struct AuthMessage {
    pub session_data: HashMap<String, String>,
    pairs: Vec<Pair>,
    combo_key: Option<Vec<u8>>,
    resend_needed: bool,
    verifier_type: u32,
}

impl AuthMessage {
    /// Adds a key/value pair to the list of key/value pairs being sent to the
    /// database during authentication.
    fn add_pair(&mut self, key: &str, value: &str, flags: u32) {
        self.pairs.push(Pair::new(key, value, flags));
    }

    /// Adds a key/value pair to the list of key/value pairs being sent to the
    /// database during authentication, but hex encodes the binary value first.
    fn add_pair_binary(&mut self, key: &str, value: &[u8], flags: u32) {
        let str_value = base16ct::upper::encode_string(value);
        self.add_pair(key, &str_value, flags);
    }

    /// Encrypts a password and adds a key/value pair containing the encrypted
    /// password.
    fn encrypt_password(
        &mut self,
        key: &str,
        password: &[u8],
        combo_key: &[u8],
    ) {
        let pad_amount = 16 - password.len() % 16;
        let mut padded_input = vec![0u8; password.len() + pad_amount + 16];
        rand::rng().fill(&mut padded_input[..16]);
        padded_input[16..16 + password.len()].copy_from_slice(password);
        let offset = password.len() + 16;
        let pad_byte: u8 = pad_amount.try_into().unwrap();
        for i in 0..pad_amount {
            padded_input[offset + i] = pad_byte;
        }
        let mut output_val = vec![0u8; padded_input.len()];
        encrypt_cbc(combo_key, &padded_input, &mut output_val);
        self.add_pair_binary(key, &output_val[..], 0);
    }

    /// Encrypts the main password (and the new password, if applicable).
    fn encrypt_passwords(&mut self, client: &mut Client, combo_key: &[u8]) {
        self.encrypt_password(
            "AUTH_PASSWORD",
            &client.config().get_password_bytes(),
            combo_key,
        );
        if let Some(new_password) = client.config().get_new_password_bytes() {
            self.encrypt_password(
                "AUTH_NEWPASSWORD",
                &new_password,
                combo_key,
            );
        }
    }

    /// Generates the password verifier and the various keys required by the
    /// server for validation.
    fn generate_verifier(&mut self, client: &mut Client) {
        // create password hash; the 11g verifier uses SHA-1 (padded to a 24
        // byte AES-192 key), whereas the 12c verifier uses PBKDF2/SHA-512
        let mut password_key = [0u8; 64];
        let password_bytes = client.config().get_password_bytes();
        let verifier_data = base16ct::upper::decode_vec(
            self.session_data.get("AUTH_VFR_DATA").unwrap(),
        )
        .unwrap();
        let is_12c = self.verifier_type == constants::VERIFIER_TYPE_12C;
        let password_hash = if is_12c {
            let iterations: u32 = self
                .session_data
                .get("AUTH_PBKDF2_VGEN_COUNT")
                .unwrap()
                .parse()
                .unwrap();
            let mut salt = verifier_data.to_vec();
            salt.extend(b"AUTH_PBKDF2_SPEEDY_KEY");
            get_derived_key(
                &password_bytes,
                &salt,
                iterations,
                &mut password_key,
            );
            let mut hasher = sha2::Sha512::new();
            hasher.update(password_key);
            hasher.update(verifier_data);
            hasher.finalize()[..32].to_vec()
        } else {
            let mut hasher = sha1::Sha1::new();
            hasher.update(&password_bytes);
            hasher.update(verifier_data);
            let mut password_hash = hasher.finalize().to_vec();
            password_hash.extend([0u8; 4]);
            password_hash
        };

        // decrypt first half of session key
        let server_session_key = base16ct::upper::decode_vec(
            self.session_data.get("AUTH_SESSKEY").unwrap(),
        )
        .unwrap();
        let session_key_len = server_session_key.len();
        let mut session_key_part_a = vec![0u8; session_key_len];
        decrypt_cbc(
            &password_hash,
            &server_session_key,
            &mut session_key_part_a,
        );

        // generate second half of session key
        let mut session_key_part_b = vec![0u8; session_key_len];
        rand::rng().fill(&mut session_key_part_b[..]);
        let mut session_key = vec![0; session_key_len];
        encrypt_cbc(&password_hash, &session_key_part_b, &mut session_key);
        self.add_pair_binary("AUTH_SESSKEY", &session_key, 1);

        // generate combo key
        let combo_key = if session_key_len == 48 {
            let mut buf = [0u8; 24];
            for i in 16..40 {
                buf[i - 16] = session_key_part_a[i] ^ session_key_part_b[i];
            }
            let mut combo_key = md5::Md5::digest(&buf[..16]).to_vec();
            combo_key.extend(md5::Md5::digest(&buf[16..]));
            combo_key.truncate(24);
            combo_key
        } else {
            let key_len = if is_12c { 32 } else { 24 };
            let mut raw_temp_key = session_key_part_b[..key_len].to_vec();
            raw_temp_key.extend_from_slice(&session_key_part_a[..key_len]);
            let temp_key_str = base16ct::upper::encode_string(&raw_temp_key);
            let temp_key = temp_key_str.as_bytes();
            let mut combo_key = vec![0u8; 32];
            let iterations: u32 = self
                .session_data
                .get("AUTH_PBKDF2_SDER_COUNT")
                .unwrap()
                .parse()
                .unwrap();
            let salt = base16ct::upper::decode_vec(
                self.session_data.get("AUTH_PBKDF2_CSK_SALT").unwrap(),
            )
            .unwrap();
            get_derived_key(temp_key, &salt, iterations, &mut combo_key);
            combo_key.truncate(key_len);
            combo_key
        };

        // generate speedy key (12c only)
        if is_12c {
            let mut temp_speedy_key = [0u8; 80];
            rand::rng().fill(&mut temp_speedy_key[..16]);
            temp_speedy_key[16..].copy_from_slice(&password_key);
            let mut speedy_key = [0u8; 80];
            encrypt_cbc(&combo_key, &temp_speedy_key, &mut speedy_key);
            self.add_pair_binary("AUTH_PBKDF2_SPEEDY_KEY", &speedy_key, 0);
        }

        // encrypt passwords and store combo key for later use
        self.encrypt_passwords(client, &combo_key);
        self.combo_key = Some(combo_key);
    }

    /// Returns the alter session statement that is sent during the initial
    /// connection to the database. This includes the directive to change the
    /// session time zone to match the time zone in use by the system.
    fn get_alter_session_statement(&self) -> String {
        let tz_repr = if let Ok(value) = std::env::var("ORA_SDTZ")
            && !value.is_empty()
        {
            match value.to_uppercase().as_str() {
                "LOCAL" | "DBTIMEZONE" => value,
                _ => crate::enquote_literal(&value),
            }
        } else {
            let offset_seconds =
                chrono::Local::now().offset().local_minus_utc();
            let tz_hour = offset_seconds / 3600;
            let tz_minute = (offset_seconds - (tz_hour * 3600)) / 60;
            if tz_hour < 0 || tz_minute < 0 {
                format!("'-{:02}:{:02}'", tz_hour.abs(), tz_minute.abs())
            } else {
                format!("'+{:02}:{:02}'", tz_hour, tz_minute)
            }
        };
        format!("ALTER SESSION SET TIME_ZONE={}\0", tz_repr)
    }

    /// Returns the authorization mode to use when performing authentication.
    fn get_auth_mode(&self, client: &Client) -> u32 {
        let mut auth_mode: u32 = 0;
        if self.get_is_phase_one() {
            auth_mode |= constants::TTC_AUTH_MODE_LOGON;
        } else {
            auth_mode |= constants::TTC_AUTH_MODE_WITH_PASSWORD;
            if client.config().get_new_password_bytes().is_some() {
                auth_mode |= constants::TTC_AUTH_MODE_CHANGE_PASSWORD;
            } else {
                auth_mode |= constants::TTC_AUTH_MODE_LOGON;
            }
        }
        let user_auth_mode = client.config().auth_mode();
        if user_auth_mode & constants::AUTH_MODE_SYSASM != 0 {
            auth_mode |= constants::TTC_AUTH_MODE_SYSASM;
        }
        if user_auth_mode & constants::AUTH_MODE_SYSBKP != 0 {
            auth_mode |= constants::TTC_AUTH_MODE_SYSBKP;
        }
        if user_auth_mode & constants::AUTH_MODE_SYSDBA != 0 {
            auth_mode |= constants::TTC_AUTH_MODE_SYSDBA;
        }
        if user_auth_mode & constants::AUTH_MODE_SYSDGD != 0 {
            auth_mode |= constants::TTC_AUTH_MODE_SYSDGD;
        }
        if user_auth_mode & constants::AUTH_MODE_SYSKMT != 0 {
            auth_mode |= constants::TTC_AUTH_MODE_SYSKMT;
        }
        if user_auth_mode & constants::AUTH_MODE_SYSOPER != 0 {
            auth_mode |= constants::TTC_AUTH_MODE_SYSOPER;
        }
        if user_auth_mode & constants::AUTH_MODE_SYSRAC != 0 {
            auth_mode |= constants::TTC_AUTH_MODE_SYSRAC;
        }
        auth_mode
    }

    /// Returns whether phase one authorization is taking place or phase two
    /// authorization. Phase one is only used on logon.
    fn get_is_phase_one(&self) -> bool {
        self.combo_key.is_none() && self.session_data.is_empty()
    }

    /// Returns the TTC meessage type to use when sending the request to the
    /// database.
    fn get_ttc_message_type(&self) -> u8 {
        if self.get_is_phase_one() {
            constants::TTC_RPC_AUTH_PHASE_ONE
        } else {
            constants::TTC_RPC_AUTH_PHASE_TWO
        }
    }

    /// Writes a key/value pair to the network buffers.
    fn write_pair(&self, buf: &mut WriteBuffer, pair: &Pair) {
        let key_bytes = pair.key.as_bytes();
        let value_bytes = pair.value.as_bytes();
        buf.write_ub4(key_bytes.len().try_into().unwrap());
        buf.write_bytes_with_length(key_bytes);
        buf.write_ub4(value_bytes.len().try_into().unwrap());
        if !value_bytes.is_empty() {
            buf.write_bytes_with_length(value_bytes);
        }
        buf.write_ub4(pair.flags);
    }

    /// Returns the domain of the database.
    pub(crate) fn get_db_domain(&self) -> String {
        if let Some(value) = self.session_data.get("AUTH_SC_DB_DOMAIN") {
            value.to_string()
        } else {
            String::new()
        }
    }

    /// Returns the name of the database.
    pub(crate) fn get_db_name(&self) -> String {
        if let Some(value) = self.session_data.get("AUTH_SC_DBUNIQUE_NAME") {
            value.to_string()
        } else {
            String::new()
        }
    }

    /// Returns the instance name used to connect to the database.
    pub(crate) fn get_instance_name(&self) -> String {
        if let Some(value) = self.session_data.get("AUTH_INSTANCENAME") {
            value.to_string()
        } else {
            String::new()
        }
    }

    /// Returns the maximum number of bytes allowed to be used in identifiers.
    pub(crate) fn get_max_identifier_length(&self) -> usize {
        if let Some(value) = self.session_data.get("AUTH_MAX_IDEN_LENGTH") {
            value.parse::<usize>().unwrap()
        } else {
            30
        }
    }

    /// Returns the maximum number of open cursors allowed by the database.
    pub(crate) fn get_max_open_cursors(&self) -> usize {
        if let Some(value) = self.session_data.get("AUTH_MAX_OPEN_CURSORS") {
            value.parse::<usize>().unwrap()
        } else {
            0
        }
    }

    /// Returns the serial number associated with the connection to the
    /// database.
    pub(crate) fn get_serial_num(&self) -> usize {
        self.session_data
            .get("AUTH_SERIAL_NUM")
            .unwrap()
            .parse::<usize>()
            .unwrap()
    }

    /// Returns the server version.
    pub(crate) fn get_server_version(&self, client: &Client) -> OracleVersion {
        let full_version_num = self
            .session_data
            .get("AUTH_VERSION_NO")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        if client.supports_ttc_field_version(
            constants::TTC_FIELD_VERSION_18_1_EXT_1,
        ) {
            OracleVersion(
                (full_version_num >> 24) & 0xff,
                (full_version_num >> 16) & 0xff,
                (full_version_num >> 12) & 0x0f,
                (full_version_num >> 4) & 0xff,
                full_version_num & 0x0f,
            )
        } else {
            OracleVersion(
                (full_version_num >> 24) & 0xff,
                (full_version_num >> 20) & 0x0f,
                (full_version_num >> 12) & 0x0f,
                (full_version_num >> 8) & 0x0f,
                full_version_num & 0x0f,
            )
        }
    }

    /// Returns the service name used to connect to the database.
    pub(crate) fn get_service_name(&self) -> String {
        if let Some(value) = self.session_data.get("AUTH_SC_SERVICE_NAME") {
            value.to_string()
        } else {
            String::new()
        }
    }

    /// Returns the session id associated with the connection to the database.
    pub(crate) fn get_session_id(&self) -> usize {
        self.session_data
            .get("AUTH_SESSION_ID")
            .unwrap()
            .parse::<usize>()
            .unwrap()
    }

    /// Creates a new auth message.
    pub(crate) fn new() -> AuthMessage {
        AuthMessage {
            session_data: HashMap::new(),
            pairs: Vec::<Pair>::new(),
            combo_key: None,
            resend_needed: false,
            verifier_type: 0,
        }
    }

    /// Sets the combo key to use for performing authentication.
    pub(crate) fn set_combo_key(&mut self, combo_key: &[u8]) {
        self.combo_key = Some(combo_key.to_vec());
    }

    /// Takes the combo key from the auth message so it can be stored
    /// elsewhere.
    pub(crate) fn take_combo_key(&mut self) -> Option<Vec<u8>> {
        self.combo_key.take()
    }
}

impl Message for AuthMessage {
    fn deserialize_return_parameters(
        &mut self,
        _client: &Client,
        resp: &mut Response,
    ) -> Result<(), Error> {
        let num_params = resp.read_ub2()?;
        for _ in 0..num_params {
            let key = resp.read_utf8_with_double_length()?.to_string();
            let value = resp.read_utf8_with_double_length()?.to_string();
            let flags = resp.read_ub4()?;
            if key == "AUTH_VFR_DATA" {
                if !matches!(
                    flags,
                    constants::VERIFIER_TYPE_12C
                        | constants::VERIFIER_TYPE_11G_1
                        | constants::VERIFIER_TYPE_11G_2
                ) {
                    return Err(Error::not_implemented(format!(
                        "password verifier type 0x{flags:x}"
                    )));
                }
                self.verifier_type = flags;
            }
            self.session_data.insert(key, value);
        }
        Ok(())
    }

    fn pre_process(&mut self, client: &mut Client) {
        self.pairs.clear();
        if self.get_is_phase_one() {
            self.add_pair("AUTH_TERMINAL", client.config().terminal(), 0);
            self.add_pair("AUTH_PROGRAM_NM", client.config().program(), 0);
            self.add_pair("AUTH_MACHINE", client.config().machine(), 0);
            self.add_pair("AUTH_PID", &std::process::id().to_string(), 0);
            self.add_pair("AUTH_SID", client.config().osuser(), 0);
            self.resend_needed = true;
        } else if let Some(combo_key) = self.combo_key.clone() {
            self.encrypt_passwords(client, &combo_key);
        } else {
            self.generate_verifier(client);
            self.add_pair(
                "SESSION_CLIENT_CHARSET",
                &constants::CHARSET_ID_UTF8.to_string(),
                0,
            );
            self.add_pair(
                "SESSION_CLIENT_DRIVER_NAME",
                client.config().driver_name(),
                0,
            );
            let major =
                env!("CARGO_PKG_VERSION_MAJOR").parse::<usize>().unwrap();
            let minor =
                env!("CARGO_PKG_VERSION_MINOR").parse::<usize>().unwrap();
            let patch =
                env!("CARGO_PKG_VERSION_PATCH").parse::<usize>().unwrap();
            let full_version_num = major << 24 | minor << 20 | patch << 12;
            self.add_pair(
                "SESSION_CLIENT_VERSION",
                &full_version_num.to_string(),
                0,
            );
            self.add_pair(
                "AUTH_ALTER_SESSION",
                &self.get_alter_session_statement(),
                1,
            );
            if let Some(cclass) = client.config().cclass() {
                self.add_pair("AUTH_KPPL_CONN_CLASS", cclass, 0);
            }
            self.resend_needed = false;
        }
    }

    fn resend_needed(&self) -> bool {
        self.resend_needed
    }

    fn serialize(&self, client: &Client, buf: &mut WriteBuffer) {
        buf.write_function_header(client, self.get_ttc_message_type());
        let user_bytes = client.config().user().unwrap().as_bytes();
        buf.write_u8(1); // pointer (user)
        buf.write_ub4(user_bytes.len().try_into().unwrap());
        buf.write_ub4(self.get_auth_mode(client));
        buf.write_u8(1); // pointer (authivl)
        buf.write_ub4(self.pairs.len().try_into().unwrap());
        buf.write_u8(1); // pointer (authovl)
        buf.write_u8(1); // pointer (authovln)
        buf.write_bytes_with_length(user_bytes);
        for pair in self.pairs.iter() {
            self.write_pair(buf, pair);
        }
    }
}
