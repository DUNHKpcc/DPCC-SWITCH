pub mod detect;
pub mod install;
pub mod types;

pub use detect::detect_installer_environment;
pub use install::{build_install_plan, get_manual_install_commands, ManualInstallCommandGroup};
pub use types::{
    InstallerDependencyKind, InstallerDependencyName, InstallerDependencyState,
    InstallerDependencyStatus, InstallerEnvironment,
};
