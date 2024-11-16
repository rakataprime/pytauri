import * as pytauri from "tauri-plugin-pytauri-api";

if ("__TAURI__" in window) {
    Object.defineProperty(window.__TAURI__, "pytauri", { value: pytauri });
}
