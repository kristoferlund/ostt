use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match ostt::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // As Rust prints an error returned from main, with its causes.
            eprintln!("Error: {err:?}");
            if err.is::<ostt::process::ProcessingFailed>() {
                ExitCode::from(ostt::process::PROCESSING_FAILED_EXIT)
            } else {
                ExitCode::FAILURE
            }
        }
    }
}
