// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title ArbExecutor
/// @notice Minimal, chain-agnostic executor for multi-hop arbitrage.
/// @dev The Rust engine builds calldata for each hop; this contract only executes.
contract ArbExecutor {
    address public owner;

    modifier onlyOwner() {
        require(msg.sender == owner, "not owner");
        _;
    }

    constructor() {
        owner = msg.sender;
    }

    /// @notice Execute a sequence of arbitrary calls (DEX swaps).
    /// @param targets The DEX pool or router addresses.
    /// @param data The calldata for each hop.
    /// @param minReturn Minimum acceptable output (slippage protection).
    function execute(
        address[] calldata targets,
        bytes[] calldata data,
        uint256 minReturn
    ) external onlyOwner returns (uint256) {
        require(targets.length == data.length, "length mismatch");

        uint256 startBalance = address(this).balance;

        for (uint256 i = 0; i < targets.length; i++) {
            (bool ok, bytes memory res) = targets[i].call(data[i]);
            require(ok, string(abi.encodePacked("call failed: ", _getRevertMsg(res))));
        }

        uint256 endBalance = address(this).balance;
        uint256 profit = endBalance - startBalance;

        require(profit >= minReturn, "slippage");

        // Return profit to owner
        payable(owner).transfer(profit);

        return profit;
    }

    /// @notice Receive ETH from WETH unwraps or DEX callbacks.
    receive() external payable {}

    /// @dev Extract revert reason from failed call.
    function _getRevertMsg(bytes memory returnData) internal pure returns (string memory) {
        if (returnData.length < 68) return "execution reverted";

        assembly {
            returnData := add(returnData, 0x04)
        }
        return abi.decode(returnData, (string));
    }
}
