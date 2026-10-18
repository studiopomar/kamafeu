use super::{CopaibaToolkitApp, PendingClassicMigration};
use std::path::PathBuf;

impl CopaibaToolkitApp {
    pub fn migrate_classic_voicebank(
        &mut self,
        profile_path: &std::path::Path,
        mapping_path: Option<&std::path::Path>,
        destination_parent: &std::path::Path,
    ) -> Result<(), String> {
        let source_root = self
            .current_dir
            .as_deref()
            .ok_or("Nenhum voicebank clássico aberto")?;
        if self.native_project.is_some() {
            return Err("A migração oto.ini só se aplica a um banco clássico".into());
        }
        let source = crate::oto::Voicebank::new(source_root)
            .map_err(|error| format!("Não foi possível abrir o oto.ini de origem: {error}"))?;
        let (profile, phone_mappings) = load_migration_inputs(profile_path, mapping_path)?;

        let slug = migration_slug(&source.name);
        let options = crate::voicebank::migration::LegacyMigrationOptions {
            voice_id: format!("org.kamafeu.voice.{slug}"),
            bank_id: format!("org.kamafeu.voice.{slug}.native"),
            bank_version: "0.1.0".into(),
            display_name: format!("{} · Nativa (rascunho)", source.name),
            publisher: if source.author.trim().is_empty() {
                "Autor não informado".into()
            } else {
                source.author.clone()
            },
            profile,
            style_id: "natural".into(),
            style_display_name: "Natural".into(),
            layer_id: "natural-midi-60".into(),
            recorded_midi: 60,
            phone_mappings,
        };
        let destination = destination_parent.join(format!("{slug}-kfv2-draft"));
        let report = crate::voicebank::migration::migrate_legacy_voicebank_to_copy(
            &source,
            &destination,
            &options,
        )?;
        drop(source);
        let project =
            crate::copaiba::native_project::CopaibaNativeProject::load_dir(&report.destination)
                .map_err(|error| format!("Rascunho criado, mas não reabriu no Copaíba: {error}"))?;
        self.activate_native_project(project);
        self.migration_report = Some(report.clone());
        let inferred = report
            .entries
            .iter()
            .filter(|entry| {
                entry.mapping_origin == crate::voicebank::migration::MappingOrigin::ExactAliasTokens
            })
            .count();
        let explicit = report
            .entries
            .iter()
            .filter(|entry| {
                entry.mapping_origin == crate::voicebank::migration::MappingOrigin::Explicit
            })
            .count();
        let skipped = report
            .entries
            .iter()
            .filter(|entry| {
                entry.status == crate::voicebank::migration::MigrationEntryStatus::Skipped
            })
            .map(|entry| format!("{}: {}", entry.alias, entry.warnings.join("; ")))
            .collect::<Vec<_>>();
        let mut status = format!(
            "Rascunho KFV 2 criado em {} · {} unidades · {} mapas explícitos · {} aliases inferidos · {} ignorados · SHA-256 {}",
            report.destination.display(),
            report.migrated_units,
            explicit,
            inferred,
            report.skipped_entries,
            report.content_sha256
        );
        if !skipped.is_empty() {
            status.push_str("\nIgnorados:\n");
            status.push_str(&skipped.join("\n"));
        }
        status.push_str(
            "\nRevise todas as marcações e licenças no Copaíba antes de usar ou distribuir.",
        );
        self.status_message = status;
        Ok(())
    }

    pub(super) fn prepare_classic_voicebank_migration(
        &mut self,
        profile_path: PathBuf,
        mapping_path: Option<PathBuf>,
        destination_parent: PathBuf,
    ) -> Result<(), String> {
        let source_root = self
            .current_dir
            .as_deref()
            .ok_or("Nenhum voicebank clássico aberto")?;
        if self.native_project.is_some() {
            return Err("A migração oto.ini só se aplica a um banco clássico".into());
        }
        let source = crate::oto::Voicebank::new(source_root)
            .map_err(|error| format!("Não foi possível abrir o oto.ini de origem: {error}"))?;
        let output_parent = std::fs::canonicalize(&destination_parent)
            .map_err(|error| format!("Não foi possível validar a pasta de destino: {error}"))?;
        let destination =
            output_parent.join(format!("{}-kfv2-draft", migration_slug(&source.name)));
        let canonical_source = std::fs::canonicalize(source_root)
            .map_err(|error| format!("Não foi possível validar o banco de origem: {error}"))?;
        if destination.starts_with(&canonical_source) {
            return Err("A pasta de destino precisa ficar fora do voicebank original".into());
        }
        if destination.exists() || std::fs::symlink_metadata(&destination).is_ok() {
            return Err(format!(
                "A pasta de destino já existe: {}",
                destination.display()
            ));
        }
        let (profile, phone_mappings) =
            load_migration_inputs(&profile_path, mapping_path.as_deref())?;
        let preview = crate::voicebank::migration::preview_legacy_voicebank_migration(
            &source,
            &profile,
            &phone_mappings,
        )?;
        self.pending_classic_migration = Some(PendingClassicMigration {
            profile_path,
            mapping_path,
            destination_parent,
            preview,
        });
        Ok(())
    }

    pub(super) fn confirm_classic_voicebank_migration(&mut self) {
        let Some(pending) = self.pending_classic_migration.take() else {
            return;
        };
        self.status_message = match self.migrate_classic_voicebank(
            &pending.profile_path,
            pending.mapping_path.as_deref(),
            &pending.destination_parent,
        ) {
            Ok(()) => self.status_message.clone(),
            Err(error) => format!("Falha na migração assistida: {error}"),
        };
    }
}

fn migration_slug(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "voicebank".into()
    } else {
        slug.chars().take(64).collect()
    }
}

fn load_migration_inputs(
    profile_path: &std::path::Path,
    mapping_path: Option<&std::path::Path>,
) -> Result<
    (
        crate::voicebank::native::PhoneticProfile,
        std::collections::BTreeMap<String, Vec<String>>,
    ),
    String,
> {
    let profile: crate::voicebank::native::PhoneticProfile = serde_json::from_slice(
        &std::fs::read(profile_path)
            .map_err(|error| format!("Não foi possível ler o perfil fonético: {error}"))?,
    )
    .map_err(|error| format!("Perfil fonético inválido: {error}"))?;
    profile.validate()?;
    let phone_mappings = if let Some(path) = mapping_path {
        serde_json::from_slice::<std::collections::BTreeMap<String, Vec<String>>>(
            &std::fs::read(path)
                .map_err(|error| format!("Não foi possível ler o mapa de aliases: {error}"))?,
        )
        .map_err(|error| {
            format!("Mapa de aliases inválido; esperado objeto alias → lista de IDs: {error}")
        })?
    } else {
        std::collections::BTreeMap::new()
    };
    Ok((profile, phone_mappings))
}
