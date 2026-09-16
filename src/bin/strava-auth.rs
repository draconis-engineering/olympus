//! Helper binary to connect Olympus to Strava (Phase 12).
//! Run: STRAVA_CLIENT_ID=xxx STRAVA_CLIENT_SECRET=yyy cargo run --bin strava-auth
//!
//! It generates a PKCE verifier/challenge, prints the authorize URL, and
//! exchanges the pasted `code` for a token saved to `data/user/strava.json`.

#[path = "../strava.rs"]
mod strava;

use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = std::env::var("STRAVA_CLIENT_ID")
        .map_err(|_| "STRAVA_CLIENT_ID not set — create an app at https://www.strava.com/settings/api")?;
    let client_secret = std::env::var("STRAVA_CLIENT_SECRET")
        .map_err(|_| "STRAVA_CLIENT_SECRET not set")?;
    let redirect_uri = std::env::var("OLYMPUS_STRAVA_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:8080/callback".to_string());

    // Generate PKCE pair.
    let (verifier, challenge) = strava::generate_pkce();
    let state = uuid::Uuid::new_v4().to_string();

    let url = strava::build_auth_url(
        &client_id,
        &redirect_uri,
        "activity:write",
        &state,
        &challenge,
    );

    println!("=== Olympus — Strava Connect ===\n");
    println!("1. Open this URL in your browser:\n\n   {url}\n");
    println!("2. Authorize Olympus when Strava asks.");
    println!("3. You'll be redirected to {redirect_uri}?code=...&state=...");
    println!("   Copy the `code` parameter from the address bar.\n");
    println!("   (If the localhost page fails to load, that's expected — just copy the code.)\n");
    print!("Paste the `code` here: ");
    io::stdout().flush()?;

    let mut code = String::new();
    io::stdin().read_line(&mut code)?;
    let code = code.trim().to_string();
    if code.is_empty() {
        eprintln!("No code pasted — aborting.");
        std::process::exit(1);
    }

    println!("\nExchanging code for token...");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let token = rt.block_on(strava::exchange_code(
        &client_id,
        &client_secret,
        &code,
        &verifier,
    ))?;

    strava::save_token(&token)?;
    println!(
        "\nSaved token to {} (expires at {})",
        strava::TOKEN_PATH,
        token.expires_at
    );
    println!("You can now run Olympus — rides will auto-upload to Strava.");
    println!("Re-run this helper any time to re-authenticate.");
    Ok(())
}
