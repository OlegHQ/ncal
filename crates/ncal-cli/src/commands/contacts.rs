use ncal_api::endpoints::get_contacts;

use super::auth::{build_client, resolve_credentials};
use crate::config::AppConfig;
use crate::output::print_contacts;
use crate::Cli;
use crate::CliError;

pub async fn run(cli: &Cli, config: &AppConfig) -> Result<(), CliError> {
    let client = build_client(config, resolve_credentials(config)?)?;
    let rows = get_contacts(&client).await.map_err(CliError::Api)?;
    print_contacts(cli, &rows)
}
