
CREATE DOMAIN UINT256 AS NUMERIC NOT NULL
    CHECK (VALUE >= 0 AND VALUE < 2^256 and SCALE(VALUE) = 0);

/**
 * BLOCK DATA
 */

CREATE TABLE IF NOT EXISTS l1_block_headers (
	hash        VARCHAR NOT NULL PRIMARY KEY,
	parent_hash VARCHAR NOT NULL,
	number      UINT256,
	timestamp   INTEGER NOT NULL CHECK (timestamp > 0)
);

CREATE TABLE IF NOT EXISTS l2_block_headers (
    -- Block header
	hash                     VARCHAR NOT NULL PRIMARY KEY,
	parent_hash              VARCHAR NOT NULL,
	number                   UINT256,
	timestamp                INTEGER NOT NULL CHECK (timestamp > 0)
);

/** 
 * EVENT DATA
 */

CREATE TABLE IF NOT EXISTS l1_contract_events (
    guid             VARCHAR NOT NULL PRIMARY KEY,
	block_hash       VARCHAR NOT NULL REFERENCES l1_block_headers(hash),
    transaction_hash VARCHAR NOT NULL,
    event_signature  VARCHAR NOT NULL,
    log_index        INTEGER NOT NULL,
    timestamp        INTEGER NOT NULL CHECK (timestamp > 0)
);

CREATE TABLE IF NOT EXISTS l2_contract_events (
    guid             VARCHAR NOT NULL PRIMARY KEY,
	block_hash       VARCHAR NOT NULL REFERENCES l2_block_headers(hash),
    transaction_hash VARCHAR NOT NULL,
    event_signature  VARCHAR NOT NULL,
    log_index        INTEGER NOT NULL,
    timestamp        INTEGER NOT NULL CHECK (timestamp > 0)
);

-- Tables that index finalization markers for L2 blocks.

CREATE TABLE IF NOT EXISTS legacy_state_batches (
	index         INTEGER NOT NULL PRIMARY KEY,
	root          VARCHAR NOT NULL,
	size          INTEGER NOT NULL,
	prev_total    INTEGER NOT NULL,

    l1_contract_event_guid VARCHAR REFERENCES l1_contract_events(guid)
);

CREATE TABLE IF NOT EXISTS output_proposals (
    output_root     VARCHAR NOT NULL PRIMARY KEY,

    l2_output_index UINT256,
    l2_block_number UINT256,

    l1_contract_event_guid VARCHAR REFERENCES l1_contract_events(guid)
);

/**
 * BRIDGING DATA
 */

-- OptimismPortal/L2ToL1MessagePasser
CREATE TABLE IF NOT EXISTS transaction_deposits (
    deposit_hash VARCHAR NOT NULL PRIMARY KEY,

    initiated_l1_event_guid VARCHAR NOT NULL REFERENCES l1_contract_events(guid),
    --inclusion_l2_block_hash VARCHAR REFERENCES l2_block_headers(hash),

    -- OptimismPortal specific
    version     UINT256,
    opaque_data VARCHAR NOT NULL,

    -- transaction data
    from_address VARCHAR NOT NULL,
    to_address   VARCHAR NOT NULL,
    amount       UINT256,
    gas_limit    UINT256,
    data         VARCHAR NOT NULL,
    timestamp    INTEGER NOT NULL CHECK (timestamp > 0)
);
CREATE TABLE IF NOT EXISTS transaction_withdrawals (
    withdrawal_hash VARCHAR NOT NULL PRIMARY KEY,

    initiated_l2_event_guid VARCHAR NOT NULL REFERENCES l2_contract_events(guid),

    -- Multistep (bedrock) process of a withdrawal
    proven_l1_event_guid    VARCHAR REFERENCES l1_contract_events(guid),
    finalized_l1_event_guid VARCHAR REFERENCES l1_contract_events(guid),

    -- L2ToL1MessagePasser specific
    nonce UINT256 UNIQUE,

    -- transaction data
    from_address VARCHAR NOT NULL,
    to_address   VARCHAR NOT NULL,
    amount       UINT256,
    gas_limit    UINT256,
    data         VARCHAR NOT NULL,
    timestamp    INTEGER NOT NULL CHECK (timestamp > 0)
);

/*
-- CrossDomainMessenger
CREATE TABLE IF NOT EXISTS l1_sent_messages(
    nonce                    UINT256 NOT NULL PRIMARY KEY,
    event_guid               VARCHAR NOT NULL REFERENCES l1_contract_events(guid),
    transaction_deposit_hash VARCHAR NOT NULL REFERENCES transaction_deposits(deposit_hash),

    -- sent message
    target_address VARCHAR NOT NULL,
    sender_address VARCHAR NOT NULL,
    value          UINT256,
    message        VARCHAR NOT NULL,
    gasLimit       UINT256 NOT NULL,
    timestamp      INTEGER NOT NULL CHECK (timestamp > 0)
);
CREATE TABLE IF NOT EXISTS l2_sent_messages(
    nonce                       UINT256 NOT NULL PRIMARY KEY,
    event_guid                  VARCHAR NOT NULL REFERENCES l2_contract_events(guid),
    transaction_withdrawal_hash VARCHAR NOT NULL transaction_withdrawals(withdrawal_hash),

    -- sent message
    target_address VARCHAR NOT NULL,
    sender_address VARCHAR NOT NULL,
    message        VARCHAR NOT NULL,
    value          UINT256,
    gasLimit       UINT256,
    timestamp      INTEGER NOT NULL CHECK (timestamp > 0)
);
*/

-- StandardBridge
CREATE TABLE IF NOT EXISTS bridge_deposits (
	guid VARCHAR PRIMARY KEY NOT NULL,

    initiated_l1_event_guid VARCHAR NOT NULL REFERENCES l1_contract_events(guid),
    finalized_l2_event_guid VARCHAR REFERENCES l2_contract_events(guid),

    deposit_hash                 VARCHAR UNIQUE NOT NULL REFERENCES transaction_deposits(deposit_hash),
    cross_domain_messenger_nonce UINT256 UNIQUE,

    -- Deposit information
	from_address     VARCHAR NOT NULL,
	to_address       VARCHAR NOT NULL,
	l1_token_address VARCHAR NOT NULL,
	l2_token_address VARCHAR NOT NULL,
	amount           UINT256,
	data             VARCHAR NOT NULL,
    timestamp        INTEGER NOT NULL CHECK (timestamp > 0)
);
CREATE TABLE IF NOT EXISTS bridge_withdrawals (
	guid                VARCHAR PRIMARY KEY NOT NULL,

    initiated_l2_event_guid VARCHAR NOT NULL REFERENCES l2_contract_events(guid),
    finalized_l1_event_guid VARCHAR REFERENCES l1_contract_events(guid),

    withdrawal_hash              VARCHAR UNIQUE NOT NULL REFERENCES transaction_withdrawals(withdrawal_hash),
    cross_domain_messenger_nonce UINT256 UNIQUE,

    -- Withdrawal information
	from_address     VARCHAR NOT NULL,
	to_address       VARCHAR NOT NULL,
	l1_token_address VARCHAR NOT NULL,
	l2_token_address VARCHAR NOT NULL,
	amount           UINT256,
	data             VARCHAR NOT NULL,
    timestamp        INTEGER NOT NULL CHECK (timestamp > 0)
);
