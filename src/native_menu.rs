//! AppKit menu bar. Events are handled by the same commands as the application UI.
#[cfg(all(target_os = "macos", not(test)))]
mod platform {
    use crate::i18n::Language;
    use eframe::egui;
    use muda::{
        Menu, MenuEvent, MenuItem, PredefinedMenuItem as System, Submenu,
        accelerator::{Accelerator, Code, Modifiers},
    };

    pub struct NativeMenu {
        _menu: Menu,
        app_menu: Submenu,
        versions: Submenu,
        undo: MenuItem,
        redo: MenuItem,
        document_items: Vec<MenuItem>,
        pub language: Language,
    }
    impl NativeMenu {
        pub fn new(language: Language, ctx: &egui::Context) -> Self {
            let menu = Menu::new();
            let title = |en: &str, ru: &str| {
                if language == Language::En {
                    en.to_owned()
                } else {
                    ru.to_owned()
                }
            };
            let item = |id: &str, en: &str, ru: &str, key: Option<(Code, bool)>| {
                MenuItem::with_id(
                    id,
                    title(en, ru),
                    true,
                    key.map(|(code, shift)| {
                        Accelerator::new(
                            if shift {
                                Modifiers::META | Modifiers::SHIFT
                            } else {
                                Modifiers::META
                            },
                            code,
                        )
                    }),
                )
            };
            let app = Submenu::new("BudgetTinker", true);
            app.append_items(&[
                &item(
                    "about",
                    "About BudgetTinker",
                    "О программе BudgetTinker",
                    None,
                ),
                &System::separator(),
                &item(
                    "preferences",
                    "Preferences…",
                    "Настройки…",
                    Some((Code::Comma, false)),
                ),
                &System::separator(),
                &System::hide(Some(&title("Hide BudgetTinker", "Скрыть BudgetTinker"))),
                &System::hide_others(Some(&title("Hide Others", "Скрыть остальные"))),
                &System::show_all(Some(&title("Show All", "Показать все"))),
                &System::separator(),
                &item(
                    "quit",
                    "Quit BudgetTinker",
                    "Завершить BudgetTinker",
                    Some((Code::KeyQ, false)),
                ),
            ])
            .unwrap();
            let file = Submenu::new(title("File", "Файл"), true);
            let versions = Submenu::new(
                title("Load Another Version", "Загрузить другую версию"),
                true,
            );
            let mut document_items = Vec::new();
            for (id, en, ru, key) in [
                (
                    "new",
                    "Create a New Budget…",
                    "Создать новый бюджет…",
                    Some((Code::KeyN, true)),
                ),
                ("open", "Open…", "Открыть…", Some((Code::KeyO, false))),
                (
                    "save_version",
                    "Save a Version",
                    "Сохранить версию",
                    Some((Code::KeyS, false)),
                ),
            ] {
                if id == "save_version" {
                    file.append(&System::separator()).unwrap();
                }
                let entry = item(id, en, ru, key);
                file.append(&entry).unwrap();
                document_items.push(entry);
            }
            file.append(&versions).unwrap();
            for (id, en, ru) in [
                ("fresh", "Make a Fresh Start…", "Начать заново…"),
                ("-", "", ""),
                (
                    "import",
                    "Import Transactions from Your Bank…",
                    "Импорт операций из банка…",
                ),
                (
                    "wifi",
                    "Wi-Fi Sync with Mobile Device…",
                    "Синхронизация с устройством по Wi-Fi…",
                ),
                ("-", "", ""),
                ("properties", "Budget Properties…", "Свойства бюджета…"),
                ("-", "", ""),
                ("export", "Export…", "Экспорт…"),
                ("print", "Print…", "Напечатать…"),
            ] {
                if id == "-" {
                    file.append(&System::separator()).unwrap();
                    continue;
                }
                let entry = item(id, en, ru, (id == "print").then_some((Code::KeyP, false)));
                file.append(&entry).unwrap();
                document_items.push(entry);
            }
            let backup = Submenu::new(title("Backup", "Резервная копия"), true);
            backup
                .append_items(&[
                    &item("backup", "Export JSON…", "Экспорт JSON…", None),
                    &item("restore", "Restore JSON…", "Восстановить JSON…", None),
                    &item(
                        "reveal",
                        "Show Budget in Finder",
                        "Показать бюджет в Finder",
                        None,
                    ),
                ])
                .unwrap();
            file.append_items(&[&System::separator(), &backup]).unwrap();
            let edit = Submenu::new(title("Edit", "Правка"), true);
            let undo = item("undo", "Undo", "Отменить", Some((Code::KeyZ, false)));
            let redo = item("redo", "Redo", "Повторить", Some((Code::KeyZ, true)));
            edit.append_items(&[
                &undo,
                &redo,
                &System::separator(),
                &item("cut", "Cut", "Вырезать", Some((Code::KeyX, false))),
                &item("copy", "Copy", "Скопировать", Some((Code::KeyC, false))),
                &item("paste", "Paste", "Вставить", Some((Code::KeyV, false))),
                &item(
                    "select_all",
                    "Select All",
                    "Выбрать все",
                    Some((Code::KeyA, false)),
                ),
            ])
            .unwrap();
            let view = Submenu::new(title("View", "Вид"), true);
            for (id, en, ru, key) in [
                (
                    "accounts",
                    "Go to Accounts",
                    "Перейти к счетам",
                    Code::Digit1,
                ),
                ("budget", "Go to Budget", "Перейти к бюджету", Code::Digit2),
                (
                    "reports",
                    "Go to Reports",
                    "Перейти к отчётам",
                    Code::Digit3,
                ),
                (
                    "payees",
                    "Go to Payee Settings",
                    "Настройки получателей",
                    Code::Digit4,
                ),
            ] {
                view.append(&item(id, en, ru, Some((key, false)))).unwrap();
            }
            view.append_items(&[
                &System::separator(),
                &item(
                    "scheduled",
                    "Scheduled Transactions",
                    "Плановые операции",
                    None,
                ),
            ])
            .unwrap();
            let window = Submenu::new(title("Window", "Окно"), true);
            window
                .append_items(&[
                    &System::minimize(Some(&title("Minimize", "Свернуть"))),
                    &System::zoom(Some(&title("Zoom", "Масштабировать"))),
                    &item("fullscreen", "Enter Full Screen", "На весь экран", None),
                    &System::separator(),
                    &item(
                        "front",
                        "Bring All to Front",
                        "Все окна на передний план",
                        None,
                    ),
                ])
                .unwrap();
            let help = Submenu::new(title("Help", "Справка"), true);
            help.append_items(&[
                &item("help", "Online Help", "Справка", None),
                &item("getting_started", "Getting Started", "Первые шаги", None),
                &System::separator(),
                &item("release_notes", "Release Notes", "История изменений", None),
            ])
            .unwrap();
            let walkthroughs = Submenu::new(title("Walkthroughs", "Обучение"), true);
            walkthroughs
                .append_items(&[
                    &item(
                        "walkthrough",
                        "Getting Started with BudgetTinker",
                        "Начало работы с BudgetTinker",
                        None,
                    ),
                    &item(
                        "credit_basics",
                        "Credit Card Basics",
                        "Основы кредитных карт",
                        None,
                    ),
                    &item(
                        "credit_payments",
                        "Credit Card Payments",
                        "Платежи по кредитным картам",
                        None,
                    ),
                ])
                .unwrap();
            help.append_items(&[
                &walkthroughs,
                &System::separator(),
                &item(
                    "diagnostics",
                    "Enable/Disable Diagnostics Mode…",
                    "Режим диагностики…",
                    None,
                ),
                &item(
                    "technical",
                    "Technical Information…",
                    "Техническая информация…",
                    None,
                ),
                &System::separator(),
                &item("privacy", "Privacy Policy", "Конфиденциальность", None),
            ])
            .unwrap();
            menu.append_items(&[&app, &file, &edit, &view, &window, &help])
                .unwrap();
            menu.init_for_nsapp();
            window.set_as_windows_menu_for_nsapp();
            help.set_as_help_menu_for_nsapp();
            let ctx = ctx.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                COMMANDS.lock().unwrap().push(event.id.0);
                ctx.request_repaint();
            }));
            Self {
                _menu: menu,
                app_menu: app,
                versions,
                undo,
                redo,
                document_items,
                language,
            }
        }
        pub fn commands(&self) -> Vec<String> {
            std::mem::take(&mut *COMMANDS.lock().unwrap())
        }
        pub fn set_enabled(&self, undo: bool, redo: bool, document: bool) {
            if self.app_menu.text() != "BudgetTinker" {
                self.app_menu.set_text("BudgetTinker");
            }
            self.undo.set_enabled(undo);
            self.redo.set_enabled(redo);
            for item in &self.document_items {
                item.set_enabled(document);
            }
            self.versions.set_enabled(document);
        }
        pub fn versions(&self, versions: &[(i64, String)]) {
            while !self.versions.items().is_empty() {
                self.versions.remove_at(0);
            }
            if versions.is_empty() {
                self.versions
                    .append(&MenuItem::new(
                        if self.language == Language::En {
                            "No saved versions"
                        } else {
                            "Нет сохранённых версий"
                        },
                        false,
                        None,
                    ))
                    .unwrap();
            }
            for (id, date) in versions {
                self.versions
                    .append(&MenuItem::with_id(
                        format!("version:{id}"),
                        date,
                        true,
                        None,
                    ))
                    .unwrap();
            }
        }
    }
    static COMMANDS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
}
#[cfg(all(target_os = "macos", not(test)))]
pub use platform::NativeMenu;

// macOS uses AppKit menus. Other platforms use the in-app egui commands;
// headless tests also use this no-op native-menu adapter.
#[cfg(any(test, not(target_os = "macos")))]
pub struct NativeMenu {
    pub language: crate::i18n::Language,
}
#[cfg(any(test, not(target_os = "macos")))]
impl NativeMenu {
    pub fn new(language: crate::i18n::Language, _: &eframe::egui::Context) -> Self {
        Self { language }
    }
    pub fn commands(&self) -> Vec<String> {
        vec![]
    }
    pub fn set_enabled(&self, _: bool, _: bool, _: bool) {}
    pub fn versions(&self, _: &[(i64, String)]) {}
}
