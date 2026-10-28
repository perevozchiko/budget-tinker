use super::*;

impl App {
    pub(in crate::ui) fn menu_commands(&mut self, ctx: &egui::Context) {
        if self.native_menu.language != self.preferences.language {
            self.native_menu = crate::native_menu::NativeMenu::new(self.preferences.language, ctx);
            self.chrome.versions_dirty = true;
        }
        if self.chrome.versions_dirty {
            match self.store.versions() {
                Ok(versions) => self.native_menu.versions(&versions),
                Err(e) => self.notify(e, true),
            }
            self.chrome.versions_dirty = false;
        }
        self.native_menu.set_enabled(
            ctx.wants_keyboard_input() || !self.undo.is_empty(),
            ctx.wants_keyboard_input() || !self.redo.is_empty(),
            self.dialog.is_none() && self.chrome.overlay.is_none(),
        );
        for command in self.native_menu.commands() {
            match command.as_str() {
                "new" => self.screen(Screen::New),
                "open" => self.screen(Screen::Open),
                "preferences" => self.screen(Screen::Preferences),
                "properties" => self.screen(Screen::Properties),
                "save_version" => match self.store.save(&self.b) {
                    Ok(()) => {
                        self.chrome.versions_dirty = true;
                        self.chrome.overlay = Some(Overlay::SavedVersion);
                    }
                    Err(e) => self.notify(e, true),
                },
                "fresh" => self.chrome.overlay = Some(Overlay::Fresh),
                "export" => {
                    if self.page == Page::Reports {
                        self.export_current_report(ctx);
                    } else {
                        self.chrome.overlay = Some(Overlay::Export);
                    }
                }
                "wifi" => self.chrome.overlay = Some(Overlay::Wifi),
                "payees" => {
                    self.chrome.payee.clear();
                    self.chrome.overlay = Some(Overlay::Payees);
                }
                "about" => self.chrome.overlay = Some(Overlay::About),
                "diagnostics" => self.chrome.overlay = Some(Overlay::Diagnostics),
                "technical" => self.chrome.overlay = Some(Overlay::Technical),
                "privacy" | "help" | "getting_started" | "release_notes" | "walkthrough"
                | "credit_basics" | "credit_payments" => {
                    self.chrome.overlay = Some(Overlay::Help(command.clone()))
                }
                "import" => {
                    self.chrome.screen = None;
                    self.open(DialogKind::Import);
                }
                "restore" => self.open(DialogKind::Restore),
                "backup" => {
                    if let Some(path) = self.choose_file(true, "json", ctx)
                        && let Err(e) = store::export_json(&self.b, Path::new(&path))
                    {
                        self.notify(e, true);
                    }
                }
                "reveal" => {
                    if let Err(error) = crate::desktop::reveal_budget(&self.store.path) {
                        self.notify(error, true);
                    }
                }
                "accounts" | "budget" | "reports" | "scheduled" => {
                    self.chrome.print_preview = false;
                    self.chrome.screen = None;
                    self.chrome.overlay = None;
                    self.dialog = None;
                    self.page = match command.as_str() {
                        "accounts" => Page::Accounts,
                        "reports" => Page::Reports,
                        "scheduled" => Page::Accounts,
                        _ => Page::Budget,
                    };
                    self.account = None;
                    if command == "scheduled" {
                        self.register_scheduled_open = true;
                    }
                }
                "undo" | "redo" => {
                    if ctx.wants_keyboard_input() {
                        ctx.input_mut(|i| {
                            i.events.push(egui::Event::Key {
                                key: egui::Key::Z,
                                physical_key: Some(egui::Key::Z),
                                pressed: true,
                                repeat: false,
                                modifiers: egui::Modifiers {
                                    shift: command == "redo",
                                    ..egui::Modifiers::COMMAND
                                },
                            })
                        });
                    } else {
                        self.undo(command == "redo");
                    }
                }
                "cut" | "copy" | "paste" | "select_all" => {
                    let event = match command.as_str() {
                        "cut" => egui::Event::Cut,
                        "copy" => egui::Event::Copy,
                        "paste" => {
                            let value = std::process::Command::new("/usr/bin/pbpaste")
                                .output()
                                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                                .unwrap_or_default();
                            egui::Event::Paste(value)
                        }
                        _ => egui::Event::Key {
                            key: egui::Key::A,
                            physical_key: Some(egui::Key::A),
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::COMMAND,
                        },
                    };
                    ctx.input_mut(|i| i.events.push(event));
                }
                "print" => {
                    self.chrome.print_preview = true;
                    self.chrome.screen = None;
                }
                "fullscreen" => ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                    !ctx.input(|i| i.viewport().fullscreen.unwrap_or(false)),
                )),
                "front" => ctx.send_viewport_cmd(egui::ViewportCommand::Focus),
                "quit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                _ => {
                    if let Some(id) = command
                        .strip_prefix("version:")
                        .and_then(|id| id.parse().ok())
                    {
                        self.chrome.overlay = Some(Overlay::LoadVersion(id));
                    }
                }
            }
        }
    }
}
