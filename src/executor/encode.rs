use anyhow::Result;
use ethers::abi::Token;
use ethers::types::{Address, Bytes, U256};

/// Encode a Uniswap V2 swap call.
pub fn encode_uniswap_v2_swap(
    amount_in: U256,
    amount_out_min: U256,
    path: Vec<Address>,
    to: Address,
) -> Result<Bytes> {
    let func = "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)";
    let tokens = vec![
        Token::Uint(amount_in),
        Token::Uint(amount_out_min),
        Token::Array(path.into_iter().map(Token::Address).collect()),
        Token::Address(to),
        Token::Uint(U256::from(9999999999u64)),
    ];
    Ok(ethers::abi::encode(&tokens).into())
}

/// Encode a Uniswap V3 exactInputSingle call.
pub fn encode_uniswap_v3_swap(
    token_in: Address,
    token_out: Address,
    fee: u32,
    amount_in: U256,
    amount_out_min: U256,
    to: Address,
) -> Result<Bytes> {
    let func = "exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))";
    let params = Token::Tuple(vec![
        Token::Address(token_in),
        Token::Address(token_out),
        Token::Uint(U256::from(fee)),
        Token::Address(to),
        Token::Uint(U256::from(9999999999u64)),
        Token::Uint(amount_in),
        Token::Uint(amount_out_min),
        Token::Uint(U256::from(0)),
    ]);
    Ok(ethers::abi::encode(&[params]).into())
}

/// Encode a Curve exchange call.
pub fn encode_curve_exchange(
    i: i128,
    j: i128,
    dx: U256,
    min_dy: U256,
) -> Result<Bytes> {
    let tokens = vec![
        Token::Int(dx.into()),
        Token::Int(i.into()),
        Token::Int(j.into()),
        Token::Uint(min_dy),
    ];
    Ok(ethers::abi::encode(&tokens).into())
}

/// Encode a Balancer swap call.
pub fn encode_balancer_swap(
    pool_id: [u8; 32],
    token_in: Address,
    token_out: Address,
    amount: U256,
) -> Result<Bytes> {
    let tokens = vec![
        Token::FixedBytes(pool_id.to_vec()),
        Token::Address(token_in),
        Token::Address(token_out),
        Token::Uint(amount),
        Token::Uint(U256::zero()),
    ];
    Ok(ethers::abi::encode(&tokens).into())
}
