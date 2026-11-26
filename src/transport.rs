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
// transport.rs
//
// Defines the structures and methods used for managing the transport used for
// sending and receiving messages.
//-----------------------------------------------------------------------------

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(windows)]
use std::os::windows::io::AsRawSocket;

use pkcs8::DecodePrivateKey;
use rustls::ClientConfig as TlsClientConfig;
use rustls::ClientConnection as TlsClientConnection;
use rustls::StreamOwned as TlsStream;
use rustls::crypto::CryptoProvider;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::pem::PemObject;
use rustls::sign::CertifiedKey;

use std::cmp::min;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;

use crate::config::Address;
use crate::config::Config;
use crate::constants;
use crate::error::Error;
use crate::packet::Packet;

enum LowLevelTransport {
    Tcp(TcpStream),
    Tls(Box<TlsStream<TlsClientConnection, TcpStream>>),
}

impl LowLevelTransport {
    /// Flushes all data to the low level transport.
    fn flush(&mut self) -> Result<(), Error> {
        match self {
            Self::Tcp(s) => Ok(s.flush()?),
            Self::Tls(s) => Ok(s.flush()?),
        }
    }

    /// Returns the read timeout associated with the low level transport.
    pub(crate) fn get_read_timeout(
        &self,
    ) -> Result<Option<std::time::Duration>, Error> {
        match self {
            Self::Tcp(s) => Ok(s.read_timeout()?),
            Self::Tls(s) => Ok(s.sock.read_timeout()?),
        }
    }

    /// Neogtiate TLS
    fn negotiate_tls(
        self,
        server_name: &str,
        config: &Config,
    ) -> Result<Self, Error> {
        let mut resolver = CustomClientCertResolver::new();
        if let Some(wallet_location) = config.wallet_location() {
            resolver.populate(
                wallet_location,
                config.get_wallet_password_bytes(),
            )?;
        }
        let tls_config = resolver.get_tls_config()?;
        let tls_server_name: rustls::pki_types::ServerName =
            server_name.to_string().try_into().unwrap();
        let conn = rustls::ClientConnection::new(
            Arc::new(tls_config),
            tls_server_name,
        )?;
        let stream = match self {
            Self::Tcp(stream) => stream,
            Self::Tls(orig_stream) => {
                let (_, stream) = orig_stream.into_parts();
                stream
            }
        };
        let tls_stream = rustls::StreamOwned::new(conn, stream);
        Ok(Self::Tls(Box::new(tls_stream)))
    }

    /// Reads bytes from the low level transport and returns the number of
    /// bytes read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        match self {
            Self::Tcp(s) => Ok(s.read(buf)?),
            Self::Tls(s) => Ok(s.read(buf)?),
        }
    }

    /// Sets the read and write timeouts for the low level transport.
    pub(crate) fn set_timeout(
        &self,
        duration: Option<std::time::Duration>,
    ) -> Result<(), Error> {
        let stream = match self {
            Self::Tcp(s) => s,
            Self::Tls(s) => &s.sock,
        };
        stream.set_read_timeout(duration)?;
        Ok(stream.set_write_timeout(duration)?)
    }

    /// Shuts down the low-level transport.
    fn shutdown(&mut self) -> Result<(), Error> {
        let stream = match self {
            Self::Tcp(s) => s,
            Self::Tls(s) => {
                s.conn.send_close_notify();
                s.flush()?;
                &mut s.sock
            }
        };
        let _ = stream.shutdown(std::net::Shutdown::Both);
        Ok(())
    }

    /// Writes bytes to the low level transport.
    fn write_all(&mut self, data: &[u8]) -> Result<(), Error> {
        match self {
            Self::Tcp(s) => Ok(s.write_all(data)?),
            Self::Tls(s) => Ok(s.write_all(data)?),
        }
    }
}

pub(crate) struct Transport {
    low_level_transport: Option<LowLevelTransport>,
    socket_num: String,
    max_packet_size: usize,
    read_buf: Vec<u8>,
    available_bytes: usize,
    used_bytes: usize,
    write_buf: Vec<u8>,
    full_packet_size: bool,
    print_packets: bool,
    op_num: usize,
}

// a custom client certificate resolver is required because the client
// certificates supplied by Oracle Database are v1 certificates and rustls will
// not accept anything less than v3 certificates
#[derive(Debug)]
struct CustomClientCertResolver {
    provider: Arc<CryptoProvider>,
    key: Option<Arc<CertifiedKey>>,
    root_store: Option<rustls::RootCertStore>,
}

// how many bytes should be included for each line of packet output
const BYTES_PER_LINE: usize = 8;

// PEM file header/footers
const PEM_ENCRYPTED_PRIVATE_KEY_HEADER: &str =
    "-----BEGIN ENCRYPTED PRIVATE KEY-----";
