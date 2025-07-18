#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! `bevy_window` provides a platform-agnostic interface for windowing in Bevy.
//!
//! This crate contains types for window management and events,
//! used by windowing implementors such as `bevy_winit`.
//! The [`WindowPlugin`] sets up some global window-related parameters and
//! is part of the [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html).

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

use alloc::sync::Arc;

use bevy_platform::sync::Mutex;

mod event;
mod monitor;
mod raw_handle;
mod system;
mod system_cursor;
mod window;

pub use crate::raw_handle::*;

#[cfg(target_os = "android")]
pub use android_activity;

pub use event::*;
pub use monitor::*;
pub use system::*;
pub use system_cursor::*;
pub use window::*;

/// The windowing prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime, MonitorSelection,
        VideoModeSelection, Window, WindowMoved, WindowPlugin, WindowPosition,
        WindowResizeConstraints,
    };
}

use bevy_app::prelude::*;

impl Default for WindowPlugin {
    fn default() -> Self {
        WindowPlugin {
            primary_window: Some(Window::default()),
            primary_cursor_options: Some(CursorOptions::default()),
            exit_condition: ExitCondition::OnAllClosed,
            close_when_requested: true,
        }
    }
}

/// A [`Plugin`] that defines an interface for windowing support in Bevy.
pub struct WindowPlugin {
    /// Settings for the primary window.
    ///
    /// `Some(custom_window)` will spawn an entity with `custom_window` and [`PrimaryWindow`] as components.
    /// `None` will not spawn a primary window.
    ///
    /// Defaults to `Some(Window::default())`.
    ///
    /// Note that if there are no windows the App will exit (by default) due to
    /// [`exit_on_all_closed`].
    pub primary_window: Option<Window>,

    /// Settings for the cursor on the primary window.
    ///
    /// Defaults to `Some(CursorOptions::default())`.
    ///
    /// Has no effect if [`WindowPlugin::primary_window`] is `None`.
    pub primary_cursor_options: Option<CursorOptions>,

    /// Whether to exit the app when there are no open windows.
    ///
    /// If disabling this, ensure that you send the [`bevy_app::AppExit`]
    /// event when the app should exit. If this does not occur, you will
    /// create 'headless' processes (processes without windows), which may
    /// surprise your users. It is recommended to leave this setting to
    /// either [`ExitCondition::OnAllClosed`] or [`ExitCondition::OnPrimaryClosed`].
    ///
    /// [`ExitCondition::OnAllClosed`] will add [`exit_on_all_closed`] to [`Update`].
    /// [`ExitCondition::OnPrimaryClosed`] will add [`exit_on_primary_closed`] to [`Update`].
    pub exit_condition: ExitCondition,

    /// Whether to close windows when they are requested to be closed (i.e.
    /// when the close button is pressed).
    ///
    /// If true, this plugin will add [`close_when_requested`] to [`Update`].
    /// If this system (or a replacement) is not running, the close button will have no effect.
    /// This may surprise your users. It is recommended to leave this setting as `true`.
    pub close_when_requested: bool,
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        // User convenience events
        app.add_event::<WindowEvent>()
            .add_event::<WindowResized>()
            .add_event::<WindowCreated>()
            .add_event::<WindowClosing>()
            .add_event::<WindowClosed>()
            .add_event::<WindowCloseRequested>()
            .add_event::<WindowDestroyed>()
            .add_event::<RequestRedraw>()
            .add_event::<CursorMoved>()
            .add_event::<CursorEntered>()
            .add_event::<CursorLeft>()
            .add_event::<Ime>()
            .add_event::<WindowFocused>()
            .add_event::<WindowOccluded>()
            .add_event::<WindowScaleFactorChanged>()
            .add_event::<WindowBackendScaleFactorChanged>()
            .add_event::<FileDragAndDrop>()
            .add_event::<WindowMoved>()
            .add_event::<WindowThemeChanged>()
            .add_event::<AppLifecycle>();

        if let Some(primary_window) = &self.primary_window {
            let mut entity_commands = app.world_mut().spawn(primary_window.clone());
            entity_commands.insert((
                PrimaryWindow,
                RawHandleWrapperHolder(Arc::new(Mutex::new(None))),
            ));
            if let Some(primary_cursor_options) = &self.primary_cursor_options {
                entity_commands.insert(primary_cursor_options.clone());
            }
        }

        match self.exit_condition {
            ExitCondition::OnPrimaryClosed => {
                app.add_systems(PostUpdate, exit_on_primary_closed);
            }
            ExitCondition::OnAllClosed => {
                app.add_systems(PostUpdate, exit_on_all_closed);
            }
            ExitCondition::DontExit => {}
        }

        if self.close_when_requested {
            // Need to run before `exit_on_*` systems
            app.add_systems(Update, close_when_requested);
        }

        // Register event types
        #[cfg(feature = "bevy_reflect")]
        app.register_type::<WindowEvent>()
            .register_type::<WindowResized>()
            .register_type::<RequestRedraw>()
            .register_type::<WindowCreated>()
            .register_type::<WindowCloseRequested>()
            .register_type::<WindowClosing>()
            .register_type::<WindowClosed>()
            .register_type::<CursorMoved>()
            .register_type::<CursorEntered>()
            .register_type::<CursorLeft>()
            .register_type::<WindowFocused>()
            .register_type::<WindowOccluded>()
            .register_type::<WindowScaleFactorChanged>()
            .register_type::<WindowBackendScaleFactorChanged>()
            .register_type::<FileDragAndDrop>()
            .register_type::<WindowMoved>()
            .register_type::<WindowThemeChanged>()
            .register_type::<AppLifecycle>()
            .register_type::<Monitor>();

        // Register window descriptor and related types
        #[cfg(feature = "bevy_reflect")]
        app.register_type::<Window>()
            .register_type::<PrimaryWindow>()
            .register_type::<CursorOptions>();
    }
}

/// Defines the specific conditions the application should exit on
#[derive(Clone)]
pub enum ExitCondition {
    /// Close application when the primary window is closed
    ///
    /// The plugin will add [`exit_on_primary_closed`] to [`PostUpdate`].
    OnPrimaryClosed,
    /// Close application when all windows are closed
    ///
    /// The plugin will add [`exit_on_all_closed`] to [`PostUpdate`].
    OnAllClosed,
    /// Keep application running headless even after closing all windows
    ///
    /// If selecting this, ensure that you send the [`bevy_app::AppExit`]
    /// event when the app should exit. If this does not occur, you will
    /// create 'headless' processes (processes without windows), which may
    /// surprise your users.
    DontExit,
}

/// [`AndroidApp`] provides an interface to query the application state as well as monitor events
/// (for example lifecycle and input events).
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<android_activity::AndroidApp> =
    std::sync::OnceLock::new();
