// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import {Test} from "forge-std/Test.sol";
import {Action, ActionItem, ActionQueue, ActionQueueLib} from "../src/ActionQueue.sol";

contract ActionQueueTest is Test {
    using ActionQueueLib for ActionQueue;

    ActionQueue queue;

    function testPushPopFifo() public {
        queue.push(ActionItem({action: Action.Deposit, sender: address(1), receiver: address(10), amount: 100}));
        queue.push(ActionItem({action: Action.Withdraw, sender: address(2), receiver: address(20), amount: 200}));
        queue.push(ActionItem({action: Action.Transfer, sender: address(3), receiver: address(30), amount: 300}));

        ActionItem memory a = queue.pop();
        assertEq(uint8(a.action), uint8(Action.Deposit));
        assertEq(a.sender, address(1));
        assertEq(a.receiver, address(10));
        assertEq(a.amount, 100);

        ActionItem memory b = queue.pop();
        assertEq(uint8(b.action), uint8(Action.Withdraw));
        assertEq(b.sender, address(2));
        assertEq(b.receiver, address(20));
        assertEq(b.amount, 200);

        ActionItem memory c = queue.pop();
        assertEq(uint8(c.action), uint8(Action.Transfer));
        assertEq(c.sender, address(3));
        assertEq(c.receiver, address(30));
        assertEq(c.amount, 300);
    }

    function testPopEmptyReturnsNotPresent() public {
        ActionItem memory item = queue.pop();
        assertEq(uint8(item.action), uint8(Action.NotPresent));
        assertEq(item.sender, address(0));
        assertEq(item.receiver, address(0));
        assertEq(item.amount, 0);
    }

    function testPeekDoesNotRemove() public {
        queue.push(ActionItem({action: Action.Deposit, sender: address(1), receiver: address(2), amount: 3}));

        ActionItem memory p = queue.peek();
        assertEq(uint8(p.action), uint8(Action.Deposit));
        assertEq(queue.len(), 1); // still there

        ActionItem memory popped = queue.pop();
        assertEq(uint8(popped.action), uint8(Action.Deposit));
        assertEq(queue.len(), 0);
    }

    function testPeekEmptyReturnsNotPresent() public view {
        ActionItem memory item = queue.peek();
        assertEq(uint8(item.action), uint8(Action.NotPresent));
    }

    function testIsEmptyAndLen() public {
        assertTrue(queue.isEmpty());
        assertEq(queue.len(), 0);

        queue.push(ActionItem({action: Action.Deposit, sender: address(0), receiver: address(0), amount: 1}));
        assertFalse(queue.isEmpty());
        assertEq(queue.len(), 1);

        queue.push(ActionItem({action: Action.Withdraw, sender: address(0), receiver: address(0), amount: 2}));
        assertEq(queue.len(), 2);

        queue.pop();
        assertEq(queue.len(), 1);
        assertFalse(queue.isEmpty());

        queue.pop();
        assertEq(queue.len(), 0);
        assertTrue(queue.isEmpty());
    }

    function testContains() public {
        // nothing pushed yet — key 0 not present
        assertFalse(queue.contains(0));

        queue.push(ActionItem({action: Action.Deposit, sender: address(0), receiver: address(0), amount: 1})); // key 0
        queue.push(ActionItem({action: Action.Withdraw, sender: address(0), receiver: address(0), amount: 2})); // key 1

        assertTrue(queue.contains(0));
        assertTrue(queue.contains(1));
        assertFalse(queue.contains(2)); // not yet pushed

        queue.pop(); // removes key 0
        assertFalse(queue.contains(0)); // popped — no longer present
        assertTrue(queue.contains(1)); // still in queue
    }

    function testPushNotPresentReverts() public {
        vm.expectRevert(abi.encodeWithSelector(ActionQueueLib.CannotPushNotPresent.selector));
        queue.push(ActionItem({action: Action.NotPresent, sender: address(0), receiver: address(0), amount: 0}));
    }

    function testLenAfterInterleavedOps() public {
        for (uint160 i = 1; i <= 4; i++) {
            queue.push(ActionItem({action: Action.Deposit, sender: address(i), receiver: address(i), amount: i}));
        }
        assertEq(queue.len(), 4);

        queue.pop();
        queue.pop();
        assertEq(queue.len(), 2);

        queue.push(ActionItem({action: Action.Transfer, sender: address(5), receiver: address(5), amount: 5}));
        assertEq(queue.len(), 3);

        while (!queue.isEmpty()) {
            queue.pop();
        }
        assertEq(queue.len(), 0);
        assertTrue(queue.isEmpty());
    }
}
