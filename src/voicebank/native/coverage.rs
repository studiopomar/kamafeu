use super::NativeBank;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCoverageSummary {
    pub profile_id: String,
    pub style_id: String,
    pub covered_phone_symbols: usize,
    pub total_phone_symbols: usize,
    pub unit_count: usize,
    pub asset_count: usize,
    pub reviewed_unit_count: usize,
}

impl NativeBank {
    /// Structural coverage only: counts profile symbols occurring in authored
    /// units, without estimating phrase coverage or acoustic suitability.
    pub fn coverage_summary(
        &self,
        profile_id: &str,
        style_id: &str,
    ) -> Result<NativeCoverageSummary, String> {
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| format!("unknown phonetic profile: {profile_id}"))?;
        if !self
            .manifest
            .styles
            .iter()
            .any(|style| style.id == style_id)
        {
            return Err(format!("unknown KFV 2 style: {style_id}"));
        }
        let mut covered_phones = HashSet::new();
        let mut assets = HashSet::new();
        let mut unit_count = 0usize;
        let mut reviewed_unit_count = 0usize;
        for (_, units) in self
            .layers
            .iter()
            .filter(|(layer, _)| layer.style_id == style_id)
        {
            unit_count += units.units.len();
            reviewed_unit_count += units
                .units
                .iter()
                .filter(|unit| unit.quality.reviewed)
                .count();
            assets.extend(units.assets.iter().map(|asset| asset.path.as_str()));
            covered_phones.extend(
                units
                    .units
                    .iter()
                    .flat_map(|unit| unit.phones.iter().map(String::as_str)),
            );
        }
        Ok(NativeCoverageSummary {
            profile_id: profile_id.to_owned(),
            style_id: style_id.to_owned(),
            covered_phone_symbols: profile
                .symbols
                .iter()
                .filter(|symbol| covered_phones.contains(symbol.id.as_str()))
                .count(),
            total_phone_symbols: profile.symbols.len(),
            unit_count,
            asset_count: assets.len(),
            reviewed_unit_count,
        })
    }
}
