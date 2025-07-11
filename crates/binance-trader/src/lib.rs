pub mod api;

#[cfg(test)]
mod tests {
    use crate::*;
    use anyhow::Result;
    #[tokio::test]
    async fn test_order_test() -> Result<()> {
        api::order_test().await?;
        Ok(())
    }
}
