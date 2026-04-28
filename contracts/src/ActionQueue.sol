// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @notice The type of action stored in the queue.
/// @dev `NotPresent` is the zero-value sentinel returned for empty/popped slots. It must never be pushed.
enum Action {
    NotPresent, // Zero-value sentinel — returned when querying a missing or popped slot
    Deposit,
    Withdraw,
    Transfer,
    Dummy // A no-op action used to initialize the queue with a non-NotPresent value at index 0
}

/// @notice A single queued action.
struct ActionItem {
    Action action;
    address sender; // Commitment, or actual sender ID for confidential actions
    address receiver; // Commitment, or actual receiver ID for confidential actions
    uint256 amount; // Commitment, or actual token amount
}

/// @notice A FIFO queue of `ActionItem`s backed by a dense mapping.
/// @dev Keys are monotonically increasing. `lowestKey` is the front; `nextKey` is one past the back.
struct ActionQueue {
    mapping(uint256 => ActionItem) data;
    uint256 nextKey; // Next key to write at (== total ever pushed)
    uint256 lowestKey; // Front of the queue (== total ever popped)
}

/// @notice Library for operating on `ActionQueue`.
library ActionQueueLib {
    error CannotPushNotPresent();

    /// @notice Push an item onto the back of the queue.
    /// @dev Reverts if `value.action == Action.NotPresent` to protect the sentinel invariant.
    /// @param value The action item to enqueue.
    function push(ActionQueue storage self, ActionItem memory value) internal returns (uint256) {
        if (value.action == Action.NotPresent) revert CannotPushNotPresent();
        self.data[self.nextKey] = value;
        uint256 insertedKey = self.nextKey;
        unchecked {
            self.nextKey++;
        }
        return insertedKey;
    }

    /// @notice Returns true if the queue has no items.
    function isEmpty(ActionQueue storage self) internal view returns (bool) {
        return self.nextKey == self.lowestKey;
    }

    /// @notice Returns the number of items currently in the queue.
    function len(ActionQueue storage self) internal view returns (uint256) {
        return self.nextKey - self.lowestKey;
    }

    /// @notice Remove and return the front item.
    /// @dev Returns a sentinel `ActionItem` with `action == Action.NotPresent` if the queue is empty.
    /// @return value The dequeued item, or a zeroed sentinel if empty.
    function pop(ActionQueue storage self) internal returns (ActionItem memory value) {
        if (self.nextKey == self.lowestKey) {
            return ActionItem({action: Action.NotPresent, sender: address(0), receiver: address(0), amount: 0});
        }
        value = self.data[self.lowestKey];
        delete self.data[self.lowestKey];
        unchecked {
            self.lowestKey++;
        }
    }

    /// @notice Return the item at index key without removing it.
    /// @dev Returns a sentinel `ActionItem` with `action == Action.NotPresent` if key is not present.
    /// @return The specified item, or a zeroed sentinel if empty.
    function get(ActionQueue storage self, uint256 key) public view returns (ActionItem memory) {
        if (key < self.lowestKey || key >= self.nextKey) {
            return ActionItem({action: Action.NotPresent, sender: address(0), receiver: address(0), amount: 0});
        }
        return self.data[key];
    }

    /// @notice Return the front item without removing it.
    /// @dev Returns a sentinel `ActionItem` with `action == Action.NotPresent` if the queue is empty.
    /// @return The front item, or a zeroed sentinel if empty.
    function peek(ActionQueue storage self) internal view returns (ActionItem memory) {
        if (self.nextKey == self.lowestKey) {
            return ActionItem({action: Action.NotPresent, sender: address(0), receiver: address(0), amount: 0});
        }
        return self.data[self.lowestKey];
    }

    function peekIndex(ActionQueue storage self) internal view returns (uint256) {
        return self.lowestKey;
    }

    /// @notice Returns true if `key` is currently in the queue (pushed but not yet popped).
    /// @param key The mapping key to check.
    function contains(ActionQueue storage self, uint256 key) internal view returns (bool) {
        return key >= self.lowestKey && key < self.nextKey;
    }
}

///////////////////////////////////////////////////////////////////////////////
