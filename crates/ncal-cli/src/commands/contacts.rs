use ncal_api::endpoints::get_contacts;

use super::context::authenticated_client;
use crate::config::AppConfig;
use crate::output::print_contacts;
use crate::Cli;
use crate::CliError;

pub async fn run(cli: &Cli, config: &AppConfig) -> Result<(), CliError> {
    let client = authenticated_client(config)?;
    let rows = get_contacts(&client).await.map_err(CliError::Api)?;
    print_contacts(cli, &rows)
}
