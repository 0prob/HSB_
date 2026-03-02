use ethers::types::{Address, Bytes};

#[derive(Debug, Clone)]
pub struct Hop {
    pub target: Address,
    pub calldata: Bytes,
}

#[derive(Debug, Clone)]
pub struct EncodedRoute {
    pub targets: Vec<Address>,
    pub data: Vec<Bytes>,
}

#[derive(Debug, Clone)]
pub enum RouteKind {
    Linear,
    Triangular,
}