const PEM_ENCRYPTED_PRIVATE_KEY_FOOTER: &str =
    "-----END ENCRYPTED PRIVATE KEY-----";
const PEM_UNENCRYPTED_PRIVATE_KEY_HEADER: &str = "-----BEGIN PRIVATE KEY-----";

/// Returns the character to use for the given byte when displaying packet
/// data.
fn display_char(byte: u8) -> char {
    let ch = byte as char;
    if ch.is_ascii_alphanumeric() || ch.is_ascii_punctuation() {
        ch
    } else {
        '.'
    }
}

/// Returns a string representation of the socket number. Platform specific
/// methods are used for this purpose.
#[cfg(unix)]
fn get_socket_num(stream: &TcpStream) -> String {
    stream.as_raw_fd().to_string()
}

#[cfg(windows)]
fn get_socket_num(stream: &TcpStream) -> String {
    stream.as_raw_socket().to_string()
}

/// Prints packet data in a way convenient for debugging.
fn print_packet(operation: &str, data: &[u8]) {
    let mut hex_values = Vec::<String>::with_capacity(BYTES_PER_LINE);
    let mut display_values = Vec::<char>::with_capacity(BYTES_PER_LINE);
    let mut output_lines = Vec::<String>::new();
    let mut offset: usize = 0;
    output_lines.push(String::from(operation));
    while offset < data.len() {
        hex_values.clear();
        display_values.clear();
        let end_index = min(offset + BYTES_PER_LINE, data.len());
        let line_data = &data[offset..end_index];
        for byte in line_data.iter() {
            hex_values.push(format!("{:02X}", byte));
            display_values.push(display_char(*byte));
        }
        while hex_values.len() < BYTES_PER_LINE {
            hex_values.push(String::from("  "));
            display_values.push(' ');
        }
        let hex_data = hex_values.join(" ");
        let display_data = display_values.iter().collect::<String>();
        let line = format!("{:0>4} : {} |{}|", offset, hex_data, display_data);
        output_lines.push(line);
        offset += BYTES_PER_LINE;
    }
    println!("{}\n", output_lines.join("\n"));
}

impl Transport {
    /// Extracts a packet from the data that has been received from the
    /// database. If insufficient bytes have been read, None is returned which
    /// signals that the caller should wait for some more bytes to be received.
    fn extract_packet(&mut self) -> Option<usize> {
        // insufficient bytes to read the packet size
        if self.available_bytes <= 4 {
            return None;
        }

        // determine packet size and see if enough bytes are available to read
        // a full packet
        let packet_size = if self.full_packet_size {
            let buf = &self.read_buf[0..4];
            u32::from_be_bytes(buf.try_into().unwrap()) as usize
        } else {
            let buf = &self.read_buf[0..2];
            u16::from_be_bytes(buf.try_into().unwrap()) as usize
        };
        if self.available_bytes < packet_size {
            return None;
        }

        // all of the bytes are available so return the packet size
        self.used_bytes = packet_size;
        Some(packet_size)
    }

    /// Returns the op header used when displaying packet data.
    fn get_op_header(&self, op: &str) -> String {
        let formatted_date =
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.6f");

        format!(
            "{} {} [op {}] on socket {}",
            formatted_date, op, self.op_num, self.socket_num
        )
    }

    /// Attempts to read a packet from the stream, if it is connected. The
    /// number of bytes read is returned.
    fn read_packet(&mut self) -> Result<(), Error> {
        let buf = &mut self.read_buf[self.available_bytes..];
        let num_bytes = self
            .low_level_transport
            .as_mut()
            .ok_or_else(Error::not_connected)
            .and_then(|t| t.read(buf))?;
        if num_bytes == 0 {
            self.low_level_transport = None;
            Err(Error::dead_connection())
        } else {
            self.available_bytes += num_bytes;
            Ok(())
        }
    }

    /// Writes a packet to the stream, if the stream is currently connected.
    fn write_packet(&mut self) -> Result<(), Error> {
        if self.low_level_transport.is_none() {
            return Err(Error::not_connected());
        }
        let data = &self.write_buf[..];
        if self.print_packets {
            self.op_num += 1;
            let header = self.get_op_header("Sending packet");
            print_packet(&header, data);
        }
        if let Some(t) = self.low_level_transport.as_mut() {
            t.write_all(data)?;
        }
        Ok(())
    }

    /// Closes the TCP connection to the database.
    pub(crate) fn close(&mut self) -> Result<(), Error> {
        if self.print_packets {
            self.op_num += 1;
            let header = self.get_op_header("Disconnecting transport");
            println!("{}\n", header);
        }
        self.low_level_transport
            .take()
            .ok_or_else(Error::not_connected)
            .and_then(|mut t| t.shutdown())
    }

