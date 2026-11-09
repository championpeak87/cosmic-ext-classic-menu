// SPDX-License-Identifier: GPL-3.0-only

//! The main menu as a layer surface.
//!
//! A popup only gets the keyboard with a grab, which needs the serial of a
//! recent input event. Menus opened from a shortcut have none, as the
//! shortcut reaches the applet over D-Bus. The compositor gives a layer
//! surface the keyboard as soon as it is mapped, whatever opened it, and the
//! panel forwards layer surfaces of applets to the compositor.

use cosmic::applet::cosmic_panel_config::{CosmicPanelConfig, PanelAnchor};
use cosmic::cctk::sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity};
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::iced::Alignment;
use cosmic::iced::Limits;
use cosmic::iced::runtime::platform_specific::wayland::layer_surface::{
    IcedMargin, SctkLayerSurfaceSettings,
};
use cosmic::iced::window::Id;

/// Whether a panel hosts the applet, rather than it being run directly.
///
/// Only then can the menu be a layer surface: the compositor refuses them to
/// sandboxed clients, which a Flatpak run directly is, while the panel
/// creates them on behalf of its applets.
pub fn hosted_by_panel() -> bool {
    std::env::var_os("COSMIC_PANEL_NAME").is_some()
}

/// Where along the panel the applet sits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PanelSection {
    Start,
    Center,
    End,
}

/// Where the menu goes on its surface, and the room left for the panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub align_x: Alignment,
    pub align_y: Alignment,
    panel: PanelAnchor,
    panel_margin: i32,
}

impl Default for Placement {
    fn default() -> Self {
        let panel = PanelAnchor::Bottom;
        let (align_x, align_y) = alignments(panel, PanelSection::Start);
        Placement {
            align_x,
            align_y,
            panel,
            panel_margin: 0,
        }
    }
}

/// Placement of the menu of the applet in the panel at `panel`, which is
/// `thickness` thick.
pub fn placement(panel: PanelAnchor, applet_id: &str, thickness: i32) -> Placement {
    let config = panel_config();
    let section = config.as_ref().map_or(PanelSection::Start, |config| {
        section_in(
            config.expand_to_edges,
            config.plugins_wings.as_ref(),
            applet_id,
        )
    });
    let (align_x, align_y) = alignments(panel, section);
    // Panels reserving their space are already clear of the surface.
    let panel_margin = match config {
        Some(config) if config.exclusive_zone => 0,
        Some(config) => thickness + i32::from(config.margin),
        None => thickness,
    };

    Placement {
        align_x,
        align_y,
        panel,
        panel_margin,
    }
}

/// Settings of the menu's layer surface, which fills the screen except for
/// the panel, with the menu placed on it by [`Placement`]. Clicks on the rest
/// of it close the menu.
///
/// It is anchored to all edges as the panel only tells a forwarded surface
/// to draw once the compositor gave it a size other than the panel asked
/// for, so a surface of a fixed size would never appear.
///
/// It gets the keyboard when mapped, and on demand so that clicking on the
/// panel or another surface takes it away, which closes it as well.
pub fn settings(id: Id, namespace: String, placement: &Placement) -> SctkLayerSurfaceSettings {
    let mut margin = IcedMargin::default();
    match placement.panel {
        PanelAnchor::Top => margin.top = placement.panel_margin,
        PanelAnchor::Bottom => margin.bottom = placement.panel_margin,
        PanelAnchor::Left => margin.left = placement.panel_margin,
        PanelAnchor::Right => margin.right = placement.panel_margin,
    }

    SctkLayerSurfaceSettings {
        id,
        namespace,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        anchor: Anchor::all(),
        margin,
        size: Some((None, None)),
        // Stays clear of the space panels reserve.
        exclusive_zone: 0,
        size_limits: Limits::NONE.min_width(1.0).min_height(1.0),
        ..Default::default()
    }
}

/// The configuration of the panel holding the applet.
fn panel_config() -> Option<CosmicPanelConfig> {
    let name = std::env::var("COSMIC_PANEL_NAME").ok()?;
    let config = CosmicPanelConfig::cosmic_config(&name).ok()?;
    Some(CosmicPanelConfig::get_entry(&config).unwrap_or_else(|(_errors, config)| config))
}

/// A panel not reaching the screen's edges is around its center, whichever
/// wing holds the applet.
fn section_in(
    expands_to_edges: bool,
    wings: Option<&(Vec<String>, Vec<String>)>,
    applet_id: &str,
) -> PanelSection {
    let Some((start, end)) = wings.filter(|_| expands_to_edges) else {
        return PanelSection::Center;
    };
    if start.iter().any(|id| id == applet_id) {
        PanelSection::Start
    } else if end.iter().any(|id| id == applet_id) {
        PanelSection::End
    } else {
        PanelSection::Center
    }
}

/// Horizontal and vertical alignment of the menu: against the panel, and
/// along it by the applet's section.
fn alignments(panel: PanelAnchor, section: PanelSection) -> (Alignment, Alignment) {
    let along = match section {
        PanelSection::Start => Alignment::Start,
        PanelSection::Center => Alignment::Center,
        PanelSection::End => Alignment::End,
    };
    match panel {
        PanelAnchor::Top => (along, Alignment::Start),
        PanelAnchor::Bottom => (along, Alignment::End),
        PanelAnchor::Left => (Alignment::Start, along),
        PanelAnchor::Right => (Alignment::End, along),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wings() -> (Vec<String>, Vec<String>) {
        (vec!["start".to_owned()], vec!["end".to_owned()])
    }

    #[test]
    fn section_follows_the_applets_wing() {
        let wings = wings();
        assert_eq!(section_in(true, Some(&wings), "start"), PanelSection::Start);
        assert_eq!(section_in(true, Some(&wings), "end"), PanelSection::End);
        assert_eq!(section_in(true, Some(&wings), "other"), PanelSection::Center);
    }

    #[test]
    fn panels_not_reaching_the_edges_center_the_menu() {
        assert_eq!(section_in(false, Some(&wings()), "start"), PanelSection::Center);
    }

    #[test]
    fn menu_sits_against_the_panel() {
        assert_eq!(
            alignments(PanelAnchor::Bottom, PanelSection::Start),
            (Alignment::Start, Alignment::End)
        );
        assert_eq!(
            alignments(PanelAnchor::Bottom, PanelSection::Center),
            (Alignment::Center, Alignment::End)
        );
        assert_eq!(
            alignments(PanelAnchor::Right, PanelSection::End),
            (Alignment::End, Alignment::End)
        );
    }

    #[test]
    fn surface_fills_the_screen_except_for_the_panel() {
        let placement = Placement {
            panel: PanelAnchor::Bottom,
            panel_margin: 60,
            ..Placement::default()
        };
        let settings = settings(Id::unique(), String::new(), &placement);

        assert_eq!(settings.anchor, Anchor::all());
        assert_eq!(settings.size, Some((None, None)));
        assert_eq!(settings.margin.bottom, 60);
        assert_eq!(settings.margin.top, 0);
    }
}
