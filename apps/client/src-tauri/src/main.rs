// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    // The wrist panel is this same binary, started by the app with `--overlay`.
    if let Some(code) = toolkitosc_lib::run_overlay() {
        return code;
    }
    // An MCP client (Claude Desktop) talking to the running app over stdio.
    if let Some(code) = toolkitosc_lib::run_mcp() {
        return code;
    }
    toolkitosc_lib::run();
    std::process::ExitCode::SUCCESS
}
