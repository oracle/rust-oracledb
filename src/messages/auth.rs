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

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

struct Pair {
    key: String,
    value: String,
    flags: u32,
}

impl Pair {
    fn new(key: &str, value: &str, flags: u32) -> Pair {
        Pair {
            key: String::from(key),
            value: String::from(value),
            flags,
        }
    }
}

fn decrypt_cbc(key: &[u8; 32], encrypted_text: &[u8], plain_text: &mut [u8]) {
    let iv = [0u8; 16];
    Aes256CbcDec::new(key.into(), &iv.into())
        .decrypt_padded_b2b::<NoPadding>(encrypted_text, plain_text)
        .unwrap();
}

fn encrypt_cbc(key: &[u8; 32], plain_text: &[u8], encrypted_text: &mut [u8]) {
    let iv = [0u8; 16];
    Aes256CbcEnc::new(key.into(), &iv.into())
        .encrypt_padded_b2b::<NoPadding>(plain_text, encrypted_text)
        .unwrap();
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
    combo_key: Option<[u8; 32]>,
    resend_needed: bool,
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
        combo_key: &[u8; 32],
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
    fn encrypt_passwords(
        &mut self,
        client: &mut Client,
        combo_key: &[u8; 32],
    ) {
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

    /// Generates the combo key used for encrypting the passwords sent to the
    /// server for validation.
    fn generate_combo_key(
        &mut self,
        session_key_part_a: &[u8],
        session_key_part_b: &[u8],
        combo_key: &mut [u8; 32],
    ) {
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
        let mut raw_temp_key: [u8; 64] = [0; 64];
        raw_temp_key[..32].copy_from_slice(session_key_part_b);
        raw_temp_key[32..].copy_from_slice(session_key_part_a);
        let temp_key_str = base16ct::upper::encode_string(&raw_temp_key);
        let temp_key = temp_key_str.as_bytes();
        get_derived_key(temp_key, &salt, iterations, combo_key);
    }

    /// Generates the "speedy" key used by the server for validation without
    /// requiring time consuming calculations.
    fn generate_speedy_key(
        &mut self,
        password_key: &[u8],
        combo_key: &[u8; 32],
    ) {
        let mut input_val = [0u8; 80];
        rand::rng().fill(&mut input_val[..16]);
        input_val[16..].copy_from_slice(password_key);
        let mut speedy_key = [0u8; 80];
        encrypt_cbc(combo_key, &input_val, &mut speedy_key);
        self.add_pair_binary("AUTH_PBKDF2_SPEEDY_KEY", &speedy_key, 0);
    }

    /// Generates the password verifier and the various keys required by the
    /// server for validation.
    fn generate_verifier(&mut self, client: &mut Client) {
        // create password hash
        let iterations: u32 = self
            .session_data
            .get("AUTH_PBKDF2_VGEN_COUNT")
            .unwrap()
            .parse()
            .unwrap();
        let verifier_data = base16ct::upper::decode_vec(
            self.session_data.get("AUTH_VFR_DATA").unwrap(),
        )
        .unwrap();
        let mut salt = verifier_data.clone();
        salt.extend(b"AUTH_PBKDF2_SPEEDY_KEY");
        let mut password_key: [u8; 64] = [0; 64];
        let password = client.config().get_password_bytes();
        get_derived_key(&password, &salt, iterations, &mut password_key);
        let mut hasher = sha2::Sha512::new();
        hasher.update(password_key);
        hasher.update(verifier_data);
        let password_hash: &[u8; 32] =
            &hasher.finalize()[..32].try_into().unwrap();

        // decrypt first half of session key
        let encoded_server_key = base16ct::upper::decode_vec(
            self.session_data.get("AUTH_SESSKEY").unwrap(),
        )
        .unwrap();
        let mut session_key_part_a = [0u8; 32];
        decrypt_cbc(
            password_hash,
            &encoded_server_key,
            &mut session_key_part_a,
        );

        // generate second half of session key
        let mut session_key_part_b = [0u8; 32];
        rand::rng().fill(&mut session_key_part_b);
        let mut session_key = vec![0; 32];
        encrypt_cbc(password_hash, &session_key_part_b, &mut session_key);
        self.add_pair_binary("AUTH_SESSKEY", &session_key, 1);

        // calculate combo key
        let mut combo_key = [0u8; 32];
        self.generate_combo_key(
            &session_key_part_a,
            &session_key_part_b,
            &mut combo_key,
        );

        // generate speedy key
        self.generate_speedy_key(&password_key, &combo_key);

        // encrypt password(s)
        self.encrypt_passwords(client, &combo_key);
        self.combo_key = Some(combo_key);
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
        }
    }

    /// Sets the combo key to use for performing authentication.
    pub(crate) fn set_combo_key(&mut self, combo_key: &[u8; 32]) {
        self.combo_key = Some(*combo_key);
    }

    /// Takes the combo key from the auth message so it can be stored
    /// elsewhere.
    pub(crate) fn take_combo_key(&mut self) -> Option<[u8; 32]> {
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
            let key = resp.read_str_with_double_length()?;
            let value = resp.read_str_with_double_length()?;
            resp.read_ub4()?; // flags
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
        } else if let Some(combo_key) = self.combo_key {
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
