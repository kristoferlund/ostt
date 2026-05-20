use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};

use super::model_provider_view::ModelProviderChoice;
use super::UserQuit;

pub struct ModelView {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl ModelView {
    pub fn new() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            let choice = super::model_provider_view::choose_model_provider(&mut self.terminal).await?;
            let result = match choice {
                ModelProviderChoice::Quit => break,
                ModelProviderChoice::Local => {
                    super::local::handle_local_models_with_terminal(&mut self.terminal).await
                }
                ModelProviderChoice::Cloud => {
                    super::cloud_model_view::run_cloud_model_selector(&mut self.terminal).await
                }
            };
            if let Err(e) = result {
                if e.downcast_ref::<UserQuit>().is_some() {
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
