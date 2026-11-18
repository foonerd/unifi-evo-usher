//! What the usher puts on a screen, as the steward is told it: a UI shelf
//! for each shelf of the catalogue that has a face, and the widget kinds
//! drawn on them.
//!
//! A UI shelf carries the name of its catalogue shelf and names the widget
//! its plugin is drawn with, so whichever plugin stocks the catalogue
//! shelf is put on the screen by the steward, and need declare nothing
//! itself. A plugin that stands with more than that one widget, or on the
//! UI shelf of a second shelf it stocks, says so in its manifest.
//! The widgets themselves are not here: they are the widget pack
//! `org.evoframework.usher.widgets`, which the steward admits from the
//! directory it serves.

use std::collections::BTreeMap;

use evo_plugin_sdk::ui::{
    AcceptedWidgets, ShelfCardinality, ShelfContract, ShelfLayout, ShelfOrder,
    UiSize, WidgetKindEnvelope,
};

/// The widget a guest is handed a voucher with.
pub const COUNTER: &str = "usher.counter";

/// The widget the voucher types on offer are kept with.
pub const TYPES: &str = "usher.types";

/// The widget a controller is set and looked at with.
pub const CONTROLLER: &str = "usher.controller";

/// The widget that shows the guest networks a voucher opens.
pub const NETWORKS: &str = "usher.networks";

/// The widget the controller's vouchers are looked at and revoked with.
pub const VOUCHERS: &str = "usher.vouchers";

/// The widget the reserve of vouchers is looked at with.
pub const RESERVE: &str = "usher.reserve";

/// The widget pack's own word on the sizes of its kinds.
const ENVELOPES: &str = include_str!(
    "../../../ui/org.evoframework.usher.widgets/size_envelopes.toml"
);

fn shelf(
    id: &str,
    label: &str,
    widgets: &[&str],
    sizes: &[UiSize],
) -> ShelfContract {
    ShelfContract {
        id: id.to_string(),
        label: Some(label.to_string()),
        // A shelf of the catalogue is stocked by one plugin at most; on
        // the screen that plugin may stand with more than one widget.
        cardinality: ShelfCardinality::AnyToMany,
        accepts_widgets: AcceptedWidgets::Allowed(
            widgets.iter().map(|widget| widget.to_string()).collect(),
        ),
        accepts_sizes: sizes.to_vec(),
        layout: ShelfLayout::Grid,
        order_by: ShelfOrder::ManifestDeclaration,
        // What a plugin that says nothing of its face is drawn with.
        default_widget: widgets.first().map(|widget| widget.to_string()),
        schema_version: 1,
        min_compatible_version: None,
    }
}

/// The usher's UI shelves.
pub fn shelves() -> Vec<ShelfContract> {
    vec![
        // What is done most stands first: a guest is handed a voucher.
        shelf(
            "issue.counter",
            "Hand out",
            &[COUNTER, TYPES],
            &[UiSize::Half, UiSize::Full],
        ),
        shelf(
            "access.controller",
            "Controller",
            &[CONTROLLER, VOUCHERS],
            &[UiSize::Half, UiSize::Full],
        ),
        shelf(
            "networks.directory",
            "Guest networks",
            &[NETWORKS],
            &[UiSize::Half, UiSize::Full],
        ),
        shelf(
            "stock.reserve",
            "Reserve",
            &[RESERVE],
            &[UiSize::Third, UiSize::Half, UiSize::Full],
        ),
    ]
}

/// The usher's widget kinds, as its widget pack declares them.
pub fn kinds() -> Vec<WidgetKindEnvelope> {
    toml::from_str::<BTreeMap<String, WidgetKindEnvelope>>(ENVELOPES)
        .expect("the widget pack's own size envelopes parse")
        .into_values()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shelf is drawn with a kind the pack provides, at a size the
    /// kind can be drawn at.
    #[test]
    fn every_shelf_names_a_kind_of_the_pack_at_a_size_it_has() {
        let kinds = kinds();
        for shelf in shelves() {
            let widget = shelf.default_widget.clone().unwrap();
            let kind = kinds
                .iter()
                .find(|k| k.id == widget)
                .unwrap_or_else(|| panic!("{widget} is not in the pack"));
            assert!(
                shelf.accepts_sizes.contains(&kind.ideal_size),
                "{}: the shelf does not take {widget} at its own size",
                shelf.id
            );
        }
    }

    /// The kinds are the ones the pack's manifest says it provides.
    #[test]
    fn the_kinds_are_the_ones_the_pack_provides() {
        let manifest: toml::Table = include_str!(
            "../../../ui/org.evoframework.usher.widgets/manifest.toml"
        )
        .parse()
        .unwrap();
        let mut provided: Vec<String> = manifest["widgets"]["provides"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap().to_string())
            .collect();
        provided.sort();
        let mut declared: Vec<String> =
            kinds().into_iter().map(|k| k.id).collect();
        declared.sort();
        assert_eq!(declared, provided);
        assert_eq!(
            declared,
            [CONTROLLER, COUNTER, NETWORKS, RESERVE, TYPES, VOUCHERS]
        );
    }

    /// The pack and the shell carry the workspace's version.
    #[test]
    fn the_pack_and_the_shell_carry_the_version() {
        for manifest in [
            include_str!(
                "../../../ui/org.evoframework.usher.widgets/manifest.toml"
            ),
            include_str!(
                "../../../ui/org.evoframework.usher.shell/manifest.toml"
            ),
        ] {
            let manifest: toml::Table = manifest.parse().unwrap();
            assert_eq!(
                manifest["plugin"]["version"].as_str().unwrap(),
                env!("CARGO_PKG_VERSION")
            );
        }
    }
}
