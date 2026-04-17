pub mod detect;
pub mod types;

pub use detect::detect_installer_environment;
pub use types::{
    InstallerDependencyKind, InstallerDependencyName, InstallerDependencyState,
    InstallerDependencyStatus, InstallerEnvironment,
};
