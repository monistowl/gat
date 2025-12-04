mod commands;
mod state;

use commands::{
    get_config, get_config_path, get_notebook_manifest, get_ybus, init_notebook_workspace,
    list_cases, load_case, read_notebook, run_n1_contingency, save_config, solve_dc_power_flow,
    solve_power_flow,
};
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_cases,
            load_case,
            solve_power_flow,
            solve_dc_power_flow,
            run_n1_contingency,
            get_ybus,
            get_config,
            save_config,
            get_config_path,
            get_notebook_manifest,
            read_notebook,
            init_notebook_workspace,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
