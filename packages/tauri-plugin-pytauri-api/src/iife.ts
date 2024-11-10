import * as pytauri from "./index";

if ("__TAURI__" in window) {
    Object.defineProperty(window.__TAURI__, "pytauri", pytauri);
}
