use graphql_client::GraphQLQuery;
use serde::{Deserialize, Serialize};

/// GraphQL query for fetching all PairCreated events from HyperIndex.
/// This schema is intentionally minimal — HyperIndex will expand it as needed.
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema.graphql",
    query_path = "queries/all_pairs.graphql",
    response_derives = "Debug, Serialize, Deserialize"
)]
pub struct AllPairs;
