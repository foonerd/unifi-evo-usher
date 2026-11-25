//! The printers on this machine's USB ports, and what each says it is.
//!
//! The kernel's `usblp` driver gives every USB printer a device,
//! `/dev/usb/lp0` and up, and keeps beside it what the printer told the
//! machine when it was plugged in: its IEEE 1284 device ID, a line of
//! `KEY:value;` pairs with its maker, its model and the command set it
//! takes, and the serial number of the USB device. That is read here
//! from `/sys`, so a printer is known for what it is without a byte
//! being sent to it.
//!
//! A device's number is the order of plugging in, and changes: a printer
//! is kept by what it says it is, and found again by that
//! ([`Found::is`]).

use std::fs;
use std::path::{Path, PathBuf};

use crate::model::Language;

/// Where the kernel lists USB printer devices.
const LISTED: &str = "/sys/class/usbmisc";

/// Where their devices are.
const DEVICES: &str = "/dev/usb";

/// A printer found on a USB port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Its device.
    pub device: PathBuf,
    /// Its maker, as it says; empty where it does not.
    pub maker: String,
    /// Its model, as it says; empty where it does not.
    pub model: String,
    /// The command set it says it takes; empty where it does not.
    pub commands: String,
    /// The serial number of the USB device; empty where it has none.
    pub serial: String,
}

impl Found {
    /// The printer in a few words: its maker and model, or its device
    /// where it says neither.
    pub fn words(&self) -> String {
        let named = [self.maker.as_str(), self.model.as_str()]
            .iter()
            .filter(|part| !part.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        if named.is_empty() {
            self.device.display().to_string()
        } else {
            named
        }
    }

    /// The language the printer speaks, where what it says of itself
    /// settles that: a Brother QL by its maker and model, a printer
    /// that takes ZPL or ESC/POS by the command set it says it takes.
    pub fn language(&self) -> Option<Language> {
        let maker = self.maker.to_ascii_lowercase();
        let model = self.model.to_ascii_uppercase();
        if maker.contains("brother") && model.starts_with("QL-") {
            return Some(Language::BrotherQl);
        }
        if self.commands.to_ascii_uppercase().contains("ZPL") {
            return Some(Language::Zpl);
        }
        if self.commands.to_ascii_uppercase().contains("ESC/POS") {
            return Some(Language::EscPos);
        }
        None
    }

    /// Whether this is the printer that said `maker`, `model` and
    /// `serial` of itself. A printer with a serial number is known by
    /// it; one with none by its maker and model, which two of a kind
    /// share.
    pub fn is(&self, maker: &str, model: &str, serial: &str) -> bool {
        self.maker == maker && self.model == model && self.serial == serial
    }
}

/// The value of `key`, or of its long form `long`, in an IEEE 1284
/// device ID.
fn value(id: &str, key: &str, long: &str) -> String {
    id.split(';')
        .filter_map(|pair| pair.split_once(':'))
        .find(|(name, _)| {
            let name = name.trim();
            name.eq_ignore_ascii_case(key) || name.eq_ignore_ascii_case(long)
        })
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_default()
}

/// A file's text with the white space about it taken off; empty where
/// it cannot be read.
fn text(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
}

/// The printers listed under `listed` whose devices are under
/// `devices`, in the order of their devices.
pub fn found_under(listed: &Path, devices: &Path) -> Vec<Found> {
    let Ok(entries) = fs::read_dir(listed) else {
        return Vec::new();
    };
    let mut found: Vec<(u32, Found)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let number: u32 = name.strip_prefix("lp")?.parse().ok()?;
            let device = devices.join(&name);
            if !device.exists() {
                return None;
            }
            // The entry is the printer's interface on the USB device;
            // the device itself is one above it.
            let interface = entry.path().join("device");
            let id = text(&interface.join("ieee1284_id"));
            let of_device = |file: &str| text(&interface.join("..").join(file));
            let mut maker = value(&id, "MFG", "MANUFACTURER");
            if maker.is_empty() {
                maker = of_device("manufacturer");
            }
            let mut model = value(&id, "MDL", "MODEL");
            if model.is_empty() {
                model = of_device("product");
            }
            Some((
                number,
                Found {
                    device,
                    maker,
                    model,
                    commands: value(&id, "CMD", "COMMAND SET"),
                    serial: of_device("serial"),
                },
            ))
        })
        .collect();
    found.sort_by_key(|(number, _)| *number);
    found.into_iter().map(|(_, found)| found).collect()
}

