// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Work off the UI thread, as Partman's client does it: each task runs on
//! its own thread and hands its result back through a channel, waking the
//! UI when done.

use std::sync::mpsc::{self, Receiver, TryRecvError};

pub struct Task<T> {
    rx: Receiver<T>,
}

pub enum Poll<T> {
    Pending,
    Ready(T),
    /// The work panicked; without this the UI would wait for it forever
    Lost,
}

impl<T: Send + 'static> Task<T> {
    pub fn spawn(ctx: &egui::Context, f: impl FnOnce() -> T + Send + 'static) -> Task<T> {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            // The receiver is gone if the screen that wanted this went away
            let _ = tx.send(f());
            ctx.request_repaint();
        });
        Task { rx }
    }

    pub fn poll(&self) -> Poll<T> {
        match self.rx.try_recv() {
            Ok(v) => Poll::Ready(v),
            Err(TryRecvError::Empty) => Poll::Pending,
            Err(TryRecvError::Disconnected) => Poll::Lost,
        }
    }
}
