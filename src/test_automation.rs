//! Socket driven UI automation backed by .ui widget ids and glib reflection.
//!
//! Commands are read from `CTRL_PANEL_AUTOMATION_SOCKET`. Widgets are resolved by
//! traversing the active window's widget tree and querying every node with
//! `gtk_widget_get_template_child()` against that node's own type (so `ids`
//! bound by any composite template in the hierarchy — `ControlPanelGuiWindow`,
//! `Settings`, `UpdatePage`, ... — are found wherever they sit), then acted
//! upon through glib reflection (`emit` on signals, `set_property`).
//!
//! Verbs:
//! - `click <id>`                    emit the widget's `clicked` signal (buttons, toggles)
//! - `set-active <id> <true|false>`  set the `active` property (toggles, switches)
//! - `set-text <id> <value>`         set the `text` property (entries)
//! - `select <id> <index>`           set the `selected` property (drop-downs)
//! - `activate <action>`             activate a `gio::Action`
//! - `set-locale-timezone <locale> <tz>` `SettingsAction` emitter (kept as-is)
//! - `get <id> <property>`           gets a `property` as string
//!
//! Any other token that matches a registered application action is activated;
//! unknown commands are only logged.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;

use anyhow::{Context, bail, ensure};
use gtk::glib;
use gtk::glib::prelude::*;
use gtk::prelude::*;
use log::{debug, error, info, warn};

use crate::application::ControlPanelGuiApplication;
use crate::settings_action::SettingsAction;

pub fn setup(app: &ControlPanelGuiApplication) {
    let Some(socket_path) = std::env::var_os("CTRL_PANEL_AUTOMATION_SOCKET") else {
        debug!("CTRL_PANEL_AUTOMATION_SOCKET not set, skipping test automation");
        return;
    };
    let socket_path = std::path::PathBuf::from(socket_path);
    info!(
        "Binding test automation socket at {}",
        socket_path.display()
    );
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)
        .unwrap_or_else(|err| panic!("failed to bind test automation socket: {err}"));
    info!("Test automation listener bound, spawning threads");
    let (action_tx, action_rx) = async_channel::unbounded();

    std::thread::spawn(move || {
        debug!("Test automation listener thread started");
        for mut stream in listener.incoming().filter_map(Result::ok) {
            let mut reader = BufReader::new(&mut stream);
            let mut action = String::new();

            match reader.read_line(&mut action) {
                Ok(0) => {}
                Ok(_) => {
                    let command = action.trim().to_string();
                    if command.is_empty() {
                        warn!("Test automation received an empty command");
                        let _ = writeln!(stream, "ERR empty action");
                        continue;
                    }
                    let (resp_tx, resp_rx) =
                        async_channel::bounded::<Result<Option<String>, String>>(1);

                    if action_tx.send_blocking((command.clone(), resp_tx)).is_ok() {
                        let resp = resp_rx.recv_blocking();
                        let _ = match resp {
                            Ok(Ok(None)) => writeln!(stream, "OK"),
                            Ok(Ok(Some(r))) => writeln!(stream, "OK {r}"),
                            Ok(Err(e)) => writeln!(stream, "ERR {e}"),
                            Err(e) => writeln!(stream, "ERR {e}"),
                        };
                    } else {
                        error!("Test automation channel closed, dropping '{command}'");
                        let _ = writeln!(stream, "ERR application channel closed");
                    }
                }
                Err(err) => {
                    warn!("Test automation read error: {err}");
                    let _ = writeln!(stream, "ERR {err}");
                }
            }
        }
    });

    glib::spawn_future_local(glib::clone!(
        #[strong]
        app,
        async move {
            while let Ok((command, resp_tx)) = action_rx.recv().await {
                debug!("Test automation dispatching '{command}'");
                match handle_command(&app, &command) {
                    Ok(v) => {
                        let _ = resp_tx.send(Ok(v)).await;
                    }
                    Err(err) => {
                        warn!("Test automation '{command}' failed: {err}");
                        let _ = resp_tx.send(Err(err.to_string())).await;
                    }
                }
            }
        }
    ));
}