    /// Establishes a TCP connection to the database.
    pub(crate) fn connect(
        &mut self,
        stream: TcpStream,
        address: &Address,
        config: &Config,
    ) -> Result<(), Error> {
        stream.set_nodelay(true)?;
        stream.set_read_timeout(None)?;
        stream.set_write_timeout(None)?;
        self.socket_num = get_socket_num(&stream);
        self.low_level_transport = Some(LowLevelTransport::Tcp(stream));
        if address.protocol() == "tcps" {
            self.negotiate_tls(address.host(), config)?;
        }
        Ok(())
    }

    /// Returns the read timeout associated with the transport.
    pub(crate) fn get_read_timeout(
        &self,
    ) -> Result<Option<std::time::Duration>, Error> {
        self.low_level_transport
            .as_ref()
            .ok_or_else(Error::not_connected)
            .and_then(|t| t.get_read_timeout())
    }

    /// Negotiates TLS on the connection.
    pub(crate) fn negotiate_tls(
        &mut self,
        server_name: &str,
        config: &Config,
    ) -> Result<(), Error> {
        if self.print_packets {
            self.op_num += 1;
            let header = self.get_op_header("Negotiate TLS");
            println!("{}\n", header);
        }
        let low_level_transport = self.low_level_transport.take().unwrap();
        self.low_level_transport =
            Some(low_level_transport.negotiate_tls(server_name, config)?);
        Ok(())
    }

    /// Creates a new empty transport.
    pub(crate) fn new() -> Self {
        Self {
            low_level_transport: None,
            socket_num: String::new(),
            max_packet_size: 0,
            read_buf: Vec::new(),
            available_bytes: 0,
            used_bytes: 0,
            write_buf: Vec::new(),
            full_packet_size: false,
            print_packets: env::var_os("RSO_DEBUG_PACKETS").is_some(),
            op_num: 0,
        }
    }

    /// Reads data from the database and returns a single packet, or an error
    /// indicating that the connection is no longer valid.
    pub(crate) fn receive_packet(&mut self) -> Result<Packet<'_>, Error> {
        if self.available_bytes > 0 && self.used_bytes > 0 {
            if self.available_bytes > self.used_bytes {
                self.read_buf
                    .copy_within(self.used_bytes..self.available_bytes, 0);

                self.available_bytes -= self.used_bytes;
                self.used_bytes = 0;
            } else {
                self.available_bytes = 0;
            }
            self.used_bytes = 0;
        }
        loop {
            if let Some(packet_size) = self.extract_packet() {
                let packet_buf = &self.read_buf[0..packet_size];
                if self.print_packets {
                    self.op_num += 1;
                    let header = self.get_op_header("Receiving packet");
                    print_packet(&header, packet_buf);
                }
                return Ok(Packet::new(packet_buf));
            }
            self.read_packet()?;
        }
    }

    pub(crate) fn send_packets(
        &mut self,
        packet_type: u8,
        packet_flags: u8,
        data_flags: u16,
        mut data: &[u8],
    ) -> Result<(), Error> {
        let header_size = Packet::header_size(packet_type);
        let max_data_size = self.max_packet_size - header_size;
        loop {
            let packet_data_size = min(max_data_size, data.len());
            let packet_data = &data[..packet_data_size];
            data = &data[packet_data_size..];
            let packet_size = packet_data.len() + header_size;
            self.write_buf.clear();
            if self.full_packet_size {
                let value: u32 = packet_size.try_into().unwrap();
                self.write_buf.extend(value.to_be_bytes());
            } else {
                let value: u16 = packet_size.try_into().unwrap();
                self.write_buf.extend(value.to_be_bytes());
                self.write_buf.push(0);
                self.write_buf.push(0);
            }
            self.write_buf.push(packet_type);
            self.write_buf.push(packet_flags);
            self.write_buf.push(0);
            self.write_buf.push(0);
            if packet_type == constants::PACKET_TYPE_DATA {
                self.write_buf.extend(data_flags.to_be_bytes());
            }
            self.write_buf.extend(packet_data);
            self.write_packet()?;
            if data.is_empty() {
                break;
            }
        }
        self.low_level_transport
            .as_mut()
            .ok_or_else(Error::not_connected)
            .and_then(|t| t.flush())
    }

    /// Sets the maximum packet size for the transport since it may vary for
    /// each description that is specified in the connect string.
    pub(crate) fn set_max_packet_size(&mut self, size: usize) {
        self.read_buf.resize(size, 0);
        self.write_buf.clear();
        self.write_buf.reserve_exact(size);
        self.max_packet_size = size;
    }

    /// Sets the read and write timeouts for the transport.
    pub(crate) fn set_timeout(
        &self,
        duration: Option<std::time::Duration>,
    ) -> Result<(), Error> {
        self.low_level_transport
            .as_ref()
            .ok_or_else(Error::not_connected)
            .and_then(|t| t.set_timeout(duration))
    }

    /// Indicates that the full packet size should be used for all subsequent
    /// packets sent on the transport.
    pub(crate) fn set_full_packet_size(&mut self) {
        self.full_packet_size = true;
    }

    /// Returns whether this transport is currently wrapped in TLS.
    pub(crate) fn uses_tls(&self) -> bool {
        matches!(self.low_level_transport, Some(LowLevelTransport::Tls(_)))
    }
}

