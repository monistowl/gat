mod commands;
mod service;
mod state;

use commands::{
    compute_ptdf, detect_islands, export_network_json, get_batch_status, get_config,
    get_config_path, get_grid_summary, get_loaded_network, get_lodf_matrix, get_notebook_manifest,
    get_thermal_analysis, get_unified_config, get_ybus, init_notebook_workspace, list_cases,
    load_case, read_notebook, run_batch_job, run_n1_contingency, save_config, solve_dc_opf,
    solve_dc_power_flow, solve_power_flow, update_gui_config,
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
            get_loaded_network,
            solve_power_flow,
            solve_dc_power_flow,
            solve_dc_opf,
            run_n1_contingency,
            get_ybus,
            // Legacy config (backward compatibility)
            get_config,
            save_config,
            get_config_path,
            // New unified config
            get_unified_config,
            update_gui_config,
            // Notebooks
            get_notebook_manifest,
            read_notebook,
            init_notebook_workspace,
            // Batch jobs
            run_batch_job,
            get_batch_status,
            // Analysis
            compute_ptdf,
            get_grid_summary,
            detect_islands,
            get_lodf_matrix,
            get_thermal_analysis,
            export_network_json,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
