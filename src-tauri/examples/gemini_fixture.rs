//! Writes only an explicitly supplied isolated test directory, using fake credentials.
use tokentoapi_connect_lib::{
    adapter,
    models::{self, Client},
    storage::Paths,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let home = std::path::Path::new(&args[1]);
    assert!(home.is_absolute());
    adapter::apply(
        &Paths::for_home(home),
        models::ValidatedInput {
            client: Client::Gemini,
            api_key: "fixture-only-key".into(),
            root: args[2].clone(),
            model: "gemini-2.5-pro".into(),
            selected_models: vec!["gemini-2.5-pro".into()],
            catalog: None,
            options: Default::default(),
        },
    )
    .unwrap();
}
