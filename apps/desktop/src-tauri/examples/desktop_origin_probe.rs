//! Temporary packaged-origin measurement; removed after its evidence is committed.
use std::collections::HashMap;
use tauri::{
    utils::config::{Csp, CspDirectiveSources},
    WebviewUrl, WebviewWindowBuilder,
};

#[tauri::command]
fn hide_main(window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("main_required".into());
    }
    window.hide().map_err(|_| "hide_failed".into())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let collector = args.get(1).expect("collector origin").clone();
    let core = args.get(2).expect("Core origin").clone();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![hide_main])
        .setup(move |app| {
            for (label, page) in [
                ("main", "index.html"),
                ("companion", "origin-companion.html"),
            ] {
                let script = include_str!("desktop_origin_probe.js")
                    .replace("__COLLECTOR__", &serde_json::to_string(&collector)?)
                    .replace("__CORE__", &serde_json::to_string(&core)?)
                    .replace("__LABEL__", &serde_json::to_string(label)?);
                let collector = collector.clone();
                let core = core.clone();
                let is_main = label == "main";
                WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
                    .title(format!("AYLUL origin probe {label}"))
                    .inner_size(if is_main { 1000.0 } else { 240.0 }, 700.0)
                    .initialization_script(&script)
                    .on_web_resource_request(move |_request, response| {
                        if let Some(header) =
                            response.headers_mut().get_mut("Content-Security-Policy")
                        {
                            let mut policy: HashMap<String, CspDirectiveSources> =
                                Csp::Policy(header.to_str().unwrap().to_owned()).into();
                            let connect = policy.entry("connect-src".into()).or_default();
                            connect.push(collector.clone());
                            if is_main {
                                connect.push(core.clone());
                            }
                            policy
                                .entry("style-src-attr".into())
                                .or_default()
                                .push("'unsafe-inline'");
                            let policy = Csp::from(policy).to_string();
                            *header = policy.parse().unwrap();
                            eprintln!("probe.csp.{label}={policy}");
                        }
                    })
                    .build()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("origin probe runtime");
}
