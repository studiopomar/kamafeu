use std::path::PathBuf;

/// Diálogo de arquivos universal e seguro para todas as plataformas (Desktop + Android)
#[derive(Default, Clone)]
pub struct FileDialog {
    title: Option<String>,
    filters: Vec<(String, Vec<String>)>,
    file_name: Option<String>,
}

impl FileDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn add_filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        self.filters.push((
            name.into(),
            extensions.iter().map(|s| s.to_string()).collect(),
        ));
        self
    }

    pub fn set_file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    pub fn pick_file(self) -> Option<PathBuf> {
        #[cfg(not(target_os = "android"))]
        {
            let mut dialog = rfd::FileDialog::new();
            if let Some(title) = self.title {
                dialog = dialog.set_title(title);
            }
            if let Some(file_name) = self.file_name {
                dialog = dialog.set_file_name(file_name);
            }
            for (name, exts) in &self.filters {
                let ext_refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
                dialog = dialog.add_filter(name, &ext_refs);
            }
            dialog.pick_file()
        }
        #[cfg(target_os = "android")]
        {
            None
        }
    }

    pub fn save_file(self) -> Option<PathBuf> {
        #[cfg(not(target_os = "android"))]
        {
            let mut dialog = rfd::FileDialog::new();
            if let Some(title) = self.title {
                dialog = dialog.set_title(title);
            }
            if let Some(file_name) = self.file_name {
                dialog = dialog.set_file_name(file_name);
            }
            for (name, exts) in &self.filters {
                let ext_refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
                dialog = dialog.add_filter(name, &ext_refs);
            }
            dialog.save_file()
        }
        #[cfg(target_os = "android")]
        {
            None
        }
    }

    pub fn pick_folder(self) -> Option<PathBuf> {
        #[cfg(not(target_os = "android"))]
        {
            let mut dialog = rfd::FileDialog::new();
            if let Some(title) = self.title {
                dialog = dialog.set_title(title);
            }
            dialog.pick_folder()
        }
        #[cfg(target_os = "android")]
        {
            None
        }
    }
}
