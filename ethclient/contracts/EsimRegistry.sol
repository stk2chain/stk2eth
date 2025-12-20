// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;



contract EsimRegistry {
    // --- Events ---

    /// @notice Emits when a profile is successfully registered to a wallet.
    event Register(bytes32 indexed profile, address indexed wallet, uint48 created);

    /// @notice Emits when a profile updates its wallet binding.
    event Update(bytes32 indexed profile, address indexed wallet, uint48 updated);

    /// @notice Emits when a profile is deregistered from a wallet.
    event Deregister(bytes32 indexed profile, address indexed wallet, uint48 removed);


    // --- State ---
    mapping(bytes32 => address) private profileToWallet;
    mapping(address => bytes32) private walletToProfile;

    bytes32 private immutable _CACHED_DOMAIN_SEPARATOR;
    uint256 private immutable _CACHED_CHAIN_ID;

    bytes32 private constant _HASHED_NAME = keccak256("EsimRegistry");
    bytes32 private constant _TYPE_HASH =
        keccak256("EIP712Domain(string name,uint256 chainId,address verifyingContract)");

    // ----------------------------------------------------------
    // Constructor
    // ----------------------------------------------------------
    constructor() {
        _CACHED_CHAIN_ID = block.chainid;
        _CACHED_DOMAIN_SEPARATOR = _buildDomainSeparator(_TYPE_HASH, _HASHED_NAME);
    }

    // --- Modifiers ---
    modifier onlySelf() {
        require(msg.sender == address(this), "Only self can call");
        _;
    }

    // ----------------------------------------------------------
    // Domain Separator
    // ----------------------------------------------------------

    /// @notice Returns the EIP-712 domain separator
    /// @dev Uses cached version if chainid unchanged, recomputes otherwise
    function DOMAIN_SEPARATOR() public view returns (bytes32) {
        return block.chainid == _CACHED_CHAIN_ID
            ? _CACHED_DOMAIN_SEPARATOR
            : _buildDomainSeparator(_TYPE_HASH, _HASHED_NAME);
    }

    /// @notice Internal builder for domain separator
    function _buildDomainSeparator(bytes32 typeHash, bytes32 nameHash) private view returns (bytes32) {
        return keccak256(abi.encode(typeHash, nameHash, block.chainid, address(this)));
    }

    /// @notice Creates an EIP-712 typed data hash using domain separator
    function _hashTypedData(bytes32 structHash) internal view returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR(), structHash));
    }






    // --- Internal Validation Modules ---
    function _checkProfileNotRegistered(bytes32 profile) internal view {
        require(profile != bytes32(0), "Invalid profile");
        require(profileToWallet[profile] == address(0), "Profile already registered");
    }

    function _checkWalletNotRegistered(address wallet) internal view {
        require(wallet != address(0), "Invalid wallet");
        require(walletToProfile[wallet] == bytes32(0), "Wallet already registered");
    }

    function _checkProfileExists(bytes32 profile) internal view {
        require(profileToWallet[profile] != address(0), "Profile not registered");
    }

    function _checkWalletExists(address wallet) internal view {
        require(walletToProfile[wallet] != bytes32(0), "Wallet not registered");
    }




    // --- Core Functions ---

    /// @notice Registers a new eSIM profile with a wallet address.
    /// @dev Callable only by this contract (via 7702 smart account execution).
    /// @param esimProfile The unique identifier of the eSIM profile (bytes32 hash).
    /// @param esimWallet The wallet address to bind to the profile.
    function registerProfile(bytes32 esimProfile, address esimWallet) external onlySelf {
        _checkProfileNotRegistered(esimProfile);
        _checkWalletNotRegistered(esimWallet);

        profileToWallet[esimProfile] = esimWallet;
        walletToProfile[esimWallet] = esimProfile;

        emit Register(esimProfile, esimWallet, uint48(block.timestamp));
    }

    /// @notice Updates an existing profile to bind to a new wallet.
    /// @dev Clears the old wallet mapping and assigns the new one.
    /// @param esimProfile The unique identifier of the eSIM profile (bytes32 hash).
    /// @param newWallet The new wallet address to bind to the profile.
    function updateProfile(bytes32 esimProfile, address newWallet) external onlySelf {
        _checkProfileExists(esimProfile);
        _checkWalletNotRegistered(newWallet);

        address oldWallet = profileToWallet[esimProfile];

        // Rebind mappings
        delete walletToProfile[oldWallet];
        profileToWallet[esimProfile] = newWallet;
        walletToProfile[newWallet] = esimProfile;

        emit Update(esimProfile, newWallet, uint48(block.timestamp));
    }

    /// @notice Deregisters a profile from its currently bound wallet.
    /// @dev Removes both sides of the mapping, effectively unbinding the profile.
    /// @param esimProfile The unique identifier of the eSIM profile (bytes32 hash).
    function deregisterProfile(bytes32 esimProfile) external onlySelf {
        _checkProfileExists(esimProfile);

        address wallet = profileToWallet[esimProfile];

        // Remove both sides of the mapping
        delete profileToWallet[esimProfile];
        delete walletToProfile[wallet];

        emit Deregister(esimProfile, wallet, uint48(block.timestamp));
    }




    // --- View Helpers ---

    /// @notice Returns the wallet bound to a given profile.
    /// @dev Reverts if the profile is not registered.
    /// @param esimProfile The unique identifier of the eSIM profile (bytes32 hash).
    /// @return The wallet address bound to the given profile.
    function getWallet(bytes32 esimProfile) external view returns (address) {
        _checkProfileExists(esimProfile);
        return profileToWallet[esimProfile];
    }

    /// @notice Returns the profile bound to a given wallet.
    /// @dev Reverts if the wallet is not registered.
    /// @param esimWallet The wallet address to query.
    /// @return The eSIM profile bound to the given wallet.
    function getProfile(address esimWallet) external view returns (bytes32) {
        _checkWalletExists(esimWallet);
        return walletToProfile[esimWallet];
    }
}
