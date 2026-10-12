//! Native macOS application menu. AppKit owns this menu bar, so it belongs
//! above the window beside the Apple menu instead of inside the egui canvas.

use std::cell::RefCell;

use muda::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    NewProject,
    OpenProject,
    SaveProject,
    ExportAudio,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SingerGallery,
    Preferences,
    Packages,
    CommandPalette,
    Help,
    Portuguese,
    English,
}

thread_local! {
    // AppKit menu objects must stay alive and be accessed on the main thread.
    static MENU: RefCell<Option<Menu>> = const { RefCell::new(None) };
}

fn item(id: &'static str, title: &'static str) -> MenuItem {
    MenuItem::with_id(id, title, true, None)
}

pub fn install() {
    MENU.with(|stored| {
        if stored.borrow().is_some() {
            return;
        }

        let menu = Menu::new();
        let app = Submenu::new("Kamafeu Studio", true);
        let prefs = item("preferences", "Preferências…");
        app.append_items(&[
            &prefs,
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ])
        .expect("native macOS application menu");

        let file = Submenu::new("Arquivo", true);
        let new_project = item("new", "Novo Projeto");
        let open = item("open", "Abrir Projeto…");
        let save = item("save", "Salvar Projeto");
        let export = item("export", "Exportar Áudio…");
        file.append_items(&[
            &new_project,
            &open,
            &save,
            &PredefinedMenuItem::separator(),
            &export,
        ])
        .expect("native macOS file menu");

        let edit = Submenu::new("Editar", true);
        let undo = item("undo", "Desfazer");
        let redo = item("redo", "Refazer");
        let cut = item("cut", "Recortar");
        let copy = item("copy", "Copiar");
        let paste = item("paste", "Colar");
        edit.append_items(&[
            &undo,
            &redo,
            &PredefinedMenuItem::separator(),
            &cut,
            &copy,
            &paste,
        ])
        .expect("native macOS edit menu");

        let singers = Submenu::new("Cantores", true);
        let gallery = item("singers", "Galeria de Cantores…");
        singers.append(&gallery).expect("native macOS singers menu");

        let tools = Submenu::new("Ferramentas", true);
        let command_palette = item("commands", "Paleta de Comandos…");
        tools
            .append(&command_palette)
            .expect("native macOS tools menu");

        let language = Submenu::new("Idioma", true);
        let portuguese = item("pt", "Português (Brasil)");
        let english = item("en", "English (US)");
        language
            .append_items(&[&portuguese, &english])
            .expect("native macOS language menu");

        let packages = Submenu::new("Pacotes", true);
        let package_manager = item("packages", "Gerenciar Pacotes…");
        packages
            .append(&package_manager)
            .expect("native macOS packages menu");

        let help = Submenu::new("Ajuda", true);
        let guide = item("help", "Guia de Atalhos…");
        help.append(&guide).expect("native macOS help menu");

        menu.append_items(&[
            &app, &file, &edit, &singers, &tools, &language, &packages, &help,
        ])
        .expect("native macOS menu bar");
        menu.init_for_nsapp();
        help.set_as_help_menu_for_nsapp();
        *stored.borrow_mut() = Some(menu);
    });
}

pub fn drain_actions() -> Vec<Action> {
    let mut actions = Vec::new();
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        let action = match event.id {
            MenuId(ref id) if id == "new" => Some(Action::NewProject),
            MenuId(ref id) if id == "open" => Some(Action::OpenProject),
            MenuId(ref id) if id == "save" => Some(Action::SaveProject),
            MenuId(ref id) if id == "export" => Some(Action::ExportAudio),
            MenuId(ref id) if id == "undo" => Some(Action::Undo),
            MenuId(ref id) if id == "redo" => Some(Action::Redo),
            MenuId(ref id) if id == "cut" => Some(Action::Cut),
            MenuId(ref id) if id == "copy" => Some(Action::Copy),
            MenuId(ref id) if id == "paste" => Some(Action::Paste),
            MenuId(ref id) if id == "singers" => Some(Action::SingerGallery),
            MenuId(ref id) if id == "preferences" => Some(Action::Preferences),
            MenuId(ref id) if id == "packages" => Some(Action::Packages),
            MenuId(ref id) if id == "commands" => Some(Action::CommandPalette),
            MenuId(ref id) if id == "help" => Some(Action::Help),
            MenuId(ref id) if id == "pt" => Some(Action::Portuguese),
            MenuId(ref id) if id == "en" => Some(Action::English),
            _ => None,
        };
        if let Some(action) = action {
            actions.push(action);
        }
    }
    actions
}
