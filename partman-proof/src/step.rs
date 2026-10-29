// Copyright (C) 2026 Jan Brittenson
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The proof's STEP model made into coloured triangles for partman-3d, as
//! Partman's client makes a part's model. Partman does it in a child
//! process with a memory cap, as some models take any amount; this one is
//! known to take a moment, so it's done on a thread here.

use std::collections::HashMap;

use ruststep::ast::Name;
use ruststep::tables::PlaceHolder;
use truck_meshalgo::prelude::*;
use truck_stepio::r#in::{ShellHolder, Table};

/// What a surface is coloured when its file doesn't say: KiCad's grey.
const UNCOLOURED: [u8; 3] = [160, 160, 160];

/// How far a surface's triangles may stray from it, in mm: fine enough to
/// look round at a part's size.
const TOLERANCE: f64 = 0.01;

/// Colours given to items in a STEP file (STYLED_ITEM), by the item's
/// entity: a face, or a solid, which is taken to colour its shell. Read
/// from the text, as the STEP reader leaves styles out.
fn colours(step: &str) -> HashMap<u64, [u8; 3]> {
    let mut ents: HashMap<u64, &str> = HashMap::new();
    for e in step.split(';') {
        if let Some((id, body)) = e.trim().strip_prefix('#').and_then(|r| r.split_once('='))
            && let Ok(id) = id.trim().parse()
        {
            ents.insert(id, body.trim());
        }
    }
    let refs = |s: &str| -> Vec<u64> { s.split('#').skip(1).filter_map(|p| p.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()).collect() };
    // A style is a chain of entities ending in a COLOUR_RGB
    fn find(id: u64, ents: &HashMap<u64, &str>, refs: &dyn Fn(&str) -> Vec<u64>, depth: u32) -> Option<[u8; 3]> {
        let body = ents.get(&id).filter(|_| depth < 12)?;
        if let Some(args) = body.strip_prefix("COLOUR_RGB(") {
            let v: Vec<f32> = args.trim_end_matches(')').split(',').skip(1).filter_map(|x| x.trim().parse().ok()).collect();
            if let [r, g, b] = v[..] {
                return Some([r, g, b].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8));
            }
        }
        refs(body).into_iter().find_map(|r| find(r, ents, refs, depth + 1))
    }
    let mut out = HashMap::new();
    for body in ents.values() {
        if !body.starts_with("STYLED_ITEM(") && !body.starts_with("OVER_RIDING_STYLED_ITEM(") {
            continue;
        }
        let rs = refs(body);
        let Some((&item, styles)) = rs.split_last() else { continue };
        let Some(c) = styles.iter().find_map(|s| find(*s, &ents, &refs, 0)) else { continue };
        out.insert(item, c);
        if let Some(solid) = ents.get(&item).filter(|b| b.starts_with("MANIFOLD_SOLID_BREP(")) {
            for shell in refs(solid) {
                out.entry(shell).or_insert(c);
            }
        }
    }
    out
}

/// A STEP file's text as a scene, a mesh per colour.
pub fn scene(step: &str) -> Result<partman_3d::Scene, String> {
    let colours = colours(step);
    let table = Table::from_step(step).ok_or("not a STEP file it can read")?;
    let mut by_colour: HashMap<[u8; 3], partman_3d::Mesh> = HashMap::new();
    for (shell_id, shell) in &table.shell {
        let shell_colour = colours.get(shell_id).copied();
        let colour_of = |face: &PlaceHolder<_>| {
            let id = match face {
                PlaceHolder::Ref(Name::Entity(id)) => Some(*id),
                _ => None,
            };
            id.and_then(|f| colours.get(&f).copied()).or(shell_colour).unwrap_or(UNCOLOURED)
        };
        let mut add = |c: [u8; 3], poly: &PolygonMesh| {
            let mesh = by_colour.entry(c).or_insert_with(|| partman_3d::Mesh { color: egui::Color32::from_rgb(c[0], c[1], c[2]), ..Default::default() });
            let base = mesh.points.len() as u32;
            mesh.points.extend(poly.positions().iter().map(|p| [p.x as f32, p.y as f32, p.z as f32]));
            for tri in poly.faces().triangle_iter() {
                mesh.triangles.push(tri.map(|v| base + v.pos as u32));
            }
        };
        // The shell at once, so faces meeting along an edge are cut alike
        // and no seams show; faces the reader drops would put the colours
        // out of step, so then a face at a time
        if let Ok(whole) = table.to_compressed_shell(shell)
            && whole.faces.len() == shell.cfs_faces.len()
        {
            let meshed = whole.robust_triangulation(TOLERANCE);
            for (face, f) in shell.cfs_faces.iter().zip(&meshed.faces) {
                if let Some(poly) = &f.surface {
                    add(colour_of(face), poly);
                }
            }
            continue;
        }
        for face in &shell.cfs_faces {
            let one = ShellHolder { label: String::new(), cfs_faces: vec![face.clone()] };
            let Ok(compressed) = table.to_compressed_shell(&one) else { continue };
            add(colour_of(face), &compressed.robust_triangulation(TOLERANCE).to_polygon());
        }
    }
    let meshes: Vec<partman_3d::Mesh> = by_colour.into_values().filter(|m| !m.triangles.is_empty()).collect();
    if meshes.is_empty() {
        return Err("no surfaces it could make out".into());
    }
    Ok(partman_3d::Scene { meshes })
}

#[cfg(test)]
mod tests {
    /// The proof's model comes out as coloured triangles: the body and the
    /// leads, of a 10 mm package. No GPU: only drawing it needs one.
    #[test]
    fn the_model_makes_triangles() {
        let scene = super::scene(include_str!("../assets/LQFP-64_10x10mm_P0.5mm.step")).unwrap();
        assert!(scene.meshes.len() >= 2, "{} colours", scene.meshes.len());
        let xs = scene.meshes.iter().flat_map(|m| m.points.iter().map(|p| p[0]));
        let (lo, hi) = xs.fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
        assert!((hi - lo - 12.0).abs() < 1.0, "leads tip to tip: {}", hi - lo);
    }
}