impl CustomClientCertResolver {
    /// Creates a new empty structure and returns it.
    fn new() -> Self {
        Self {
            provider: CryptoProvider::get_default().cloned().unwrap_or_else(
                || Arc::new(rustls::crypto::aws_lc_rs::default_provider()),
            ),
            key: None,
            root_store: Some(rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.into(),
            }),
        }
    }

    /// Returns the TLS configuration to use when connecting to the database.
    fn get_tls_config(mut self) -> Result<TlsClientConfig, Error> {
        let root_store = self.root_store.take().unwrap();
        Ok(TlsClientConfig::builder_with_provider(self.provider.clone())
            .with_safe_default_protocol_versions()?
            .with_root_certificates(root_store)
            .with_client_cert_resolver(Arc::new(self)))
    }

    /// Populates the certified key with the contents of the wallet defined in
    /// the configuration, if one was defined. If the wallet does not contain a
    /// private key, then the certificates that are contained in the wallet are
    /// returned directly and used as part of the certificate roots used to
    /// validate the server certificate instead.
    fn populate(
        &mut self,
        wallet_location: &str,
        wallet_password: Vec<u8>,
    ) -> Result<(), Error> {
        let file_name = Path::new(wallet_location).join("ewallet.pem");
        let contents = fs::read_to_string(&file_name).map_err(|e| {
            Error::wallet_missing(e, file_name.display().to_string())
        })?;

        // retrieve any certificates
        let mut certs: Vec<CertificateDer> = Vec::new();
        for result in CertificateDer::pem_slice_iter(contents.as_bytes()) {
            certs.push(result?);
        }

        // if an encrypted private key is found, the pkcs8 crate is
        // required to decrypt it using the wallet password
        let mut private_key: Option<PrivateKeyDer> = None;
        if let Some(start_pos) =
            contents.find(PEM_ENCRYPTED_PRIVATE_KEY_HEADER)
            && let Some(end_pos) =
                contents.find(PEM_ENCRYPTED_PRIVATE_KEY_FOOTER)
        {
            let part = &contents
                [start_pos..end_pos + PEM_ENCRYPTED_PRIVATE_KEY_FOOTER.len()];
            let (_label, doc) = pkcs8::SecretDocument::from_pem(part)
                .map_err(|e| {
                    Error::wallet_private_key_invalid(e.to_string())
                })?;
            let decrypted_key =
                pkcs8::SecretDocument::from_pkcs8_encrypted_der(
                    doc.as_bytes(),
                    wallet_password,
                )
                .map_err(|e| {
                    Error::wallet_password_missing_or_invalid(
                        file_name.display().to_string(),
                        e.to_string(),
                    )
                })?;
            private_key = Some(PrivateKeyDer::Pkcs8(
                decrypted_key.as_bytes().to_vec().into(),
            ));

        // for the unencrypted private key, the rustls crate can be used
        } else if contents.contains(PEM_UNENCRYPTED_PRIVATE_KEY_HEADER) {
            private_key = Some(
                PrivateKeyDer::from_pem_slice(contents.as_bytes()).map_err(
                    |e| Error::wallet_private_key_invalid(e.to_string()),
                )?,
            );
        }

        // if a private key was found, setup the certified key to pass to the
        // server
        if let Some(key) = private_key {
            let signing_key =
                self.provider.key_provider.load_private_key(key)?;
            self.key = Some(Arc::new(CertifiedKey::new(certs, signing_key)));
        } else {
            let root_store: &mut rustls::RootCertStore =
                self.root_store.as_mut().unwrap();
            for cert in &certs {
                root_store.add(cert.clone())?;
            }
        }

        Ok(())
    }
}

impl rustls::client::ResolvesClientCert for CustomClientCertResolver {
    fn resolve(
        &self,
        _acceptable_issuers: &[&[u8]],
        _sigschemes: &[rustls::SignatureScheme],
    ) -> Option<Arc<CertifiedKey>> {
        self.key.clone()
    }

    fn has_certs(&self) -> bool {
        self.key.is_some()
    }
}
