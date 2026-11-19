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

/// The widget the site's name, logo and time zone are set with.
pub const SITE: &str = "usher.site";

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
        shelf(
            "site.settings",
            "Site",
            &[SITE],
            &[UiSize::Half, UiSize::Full],
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
            [CONTROLLER, COUNTER, NETWORKS, RESERVE, SITE, TYPES, VOUCHERS]
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
            THEME,
        ] {
            let manifest: toml::Table = manifest.parse().unwrap();
            assert_eq!(
                manifest["plugin"]["version"].as_str().unwrap(),
                env!("CARGO_PKG_VERSION")
            );
        }
    }

    /// The usher's theme: its manifest, and the stylesheet a shell loads.
    const THEME: &str = include_str!(
        "../../../ui/org.evoframework.usher.theme.frost/manifest.toml"
    );
    const THEME_CSS: &str = include_str!(
        "../../../ui/org.evoframework.usher.theme.frost/theme.css"
    );

    /// The tokens of the manifest: those of every variant, and each
    /// variant's own.
    fn tokens() -> (BTreeMap<String, String>, [BTreeMap<String, String>; 2]) {
        let manifest: toml::Table = THEME.parse().unwrap();
        let tokens = manifest["theme"]["tokens"].as_table().unwrap();
        let written = |table: &toml::Table| -> BTreeMap<String, String> {
            table
                .iter()
                .filter_map(|(name, value)| {
                    Some((name.clone(), value.as_str()?.to_string()))
                })
                .collect()
        };
        let variant =
            |name: &str| written(tokens[name].as_table().expect("a variant"));
        (written(tokens), [variant("dark"), variant("light")])
    }

    /// The custom properties of the stylesheet: those it sets for every
    /// reader, and those it sets for one who asks for a light page.
    fn properties() -> [BTreeMap<String, String>; 2] {
        let mut sets = [BTreeMap::new(), BTreeMap::new()];
        let mut light = false;
        for line in THEME_CSS.lines().map(str::trim) {
            if line.starts_with("@media") {
                assert_eq!(line, "@media (prefers-color-scheme: light) {");
                light = true;
            } else if let Some(property) = line.strip_prefix("--") {
                let (name, value) = property
                    .strip_suffix(';')
                    .and_then(|property| property.split_once(": "))
                    .unwrap_or_else(|| panic!("not a property: {line}"));
                sets[usize::from(light)]
                    .insert(name.to_string(), value.to_string());
            }
        }
        sets
    }

    /// A shell loads the stylesheet; the steward is told the manifest.
    /// The two say the same.
    #[test]
    fn the_themes_stylesheet_sets_the_tokens_of_its_manifest() {
        let (shared, [dark, light]) = tokens();
        let [for_all, for_light] = properties();
        let mut unless_light = shared;
        unless_light.extend(dark);
        assert_eq!(for_all, unless_light, "dark unless light is asked for");
        assert_eq!(for_light, light);
        let manifest: toml::Table = THEME.parse().unwrap();
        assert_eq!(manifest["theme"]["variants"].as_array().unwrap().len(), 2);
        assert_eq!(
            manifest["theme"]["assets"]["stylesheet"].as_str(),
            Some("theme.css")
        );
    }

    /// How light a colour is to the eye, as WCAG reckons it.
    fn luminance(colour: &str) -> f64 {
        let hex = colour
            .strip_prefix('#')
            .filter(|hex| hex.len() == 6)
            .unwrap_or_else(|| panic!("{colour} is not #rrggbb"));
        let channel = |at: usize| {
            let part =
                f64::from(u8::from_str_radix(&hex[at..at + 2], 16).unwrap())
                    / 255.0;
            if part <= 0.04045 {
                part / 12.92
            } else {
                ((part + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4)
    }

    fn contrast(one: &str, other: &str) -> f64 {
        let (one, other) = (luminance(one), luminance(other));
        (one.max(other) + 0.05) / (one.min(other) + 0.05)
    }

    /// The theme says it is AAA: what is written stands at 7 to 1 or more
    /// against what it is written on. evo takes a theme's word for it, so
    /// the word is held here. Every colour that is written in is held
    /// against every surface, the lightest and darkest the backdrop gets
    /// among them, since a pane lets the backdrop through.
    #[test]
    fn the_theme_holds_the_contrast_it_declares() {
        let manifest: toml::Table = THEME.parse().unwrap();
        assert_eq!(
            manifest["theme"]["compliance"]["wcag_conformance"].as_str(),
            Some("AAA")
        );
        let (_, variants) = tokens();
        for (variant, tokens) in ["dark", "light"].iter().zip(&variants) {
            for written in [
                "foreground",
                "muted-foreground",
                "primary",
                "destructive",
                "status-ok",
                "status-warn",
            ] {
                for surface in ["background", "backdrop-glow", "card", "input"]
                {
                    let ratio = contrast(&tokens[written], &tokens[surface]);
                    assert!(
                        ratio >= 7.0,
                        "{variant}: {written} on {surface} is {ratio:.2} to 1"
                    );
                }
            }
            let ratio =
                contrast(&tokens["primary-foreground"], &tokens["primary"]);
            assert!(
                ratio >= 7.0,
                "{variant}: a button's words are {ratio:.2} to 1"
            );
        }
    }
}
