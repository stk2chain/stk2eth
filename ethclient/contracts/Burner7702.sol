// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.8.32;

contract Burner7702 {
    address public immutable OPERATOR;
    
    constructor(address operator_) {
        require(operator_ != address(0), "Zero address operator");
        OPERATOR = operator_;
    }

    receive() external payable {}

    modifier onlyOperator() {
        require(msg.sender == OPERATOR, "Only operator can call");
        _;
    }

    /**
     * @notice Executes a single call
     * @param target The target address to call
     * @param value The amount of ETH to send
     * @param data The data to send
     * @dev Callable only by the operator
     */
    function execute(
        address target,
        uint256 value,
        bytes calldata data
    ) external onlyOperator {
        (bool ok, bytes memory ret) = target.call{value: value}(data);
        if (!ok) assembly {
            revert(add(ret, 32), mload(ret))
        }
    }

    /**
     * @notice Executes a batch of calls
     * @param targets The target addresses to call
     * @param values The amounts of ETH to send
     * @param data The data to send
     * @dev Callable only by the operator
     */
    function executeBatch(
        address[] calldata targets,
        uint256[] calldata values,
        bytes[] calldata data
    ) external onlyOperator {
        uint256 len = targets.length;
        for (uint256 i; i < len; ++i) {
            (bool ok, bytes memory ret) =
                targets[i].call{value: values[i]}(data[i]);
            if (!ok) assembly {
                revert(add(ret, 32), mload(ret))
            }
        }
    }
}
