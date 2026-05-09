//! UOF Plugin SDK — manifest schema, template definition, and packaging tools.

pub mod digest;
pub mod error;
pub mod manifest;
pub mod packager;
pub mod template;

pub use digest::digest_bytes;
pub use error::{PluginError, PluginResult};
pub use manifest::{PluginManifest, ProbeEntry, ResourceBudget};
pub use packager::PluginPackager;
pub use template::{TargetSelector, Template, TemplateBinding};
