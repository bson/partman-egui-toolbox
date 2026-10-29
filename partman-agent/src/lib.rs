// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! AI agents in an egui app, as the pin planner has them: any agent
//! speaking the Agent Client Protocol (ACP), run as a child process; the
//! user's choice of agents, kept by the app (`AgentsDialog`); and the
//! agent panel (`AgentRun`): its replies, tool calls, plan and usage as
//! they come, its questions, and a send box for the user's messages, to
//! be shown in partman-sidebar's sidebar. A made-up agent (`fake`) shows
//! the panel without a real one.

pub mod acp;
mod dialog;
pub mod fake;
mod panel;
pub mod profiles;
pub mod servers;
mod task;

pub use dialog::{AgentsDialog, Closed};
pub use panel::{AgentRun, Colors, Entry, Grant, Saved, Setup};
pub use profiles::{AgentProfile, ModelServer};
