//! Explicit acceptance child: validates an encrypted store against the backend.
fn main() {
    let result = (|| {
        let path = std::env::args_os().nth(1).ok_or("path_required")?;
        let service = nylra_desktop::auth::service::AuthService::from_environment(path.into())?;
        let view = service.status()?;
        Ok::<_, &'static str>(
            serde_json::json!({"state":view.state,"hasEnrollment":view.has_enrollment,
            "deviceId":view.principal.as_ref().map(|p|&p.device_id)}),
        )
    })();
    match result {
        Ok(v) => println!("{v}"),
        Err(code) => {
            eprintln!("{code}");
            std::process::exit(1);
        }
    }
}
