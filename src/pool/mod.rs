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
// mod.rs
//
// Main module for the pool submodule.
//-----------------------------------------------------------------------------

mod contents;
mod manager;

use std::sync::mpsc;
use std::thread;

use crate::config::PoolConfig;
use crate::connection::Connection;
use crate::error::Error;
use contents::PoolAcquireResponse;
use contents::PoolContents;
pub(crate) use contents::PoolContentsRef;
use manager::PoolManager;
use manager::PoolManagerRequest;

/// Represents a connection pool which manages connections. It is created by
/// calling [create_pool()](`crate::create_pool`).
pub struct Pool {
    contents_ref: PoolContentsRef,
    bg_task: Option<thread::JoinHandle<()>>,
}

impl Pool {
    /// Checks to see that the pool is open and returns an error if it is not.
    fn check_open(&self) -> Result<(), Error> {
        if self.bg_task.is_some() {
            Ok(())
        } else {
            Err(Error::pool_not_open())
        }
    }

    /// Creates a new pool and returns it.
    pub(crate) fn create(config: PoolConfig) -> Result<Self, Error> {
        config.validate()?;
        let mut actual_config = config;
        if actual_config.cclass().is_none() {
            let cclass = format!("RSO:{}", uuid::Uuid::new_v4());
            actual_config = actual_config.set_cclass(cclass);
        }
        let manager_config = actual_config.clone();
        let (tx, rx) = mpsc::channel();
        let contents = PoolContents::new(actual_config, tx);
        let contents_ref =
            std::sync::Arc::new(std::sync::Mutex::new(contents));
        let manager_contents_ref = contents_ref.clone();
        let bg_task = thread::spawn(move || {
            let mut manager =
                PoolManager::new(manager_contents_ref, rx, manager_config);
            manager.run();
        });
        Ok(Self {
            contents_ref,
            bg_task: Some(bg_task),
        })
    }

    /// Returns a connection from the pool.
    pub fn acquire(&self) -> Result<Connection, Error> {
        self.check_open()?;
        let resp = self.contents_ref.lock().unwrap().acquire();
        let conn_impl_result = match resp {
            PoolAcquireResponse::Connection(result) => result,
            PoolAcquireResponse::Wait(channel) => channel.recv().unwrap(),
        };
        conn_impl_result.map(|conn_impl| {
            Connection::create_pooled(conn_impl, &self.contents_ref)
        })
    }

    /// Returns the number of busy connections in the pool, or an error if the
    /// pool has been closed.
    pub fn busy_count(&self) -> Result<usize, Error> {
        self.check_open()?;
        Ok(self.contents_ref.lock().unwrap().busy_count())
    }

    /// Closes the pool and makes it unusable now instead of when the
    /// pool is dropped.
    pub fn close(&mut self) -> Result<(), Error> {
        self.check_open()?;
        self.contents_ref.lock().unwrap().close()?;
        if let Some(handle) = self.bg_task.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    /// Returns the number of open connections in the pool, or an error if the
    /// pool has been closed.
    pub fn open_count(&self) -> Result<usize, Error> {
        self.check_open()?;
        Ok(self.contents_ref.lock().unwrap().open_count())
    }
}

impl Drop for Pool {
    /// Called when the pool is being dropped. Since connections checked out of
    /// the pool have a reference to the pool the only time this will be called
    /// is when all connections have been checked into the pool. A simple close
    /// is sufficient in that case.
    fn drop(&mut self) {
        let _ = self.close();
    }
}
