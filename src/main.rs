#[tokio::main]
async fn main() -> anyhow::Result<()> {
    datastar_matrix::run().await
}
