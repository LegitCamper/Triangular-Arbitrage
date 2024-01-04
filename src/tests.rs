#[cfg(test)]
mod tests {
    use crate::func::*;
    use crate::interface::*;
    use crate::websocket::*;

    #[tokio::test]
    async fn test_create_valid_pairs_catalog() {
        let pairs = BinanceInterface::new().get_pairs().await.unwrap();
        panic!("{:?}", create_valid_pairs_catalog(pairs).await.len());
    }
}