fn handle_command(
    app: &ControlPanelGuiApplication,
    command: &str,
) -> anyhow::Result<Option<String>> {
    let mut args = command.split_whitespace();
    let verb = args.next().context("empty command")?;

    match verb {
        "activate" => {
            let action = args.next().context("activate: missing action name")?;
            app.activate_action(action, None);
            return Ok(None);
        }
        "set-locale-timezone" => {
            let locale = args.next().context("set-locale-timezone: missing locale")?;
            let timezone = args
                .next()
                .context("set-locale-timezone: missing timezone")?;
            app.perform_setting_action(SettingsAction::RegionNLanguage {
                locale: locale.to_string(),
                timezone: timezone.to_string(),
            });
            return Ok(None);
        }
        _ => {}
    }

    let id = args.next().with_context(|| format!("{verb}: missing id"))?;
    let widget =
        find_widget(app, id).with_context(|| format!("{verb}: widget '{id}' not found"))?;

    match verb {
        "click" => {
            widget.emit_by_name::<()>("clicked", &[]);
        }
        "set-active" => {
            let value = args.next().context("set-active: missing true|false")?;
            let active: bool = value.parse().context("set-active: invalid value")?;
            widget.set_property("active", active);
        }
        "set-text" => {
            let value = args.collect::<Vec<_>>().join(" ");
            ensure!(!value.is_empty(), "set-text: usage: set-text <id> <value>");
            widget.set_property("text", value);
        }
        "select" => {
            let value = args.next().context("select: missing index")?;
            let index = value
                .parse()
                .with_context(|| format!("select: invalid index '{value}'"))?;
            match widget.downcast::<gtk::ListBox>() {
                Ok(listbox) => {
                    if let Some(row) = listbox.row_at_index(index) {
                        listbox.select_row(Some(&row));
                    } else {
                        bail!("select-row: invalid index");
                    }
                }
                Err(widget) => widget.set_property("selected", u32::try_from(index).unwrap_or(0)),
            }
        }
        "get" => {
            let property = args.next().context("get: missing property")?;
            let spec = widget
                .find_property(property)
                .context("get: property not found")?;
            ensure!(
                spec.flags().contains(glib::ParamFlags::READABLE),
                "property not readable"
            );

            return widget
                .property_value(property)
                .transform::<String>()
                .context("get: transform failed")?
                .get_owned()
                .context("get: value fetch failed")
                .map(Some);
        }
        _ => bail!("[test-automation] unknown command '{verb}'"),
    }
    Ok(None)
}

struct ChildrenIterator {
    child: Option<gtk::Widget>,
}

impl Iterator for ChildrenIterator {
    type Item = gtk::Widget;

    fn next(&mut self) -> Option<Self::Item> {
        self.child
            .take()
            .inspect(|prev| self.child = prev.next_sibling())
    }
}

trait WidgetChildExt {
    fn children(&self) -> ChildrenIterator;
}

impl<W: IsA<gtk::Widget>> WidgetChildExt for W {
    fn children(&self) -> ChildrenIterator {
        ChildrenIterator {
            child: self.first_child(),
        }
    }
}

fn find_widget(app: &ControlPanelGuiApplication, id: &str) -> Option<gtk::Widget> {
    let root = app.active_window()?.upcast::<gtk::Widget>();
    find_template_child_by_id(&root, id)
}

/// Recursively find a template child `id` anywhere under `root`.
///
/// `gtk_widget_get_template_child()` only knows the class template of the type
/// it is asked for, so every node is queried with its own dynamic type. That
/// resolves ids bound by *any* composite template in the hierarchy — e.g.
/// `check_button` from `UpdatePage`, even though the widget lives inside
/// `Settings` inside `ControlPanelGuiWindow`.
fn find_template_child_by_id(root: &gtk::Widget, id: &str) -> Option<gtk::Widget> {
    if let Some(found) = lookup_template_child(root, id) {
        return Some(found);
    }
    root.children()
        .find_map(|child| find_template_child_by_id(&child, id))
}

/// Look up a single template child by its `id` via
/// `gtk_widget_get_template_child()`, scoped to `widget`'s concrete type.
///
/// This only finds children a composite template class *bound* with
/// `#[template_child]`; un-bound children or unknown ids yield `None`.
///
/// # Safety
///
/// This is the only place that touches raw `gtk4_sys`, and the whole call lives
/// in a single `unsafe` block. It is sound because:
///
/// - `widget` is a live `&gtk::Widget`; `to_glib_none()` passes a borrowed
///   `GtkWidget*` that stays valid for the duration of the call.
/// - `widget.type_()` is that same widget's dynamic `GType`, so the template
///   child offsets GTK resolves against it are valid for the instance.
/// - The return value is a borrowed pointer (transfer none) owned by the
///   widget's template. `from_glib_none()` takes the strong reference the
///   binding requires, so the result owns its ref and drops it on destruction.
/// - `NULL` (no such child) is checked before any dereference, and the id is a
///   NUL-terminated `CString`, so the string arguments are always valid.
fn lookup_template_child(widget: &gtk::Widget, id: &str) -> Option<gtk::Widget> {
    use gtk::glib::translate::{FromGlibPtrNone, IntoGlib, ToGlibPtr};
    use std::ffi::CString;

    let name = CString::new(id).expect("widget id must not contain NUL bytes");
    let widget_type = widget.type_().into_glib();
    unsafe {
        let child = gtk::ffi::gtk_widget_get_template_child(
            widget.to_glib_none().0,
            widget_type,
            name.as_ptr(),
        );
        if child.is_null() {
            return None;
        }
        Some(gtk::Widget::from_glib_none(child.cast()))
    }
}