/// The printers on this machine's USB ports.
pub fn found() -> Vec<Found> {
    found_under(Path::new(LISTED), Path::new(DEVICES))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A printer as the kernel lists it: its entry, its interface with
    /// the device ID, the USB device above it, and its device.
    fn plugged(
        root: &Path,
        number: u32,
        id: Option<&str>,
        of_device: &[(&str, &str)],
        with_device: bool,
    ) {
        let usb = root.join(format!("usb{number}"));
        let interface = usb.join("interface");
        fs::create_dir_all(&interface).unwrap();
        if let Some(id) = id {
            fs::write(interface.join("ieee1284_id"), format!("{id}\n"))
                .unwrap();
        }
        for (file, value) in of_device {
            fs::write(usb.join(file), format!("{value}\n")).unwrap();
        }
        let entry = root.join("listed").join(format!("lp{number}"));
        fs::create_dir_all(&entry).unwrap();
        std::os::unix::fs::symlink(&interface, entry.join("device")).unwrap();
        let devices = root.join("devices");
        fs::create_dir_all(&devices).unwrap();
        if with_device {
            fs::write(devices.join(format!("lp{number}")), b"").unwrap();
        }
    }

    #[test]
    fn a_printer_is_known_by_what_it_told_the_machine() {
        let root = tempfile::tempdir().unwrap();
        plugged(
            root.path(),
            1,
            Some("MFG:Brother;CMD:PT-CBP;MDL:QL-820NWB;CLS:PRINTER;"),
            &[("serial", "000A1B2C3D")],
            true,
        );
        // One that spells its keys out, and has no serial number.
        plugged(
            root.path(),
            0,
            Some(
                "MANUFACTURER:Sample Maker ;COMMAND SET:XYZ;\
                 MODEL:Label 4;CLASS:PRINTER;",
            ),
            &[],
            true,
        );
        // One with no device ID: the USB device's own words stand.
        plugged(
            root.path(),
            2,
            None,
            &[("manufacturer", "Other Maker"), ("product", "Ticket 80")],
            true,
        );
        // One that is listed and has no device here, as in a container
        // that was not given it.
        plugged(root.path(), 3, Some("MFG:Brother;MDL:QL-800;"), &[], false);
        // One that takes ZPL, with the device ID Zebra's programming
        // guide prints (p.1026).
        plugged(
            root.path(),
            4,
            Some(
                "MANUFACTURER:Zebra Technologies ;COMMAND SET:ZPL;\
                 MODEL:ZTC ZT220-200dpi ZPL;CLASS:PRINTER;OPTIONS:XML;",
            ),
            &[("serial", "S4MPL3")],
            true,
        );

        let found = found_under(
            &root.path().join("listed"),
            &root.path().join("devices"),
        );
        let words: Vec<String> = found.iter().map(Found::words).collect();
        assert_eq!(
            words,
            [
                "Sample Maker Label 4",
                "Brother QL-820NWB",
                "Other Maker Ticket 80",
                "Zebra Technologies ZTC ZT220-200dpi ZPL"
            ]
        );
        assert_eq!(found[0].commands, "XYZ");
        assert_eq!(found[0].language(), None);
        assert_eq!(found[1].device, root.path().join("devices").join("lp1"));
        assert_eq!(found[1].commands, "PT-CBP");
        assert_eq!(found[1].language(), Some(Language::BrotherQl));
        assert!(found[1].is("Brother", "QL-820NWB", "000A1B2C3D"));
        assert!(!found[1].is("Brother", "QL-820NWB", ""));
        assert_eq!(found[2].language(), None);
        assert_eq!(found[3].commands, "ZPL");
        assert_eq!(found[3].language(), Some(Language::Zpl));
    }

    #[test]
    fn a_printer_that_says_it_takes_zpl_speaks_it() {
        let says = |maker: &str, model: &str, commands: &str| {
            Found {
                device: PathBuf::from("/dev/usb/lp0"),
                maker: maker.to_string(),
                model: model.to_string(),
                commands: commands.to_string(),
                serial: String::new(),
            }
            .language()
        };
        assert_eq!(says("Sample Maker", "Label 4", "ZPL"), Some(Language::Zpl));
        // Among other command sets, and in any case.
        assert_eq!(
            says("Sample Maker", "Label 4", "EPL2,zpl2,XML"),
            Some(Language::Zpl)
        );
        // A printer of another maker that takes it as an emulation.
        assert_eq!(says("Brother", "TD-4420TN", "ZPL"), Some(Language::Zpl));
        // What a printer is named does not say what it takes.
        assert_eq!(says("Sample Maker", "ZPL Label 4", "XYZ"), None);
        assert_eq!(says("Sample Maker", "ZPL Label 4", ""), None);
        // A Brother QL is one by its name, whatever it says it takes.
        assert_eq!(
            says("Brother", "QL-820NWB", "PT-CBP"),
            Some(Language::BrotherQl)
        );
    }

    #[test]
    fn a_printer_that_says_it_takes_escpos_speaks_it() {
        let says = |maker: &str, model: &str, commands: &str| {
            Found {
                device: PathBuf::from("/dev/usb/lp0"),
                maker: maker.to_string(),
                model: model.to_string(),
                commands: commands.to_string(),
                serial: String::new(),
            }
            .language()
        };
        assert_eq!(
            says("Sample Maker", "Ticket 80", "ESC/POS"),
            Some(Language::EscPos)
        );
        // Among other command sets, and in any case.
        assert_eq!(
            says("Sample Maker", "Ticket 58", "XYZ,esc/pos"),
            Some(Language::EscPos)
        );
        // What a printer is named does not say what it takes.
        assert_eq!(says("Sample Maker", "ESC/POS Ticket", "XYZ"), None);
        assert_eq!(says("Sample Maker", "ESC/POS Ticket", ""), None);
        // A printer that says it takes both is settled by the rule
        // that stands first.
        assert_eq!(
            says("Sample Maker", "Label 4", "ZPL,ESC/POS"),
            Some(Language::Zpl)
        );
    }

    #[test]
    fn a_machine_with_no_usb_printers_has_none() {
        let root = tempfile::tempdir().unwrap();
        assert!(
            found_under(&root.path().join("listed"), root.path()).is_empty()
        );
    }

    #[test]
    fn a_printer_that_says_nothing_of_itself_is_its_device() {
        let found = Found {
            device: PathBuf::from("/dev/usb/lp4"),
            maker: String::new(),
            model: String::new(),
            commands: String::new(),
            serial: String::new(),
        };
        assert_eq!(found.words(), "/dev/usb/lp4");
        assert_eq!(found.language(), None);
    }
}
