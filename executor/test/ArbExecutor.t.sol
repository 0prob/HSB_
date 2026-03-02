// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../src/ArbExecutor.sol";

contract MockDEX {
    uint256 public amount;

    constructor(uint256 _amount) {
        amount = _amount;
    }

    function swap() external payable returns (uint256) {
        payable(msg.sender).transfer(amount);
        return amount;
    }
}

contract ArbExecutorTest is Test {
    ArbExecutor exec;
    MockDEX dex1;
    MockDEX dex2;

    function setUp() public {
        exec = new ArbExecutor();
        dex1 = new MockDEX(1 ether);
        dex2 = new MockDEX(2 ether);

        vm.deal(address(exec), 1 ether);
    }

    function testMultiHop() public {
        address[] memory targets = new address[](2);
        bytes[] memory data = new bytes[](2);

        targets[0] = address(dex1);
        targets[1] = address(dex2);

        data[0] = abi.encodeWithSignature("swap()");
        data[1] = abi.encodeWithSignature("swap()");

        uint256 profit = exec.execute(targets, data, 0.5 ether);
        assertGt(profit, 0);
    }

    function testSlippageReverts() public {
        address[] memory targets = new address[](1);
        bytes[] memory data = new bytes[](1);

        targets[0] = address(dex1);
        data[0] = abi.encodeWithSignature("swap()");

        vm.expectRevert();
        exec.execute(targets, data, 5 ether);
    }
}
