// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The fonts Partman, the pin planner and the apps like them are drawn in,
//! built in so every app has the same: Inter for text, its semibold face
//! as a family of its own for headings and the agent button's A, and
//! egui's own faces behind both, for scripts and symbols Inter lacks.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, FontId};

/// The semibold family's name, for `FontFamily::Name`.
pub const SEMIBOLD: &str = "semibold";

/// Makes the app's fonts Inter's, before its first frame.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("Inter".into(), Arc::new(FontData::from_static(include_bytes!("../fonts/Inter-Regular.ttf"))));
    fonts.font_data.insert("Inter-SemiBold".into(), Arc::new(FontData::from_static(include_bytes!("../fonts/Inter-SemiBold.ttf"))));
    let defaults: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut prop = vec!["Inter".to_string()];
    prop.extend(defaults.iter().cloned());
    fonts.families.insert(FontFamily::Proportional, prop);
    let mut semi = vec!["Inter-SemiBold".to_string()];
    semi.extend(defaults);
    fonts.families.insert(semibold_family(), semi);
    ctx.set_fonts(fonts);
}

/// The semibold family, for a bold A on agent buttons, say.
pub fn semibold_family() -> FontFamily {
    FontFamily::Name(SEMIBOLD.into())
}

/// Semibold text at a size, for headings.
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, semibold_family())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Installed, the families are there and Inter lays out text, the
    /// arrow the advance buttons end in included, which egui's own
    /// proportional face lacks.
    #[test]
    fn installed_fonts_lay_out_text() {
        // Nothing drawn: the harness draws a frame as it's made, before
        // the fonts are installed
        let mut h = egui_kittest::Harness::new_ui(|_| {});
        install(&h.ctx);
        h.run();
        assert!(h.ctx.fonts(|f| f.families().contains(&semibold_family())));
        // Asking about glyphs loads them, so it takes the fonts mutably
        h.ctx.fonts_mut(|f| {
            assert!(f.has_glyphs(&FontId::proportional(14.0), "Details →"));
            assert!(f.has_glyphs(&semibold(14.0), "Details →"));
        });
    }
}
