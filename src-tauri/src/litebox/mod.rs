// Copyright (c) Microsoft Corporation. Licensed under the MIT license.
// Refactored for real OS-level containment by diegoakanottheoperator.

//! # LiteBox (OS-Level Sandboxing)
//!
//! Provides true OS-level process isolation and sandboxing capabilities.
//! This is a structural enforcement engine implementing Windows Sandbox/AppContainer
//! and Linux Namespace/Seccomp boundaries.

use crate::traits::SecuritySandbox;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

pub struct LiteBox {
    initialized: bool,
}

impl Default for LiteBox {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteBox {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    /// Explicitly initialize the sandbox constraints for the current platform.
    pub fn apply_sandbox(&mut self) -> Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        {
            log::info!("LiteBox: Applying Linux generic namespace/seccomp boundaries");
            linux::apply_linux_sandbox()?;
        }

        #[cfg(target_os = "windows")]
        {
            log::info!("LiteBox: Applying Windows AppContainer/Job Object boundaries");
            windows::apply_windows_sandbox()?;
        }

        self.initialized = true;
        Ok(())
    }
}

impl SecuritySandbox for LiteBox {
    fn initialize(&self) {
        log::info!("SecuritySandbox: LiteBox OS-Level Enforcement Online.");
    }

    fn get_fd_count(&self) -> usize {
        // OS-level limits instead of manual tracking
        #[cfg(target_os = "linux")]
        {
            linux::get_open_fd_count()
        }
        #[cfg(not(target_os = "linux"))]
        {
            0
        }
    }
}

// Fallback for macOS or unimplemented OS
#[allow(dead_code)]
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn apply_fallback_sandbox() -> Result<(), String> {
    log::warn!("LiteBox: No OS-level sandbox implemented for this platform. Running dangerously.");
    Ok(())
}
