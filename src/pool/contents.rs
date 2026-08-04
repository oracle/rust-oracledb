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
// contents.rs
//
// Defines the structure holding the contents of the pool.
//-----------------------------------------------------------------------------

use std::collections::VecDeque;
use std::sync::mpsc;

use crate::config::PoolConfig;
use crate::connection::ConnImpl;
use crate::connection::ConnImplStatus;
use crate::error::Error;
use crate::pool::PoolManagerRequest;

pub(crate) struct PoolContents {
    config: PoolConfig,
    free_connections: VecDeque<ConnImpl>,
    connections_requiring_ping: VecDeque<ConnImpl>,
    connections_requiring_drop: Vec<ConnImpl>,
    last_create_error: Option<Error>,
    num_busy: usize,
    num_being_created: usize,
    num_being_pinged: usize,
    pending_requests: VecDeque<mpsc::Sender<Result<ConnImpl, Error>>>,
    pool_manager_request_channel: mpsc::Sender<PoolManagerRequest>,
}

pub(crate) enum PoolAcquireResponse {
    Connection(Result<ConnImpl, Error>),
    Wait(mpsc::Receiver<Result<ConnImpl, Error>>),
}

pub(crate) type PoolContentsRef =
    std::sync::Arc<std::sync::Mutex<PoolContents>>;

impl PoolContents {
    /// Returns a boolean indicating if the pool can grow.
    fn can_pool_grow(&mut self) -> bool {
        let num_in_pool = self.open_count();
        let min_connections = self.config.min_connections();
        self.num_being_created = if num_in_pool < min_connections {
            min_connections - num_in_pool
        } else {
            let max_connections = self.config.max_connections();
            let increment = self.config.connection_increment();
            (max_connections - num_in_pool).min(increment)
        };
        self.num_being_created > 0
    }

    /// Gets a connection from the pool that is capable of being returned
    /// immediately to the caller. This checks the connection to ensure that it
    /// is still healthy. If no connections are available and healthy, None is
    /// returned.
    fn get_connection(&mut self) -> Option<ConnImpl> {
        while let Some(conn_impl) = self.free_connections.pop_front() {
            match conn_impl.get_status(self.config.ping_interval()) {
                ConnImplStatus::RequiresClose => {
                    self.connections_requiring_drop.push(conn_impl);
                }
                ConnImplStatus::RequiresPing => {
                    self.connections_requiring_ping.push_back(conn_impl);
                }
                ConnImplStatus::Healthy => {
                    return Some(conn_impl);
                }
            }
        }
        None
    }

    /// Notify the manager that some work needs to be done. If a connection
    /// requires a ping before it can be returned, that message is sent. If
    /// no connections require a ping but there is room in the pool to grow,
    /// that message is sent. If neither of these actions are possible but a
    /// connection needs to be dropped that message is sent.
    fn notify_manager(&mut self) {
        if let Some(conn_impl) = self.connections_requiring_ping.pop_front() {
            self.num_being_pinged += 1;
            self.send_manager_request(PoolManagerRequest::PingConnection(
                conn_impl,
            ));
        } else if self.num_being_created == 0 && self.can_pool_grow() {
            self.send_manager_request(PoolManagerRequest::GrowPool);
        } else if let Some(conn_impl) = self.connections_requiring_drop.pop() {
            self.send_manager_request(PoolManagerRequest::DropConnection(
                conn_impl,
            ));
        }
    }

    /// Satisfy a pending request with the given connection, if possible. If
    /// there are no pending requests (or they have given up waiting), then the
    /// connection is added to the list of free connections.
    fn satisfy_request(&mut self, conn_impl_result: Result<ConnImpl, Error>) {
        if let Some(sender) = self.pending_requests.pop_front() {
            match sender.send(conn_impl_result) {
                Ok(_) => {
                    self.num_busy += 1;
                }
                Err(e) => {
                    self.satisfy_request(e.0);
                }
            }
        } else {
            match conn_impl_result {
                Ok(conn_impl) => {
                    self.free_connections.push_back(conn_impl);
                }
                Err(err) => {
                    self.last_create_error = Some(err);
                }
            }
        }
    }

    /// Satisfies any pending requests, if possible.
    fn satisfy_requests(&mut self) {
        while !self.free_connections.is_empty()
            && !self.pending_requests.is_empty()
        {
            if let Some(conn_impl) = self.get_connection() {
                self.satisfy_request(Ok(conn_impl));
            }
        }
    }

    /// Sends a request to the manager.
    fn send_manager_request(&self, request: PoolManagerRequest) {
        self.pool_manager_request_channel.send(request).unwrap();
    }

    /// Returns a connection from the pool if one is available.
    pub(super) fn acquire(&mut self) -> PoolAcquireResponse {
        if let Some(conn_impl) = self.get_connection() {
            self.num_busy += 1;
            return PoolAcquireResponse::Connection(Ok(conn_impl));
        }
        let (tx, rx) = mpsc::channel();
        self.pending_requests.push_back(tx);
        self.notify_manager();
        PoolAcquireResponse::Wait(rx)
    }

    /// Adds a new connection to the pool. This is called by the manager when
    /// the pool is growing.
    pub(super) fn add_new_connection(
        &mut self,
        conn_impl_result: Result<ConnImpl, Error>,
    ) {
        self.num_being_created -= 1;
        self.satisfy_request(conn_impl_result);
        self.notify_manager();
    }

    /// Adds a pinged connection back to the pool. This is called by the
    /// manager when a ping has completed. If no connection is supplied, the
    /// connection has been dropped because the ping failed.
    pub(super) fn add_pinged_connection(
        &mut self,
        conn_impl_opt: Option<ConnImpl>,
    ) {
        self.num_being_pinged -= 1;
        if let Some(conn_impl) = conn_impl_opt {
            self.satisfy_request(Ok(conn_impl));
        }
        self.notify_manager();
    }

    /// Returns the number of busy connections.
    pub(super) fn busy_count(&self) -> usize {
        self.num_busy
    }

    /// Closes all of the connections in the pool.
    pub(super) fn close(&mut self) -> Result<(), Error> {
        if self.num_busy > 0 {
            return Err(Error::pool_has_busy_connections());
        }
        self.pool_manager_request_channel
            .send(PoolManagerRequest::ClosePool)
            .unwrap();
        self.free_connections.clear();
        self.connections_requiring_ping.clear();
        self.connections_requiring_drop.clear();
        Ok(())
    }

    /// Creates a new structure containing the contents of the pool.
    pub(super) fn new(
        config: PoolConfig,
        request_channel: mpsc::Sender<PoolManagerRequest>,
    ) -> Self {
        let min_connections = config.min_connections();
        let max_connections = config.max_connections();
        if min_connections > 0 {
            request_channel.send(PoolManagerRequest::GrowPool).unwrap();
        }
        Self {
            config,
            free_connections: VecDeque::with_capacity(max_connections),
            connections_requiring_ping: VecDeque::new(),
            connections_requiring_drop: Vec::new(),
            last_create_error: None,
            num_busy: 0,
            num_being_created: min_connections,
            num_being_pinged: 0,
            pending_requests: VecDeque::new(),
            pool_manager_request_channel: request_channel,
        }
    }

    /// Returns the number of open connections.
    pub(super) fn open_count(&self) -> usize {
        self.free_connections.len() + self.num_being_pinged + self.num_busy
    }

    /// Returns a connection to the pool.
    pub(crate) fn return_connection(&mut self, conn_impl: ConnImpl) {
        self.num_busy -= 1;
        self.free_connections.push_front(conn_impl);
        self.satisfy_requests();
    }
}
