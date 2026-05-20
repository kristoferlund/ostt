use crate::model::ModelSelector;

pub async fn handle_model() -> anyhow::Result<()> {
    ModelSelector::new()?.run().await
}
