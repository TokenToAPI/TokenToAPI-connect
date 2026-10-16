use serde::Deserialize;
use std::{env, fs, path::PathBuf};
use tokentoapi_connect_lib::{adapter, models, restore, storage::Paths};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    home: PathBuf,
    root: String,
    client: models::Client,
    selected_models: Vec<String>,
    default_model: String,
    #[serde(default)]
    restore: bool,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let path = env::args().nth(1).ok_or("fixture JSON path required")?;
    let fixture: Fixture =
        serde_json::from_str(&fs::read_to_string(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if !fixture.home.is_absolute()
        || !fixture.home.join(".tokentoapi-codex-fixture").is_file()
        || !fixture.root.starts_with("http://127.0.0.1:")
        || !fixture.client.is_codex()
    {
        return Err("Only marked temporary homes and a loopback gateway are allowed".into());
    }
    let paths = Paths::for_home(&fixture.home);
    if fixture.restore {
        let point = restore::list(&paths)
            .into_iter()
            .find(|point| point.client == models::Client::Codex)
            .ok_or("restore point missing")?;
        let result = restore::apply(
            &paths,
            fixture.client,
            point
                .review_id
                .as_deref()
                .ok_or("restore preview missing")?,
            false,
        )?;
        println!("{}", serde_json::to_string(&result).unwrap());
        return Ok(());
    }

    let discovered = models::discover(models::DiscoveryInput {
        client: fixture.client,
        api_key: "sk-tokentoapi-fixture-only".into(),
        base_url: fixture.root,
    })
    .await?;
    let session = models::DiscoverySession::new(discovered);
    let input = session.prepare(&models::SelectionInput {
        discovery_id: session.response().discovery_id,
        selected_models: fixture.selected_models,
        default_model: fixture.default_model,
    })?;
    let result = adapter::apply(&paths, input)?;
    println!("{}", serde_json::to_string(&result).unwrap());
    Ok(())
}
