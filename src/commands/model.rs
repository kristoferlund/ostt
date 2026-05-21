use crate::model::ModelView;

pub async fn handle_model() -> anyhow::Result<()> {
    ModelView::new()?.run().await
}
