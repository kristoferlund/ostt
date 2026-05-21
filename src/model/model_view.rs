use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

use super::model_provider_view::ModelProviderChoice;
use super::UserQuit;

pub struct ModelView {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl ModelView {
    /// Creates the top-level model view and owns terminal cleanup for child views.
    pub fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        tracing::debug!("Model view started");
        loop {
            let choice = super::model_provider_view::run(&mut self.terminal).await?;
            let result = match choice {
                ModelProviderChoice::Quit => {
                    tracing::debug!("Model view exited from provider picker");
                    break;
                }
                ModelProviderChoice::Local => {
                    tracing::debug!("Opening local model view");
                    super::local_model_view::run(&mut self.terminal).await
                }
                ModelProviderChoice::Cloud => {
                    tracing::debug!("Opening cloud model view");
                    super::cloud_model_view::run(&mut self.terminal).await
                }
            };
            if let Err(e) = result {
                if e.downcast_ref::<UserQuit>().is_some() {
                    tracing::debug!("Model view exited via Ctrl+C");
                    break;
                }
                return Err(e);
            }
        }
        Ok(())
    }
}

impl Drop for ModelView {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
