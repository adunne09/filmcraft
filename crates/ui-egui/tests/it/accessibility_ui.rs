//! Semantic inspection must work without pointer movement, hover, or editing commands.

use std::sync::Arc;

use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use filmcraft_engine::Session;
use filmcraft_ui_egui::FilmcraftApp;
use serde_json::json;

#[test]
fn clipped_toggle_exposes_only_its_visible_bounds_and_inherited_disabled_state() {
    use filmcraft_ui_egui::{
        theme::{ThemeKind, Tokens},
        widgets,
    };
    let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(14.0, 14.0));
    let clip = egui::Rect::from_min_max(egui::pos2(0.0, 27.0), egui::pos2(100.0, 100.0));
    let mut h = Harness::builder().with_context_setup(|ctx| filmcraft_ui_egui::theme::install(ctx, &Tokens::for_kind(ThemeKind::Dark))).build_ui(|ui| {
        ui.add_enabled_ui(false, |ui| {
            let response = widgets::letter_toggle_clipped(
                ui,
                rect,
                clip,
                "M",
                true,
                egui::Color32::GREEN,
                &Tokens::for_kind(ThemeKind::Dark),
                egui::Id::unique("clipped-mute"),
            );
            widgets::button_accessibility(&response, "Mute A1", Some(true));
        });
    });
    h.run_steps(2);
    let node = h.get_by_role_and_label(Role::Button, "Mute A1");
    assert_eq!(node.rect(), rect.intersect(clip));
    assert!(node.accesskit_node().is_disabled());
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::True));
}

#[test]
fn fully_clipped_icon_has_no_native_accessibility_target_or_infinite_bounds() {
    use filmcraft_ui_egui::{
        icons::Icon,
        theme::{ThemeKind, Tokens},
        widgets,
    };
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let response =
            widgets::icon_toggle(ui, egui::Rect::NOTHING, Icon::Lock, false, &Tokens::for_kind(ThemeKind::Dark), egui::Id::unique("hidden-lock"), None);
        widgets::button_accessibility(&response, "Lock V3", Some(false));
    });
    output.textures_delta.clear();
    let update = output.platform_output.accesskit_update.as_ref().unwrap();
    for (_, node) in &update.nodes {
        assert!(node.is_hidden() || node.label() != Some("Lock V3"), "fully clipped controls must not be offered as click targets");
        if let Some(bounds) = node.bounds() {
            assert!([bounds.x0, bounds.y0, bounds.x1, bounds.y1].into_iter().all(f64::is_finite));
        }
    }
}

#[test]
fn editing_controls_have_names_roles_states_and_visible_bounds_without_hover() {
    let mut session = Session::default();
    session.execute("file.openDemoProject", json!({})).unwrap();
    let project = session.project.clone();
    let playheads = session.state.playheads.clone();
    let app = FilmcraftApp::new(session);
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 980.0)).build_eframe(move |_| app);
    h.run_steps(4);
    for (label, role, toggled) in [
        ("Razor Tool", Role::Button, Some(Toggled::False)),
        ("Snap in Timeline (S)", Role::Button, Some(Toggled::True)),
        ("Program: Play-Stop Toggle (Space)", Role::Button, Some(Toggled::False)),
        ("Quick Export", Role::Button, None),
        ("Export", Role::Tab, None),
        ("Lock V1", Role::Button, Some(Toggled::False)),
        ("Mute A1", Role::Button, Some(Toggled::False)),
        ("Lock V2", Role::Button, Some(Toggled::False)),
        ("Mute A2", Role::Button, Some(Toggled::False)),
        ("Solo A1", Role::Button, Some(Toggled::False)),
        ("Sync Lock V1", Role::Button, Some(Toggled::True)),
        ("Track Output V1", Role::Button, Some(Toggled::True)),
        ("Target V1", Role::Button, Some(Toggled::True)),
        ("Source Patch V1", Role::Button, Some(Toggled::True)),
    ] {
        let node = h.get_by_role_and_label(role, label);
        assert_eq!(node.accesskit_node().toggled(), toggled, "{label}");
        assert!(!node.accesskit_node().is_disabled(), "{label}");
        assert!(node.rect().is_positive(), "{label}");
    }
    assert_eq!(h.get_by_role_and_label(Role::Tab, "Export").accesskit_node().is_selected(), Some(false));
    assert_eq!(h.get_by_role_and_label(Role::Tab, "Edit").accesskit_node().is_selected(), Some(true));
    assert!(Arc::ptr_eq(&project, &h.state().session.project));
    assert_eq!(h.state().session.state.playheads, playheads);
    assert!(!h.state().playback.playing);
    for label in ["Clear (Delete)", "Automate to Sequence"] {
        assert!(h.get_by_role_and_label(Role::Button, label).accesskit_node().is_disabled(), "{label}");
    }
    for label in ["New Bin", "New Item", "Sort Icons"] {
        assert!(!h.get_by_role_and_label(Role::Button, label).accesskit_node().is_disabled(), "{label}");
    }

    // Change scratch state directly: semantics must reflect it, not a constant visual tint.
    h.state_mut().session.execute("timeline.setTrack", json!({"track": "V1", "locked": true})).unwrap();
    h.state_mut().session.execute("timeline.setTrack", json!({"track": "A1", "muted": true})).unwrap();
    h.state_mut().session.state.snapping = false;
    h.run_steps(2);
    for (label, toggled) in [("Lock V1", Toggled::True), ("Mute A1", Toggled::True), ("Snap in Timeline (S)", Toggled::False)] {
        assert_eq!(h.get_by_role_and_label(Role::Button, label).accesskit_node().toggled(), Some(toggled), "{label}");
    }
}
