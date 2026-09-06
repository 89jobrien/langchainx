use langchainx::vectorstore::VecStoreOptions;
use langchainx_testsuite::{
    contracts::vectorstore::assert_vectorstore_contract, fakes::InMemoryVectorStore,
};

#[tokio::test]
async fn in_memory_vectorstore_satisfies_contract() {
    let store = InMemoryVectorStore::new();
    assert_vectorstore_contract(&store, &VecStoreOptions::default()).await;
    assert_eq!(store.documents().await.len(), 2);
}
