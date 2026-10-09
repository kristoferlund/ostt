use ratcn::terminal::Session;

use super::UserQuit;

pub struct ModelView {
    session: Session,
}

impl ModelView {
    /// Own the adaptive terminal session for the model command.
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            session: crate::ui::session::open()?,
        })
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        tracing::debug!("Model view started");
        match super::local_model_view::run(&mut self.session).await {
            Err(error) if error.downcast_ref::<UserQuit>().is_some() => {
                tracing::debug!("Model view exited via Ctrl+C");
                Ok(())
            }
            result => result,
        }
    }
}
