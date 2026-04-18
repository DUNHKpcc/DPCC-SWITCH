#![allow(non_snake_case)]

use tauri::AppHandle;

#[tauri::command]
pub async fn detect_installer_environment(
) -> Result<crate::services::installer::types::InstallerEnvironment, String> {
    Ok(crate::services::installer::detect::detect_installer_environment())
}

#[tauri::command]
pub async fn install_missing_dependencies(
    app: AppHandle,
) -> Result<crate::services::installer::install::InstallerRunResult, String> {
    crate::services::installer::install::install_missing_dependencies(&app).await
}

#[tauri::command]
pub async fn install_selected_dependencies(
    app: AppHandle,
    dependencies: Vec<crate::services::installer::types::InstallerDependencyName>,
) -> Result<crate::services::installer::install::InstallerRunResult, String> {
    crate::services::installer::install::install_selected_dependencies(&app, &dependencies).await
}

#[tauri::command]
pub async fn get_manual_install_commands(
) -> Result<Vec<crate::services::installer::install::ManualInstallCommandGroup>, String> {
    Ok(crate::services::installer::install::get_manual_install_commands(
        std::env::consts::OS,
    ))
}
