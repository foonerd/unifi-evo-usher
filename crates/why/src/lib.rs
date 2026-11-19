//! Why the usher refuses what it was asked.
//!
//! A refusal that can reach a screen is said three ways at once: by a
//! code, which stays what it is from release to release and is what a
//! screen finds its own words by, in whatever language it speaks; by the
//! values it speaks of, each under a name; and in words, in English, for
//! a log and for a caller that is no screen.
//!
//! A plugin's refusal travels as evo carries one: the code as the
//! refusal's subclass, the words as its message. evo's refusal of a
//! plugin has no place for values, so a plugin's values stay in its
//! words; the usher's own routes, which write their answers themselves,
//! carry them beside the code.

#![forbid(unsafe_code)]

use std::fmt;

use evo_plugin_sdk::contract::PluginError;
use evo_plugin_sdk::error_taxonomy::ErrorClass;
use serde_json::{Map, Value};

/// Why something asked was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Why {
    /// The code: lower-case words joined by `_`, the same from release to
    /// release for the same refusal.
    pub code: &'static str,
    /// The refusal in words, in English.
    pub words: String,
    /// The values the words speak of, each under a name.
    pub values: Map<String, Value>,
}

/// A refusal under a code, in words.
pub fn why(code: &'static str, words: impl Into<String>) -> Why {
    Why {
        code,
        words: words.into(),
        values: Map::new(),
    }
}

impl Why {
    /// The same refusal, with one more value it speaks of.
    #[must_use]
    pub fn with(mut self, name: &str, value: impl Into<Value>) -> Self {
        self.values.insert(name.to_string(), value.into());
        self
    }

    /// The refusal as a plugin gives it to the steward, where asking
    /// again as asked will not succeed.
    pub fn as_asked(self) -> PluginError {
        self.of_class(ErrorClass::ContractViolation)
    }

    /// The refusal as a plugin gives it to the steward, where asking
    /// again may succeed.
    pub fn for_now(self) -> PluginError {
        self.of_class(ErrorClass::Transient)
    }

    /// The refusal as a plugin gives it to the steward, under a class of
    /// evo's.
    pub fn of_class(self, class: ErrorClass) -> PluginError {
        PluginError::WithSubclass {
            class,
            subclass: self.code.to_string(),
            message: self.words,
        }
    }
}

impl fmt::Display for Why {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.words)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_reaches_the_steward_under_its_code_in_its_own_words() {
        let refused = why("no_such_type", "There is no voucher type Day.")
            .with("name", "Day");
        assert_eq!(refused.to_string(), "There is no voucher type Day.");
        assert_eq!(refused.values["name"], "Day");
        for (told, class) in [
            (refused.clone().as_asked(), ErrorClass::ContractViolation),
            (refused.clone().for_now(), ErrorClass::Transient),
        ] {
            let PluginError::WithSubclass {
                class: under,
                subclass,
                message,
            } = told
            else {
                panic!("not a refusal with a code");
            };
            assert_eq!(under, class);
            assert_eq!(subclass, "no_such_type");
            assert_eq!(message, "There is no voucher type Day.");
        }
    }
}
