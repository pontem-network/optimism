package database

import (
	"errors"
	"math/big"

	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/ethereum/go-ethereum/common"
)

/**
 * Types
 */

type TokenPair struct {
	L1TokenAddress common.Address `gorm:"serializer:json"`
	L2TokenAddress common.Address `gorm:"serializer:json"`
}

type BridgeDeposit struct {
	GUID                 uuid.UUID `gorm:"primaryKey"`
	InitiatedL1EventGUID uuid.UUID

	// Since we're only currently indexing a single StandardBridge,
	// the message nonce serves as a unique identifier for this
	// deposit. Once this generalizes to more than 1 deployed
	// bridge, we need to include the `CrossDomainMessenger` address
	// such that the (messenger_addr, nonce) is the unique identifier
	// for a bridge msg
	CrossDomainMessengerNonce U256
	DepositHash               common.Hash `gorm:"serializer:json"`

	FinalizedL2EventGUID *uuid.UUID

	Tx        Transaction `gorm:"embedded"`
	TokenPair TokenPair   `gorm:"embedded"`
}

type BridgeDepositWithTransactionHashes struct {
	BridgeDeposit              BridgeDeposit `gorm:"embedded"`
	L1TransactionHash          common.Hash   `gorm:"serializer:json"`
	FinalizedL2TransactionHash common.Hash   `gorm:"serializer:json"`
}

type BridgeWithdrawal struct {
	GUID                 uuid.UUID `gorm:"primaryKey"`
	InitiatedL2EventGUID uuid.UUID

	// Since we're only currently indexing a single StandardBridge,
	// the message nonce serves as a unique identifier for this
	// withdrawal. Once this generalizes to more than 1 deployed
	// bridge, we need to include the `CrossDomainMessenger` address
	// such that the (messenger_addr, nonce) is the unique identifier
	// for a bridge msg
	CrossDomainMessengerNonce U256
	WithdrawalHash            common.Hash `gorm:"serializer:json"`

	FinalizedL1EventGUID *uuid.UUID

	Tx        Transaction `gorm:"embedded"`
	TokenPair TokenPair   `gorm:"embedded"`
}

type BridgeWithdrawalWithTransactionHashes struct {
	BridgeWithdrawal  BridgeWithdrawal `gorm:"embedded"`
	L2TransactionHash common.Hash      `gorm:"serializer:json"`

	ProvenL1TransactionHash    common.Hash `gorm:"serializer:json"`
	FinalizedL1TransactionHash common.Hash `gorm:"serializer:json"`
}

type BridgeTransfersView interface {
	BridgeDepositsByAddress(address common.Address) ([]*BridgeDepositWithTransactionHashes, error)
	BridgeDepositByMessageNonce(*big.Int) (*BridgeDeposit, error)
	LatestBridgeDepositMessageNonce() (*big.Int, error)

	BridgeWithdrawalsByAddress(address common.Address) ([]*BridgeWithdrawalWithTransactionHashes, error)
	BridgeWithdrawalByMessageNonce(*big.Int) (*BridgeWithdrawal, error)
	LatestBridgeWithdrawalMessageNonce() (*big.Int, error)
}

type BridgeTransfersDB interface {
	BridgeTransfersView

	StoreBridgeDeposits([]*BridgeDeposit) error
	MarkFinalizedBridgeDepositEvent(uuid.UUID, uuid.UUID) error

	StoreBridgeWithdrawals([]*BridgeWithdrawal) error
	MarkFinalizedBridgeWithdrawalEvent(uuid.UUID, uuid.UUID) error
}

/**
 * Implementation
 */

type bridgeTransfersDB struct {
	gorm *gorm.DB
}

func newBridgeTransfersDB(db *gorm.DB) BridgeTransfersDB {
	return &bridgeTransfersDB{gorm: db}
}

// Deposits

func (db *bridgeTransfersDB) StoreBridgeDeposits(deposits []*BridgeDeposit) error {
	result := db.gorm.Create(&deposits)
	return result.Error
}

func (db *bridgeTransfersDB) BridgeDepositsByAddress(address common.Address) ([]*BridgeDepositWithTransactionHashes, error) {
	depositsQuery := db.gorm.Table("bridge_deposits").Select("bridge_deposits.*, l1_contract_events.transaction_hash AS l1_transaction_hash, l2_contract_events.transaction_hash AS finalized_l2_transaction_hash")

	initiatedJoinQuery := depositsQuery.Joins("LEFT JOIN l1_contract_events ON bridge_deposits.initiated_l1_event_guid = l1_contract_events.guid")
	finalizedJoinQuery := initiatedJoinQuery.Joins("LEFT JOIN l2_contract_events ON bridge_deposits.finalized_l2_event_guid = l2_contract_events.guid")

	// add in cursoring options
	filteredQuery := finalizedJoinQuery.Where(&Transaction{FromAddress: address}).Order("bridge_deposits.timestamp DESC").Limit(100)

	deposits := make([]*BridgeDepositWithTransactionHashes, 100)
	result := filteredQuery.Scan(&deposits)
	if result.Error != nil {
		if errors.Is(result.Error, gorm.ErrRecordNotFound) {
			return nil, nil
		}

		return nil, result.Error
	}

	return deposits, nil
}

func (db *bridgeTransfersDB) BridgeDepositByMessageNonce(nonce *big.Int) (*BridgeDeposit, error) {
	var deposit BridgeDeposit
	result := db.gorm.Where(&BridgeDeposit{CrossDomainMessengerNonce: U256{Int: nonce}}).Take(&deposit)
	if result.Error != nil {
		if errors.Is(result.Error, gorm.ErrRecordNotFound) {
			return nil, nil
		}

		return nil, result.Error
	}

	return &deposit, nil
}

func (db *bridgeTransfersDB) LatestBridgeDepositMessageNonce() (*big.Int, error) {
	var deposit BridgeDeposit
	result := db.gorm.Order("cross_domain_messager_nonce DESC").Take(&deposit)
	if result.Error != nil {
		if errors.Is(result.Error, gorm.ErrRecordNotFound) {
			return nil, nil
		}

		return nil, result.Error
	}

	return deposit.CrossDomainMessengerNonce.Int, nil
}

func (db *bridgeTransfersDB) MarkFinalizedBridgeDepositEvent(guid, finalizationEventGUID uuid.UUID) error {
	var deposit BridgeDeposit
	result := db.gorm.Where(&BridgeDeposit{GUID: guid}).Take(&deposit)
	if result.Error != nil {
		return result.Error
	}

	deposit.FinalizedL2EventGUID = &finalizationEventGUID
	result = db.gorm.Save(&deposit)
	return result.Error
}

// Withdrawals

func (db *bridgeTransfersDB) StoreBridgeWithdrawals(withdrawals []*BridgeWithdrawal) error {
	result := db.gorm.Create(&withdrawals)
	return result.Error
}

func (db *bridgeTransfersDB) MarkFinalizedBridgeWithdrawalEvent(guid, finalizedL1EventGuid uuid.UUID) error {
	var withdrawal BridgeWithdrawal
	result := db.gorm.Where(&BridgeWithdrawal{GUID: guid}).Take(&withdrawal)
	if result.Error != nil {
		return result.Error
	}

	withdrawal.FinalizedL1EventGUID = &finalizedL1EventGuid
	result = db.gorm.Save(&withdrawal)
	return result.Error
}

func (db *bridgeTransfersDB) BridgeWithdrawalsByAddress(address common.Address) ([]*BridgeWithdrawalWithTransactionHashes, error) {
	withdrawalsQuery := db.gorm.Table("bridge_withdrawals").Select("bridge_withdrawals.*, l2_contract_events.transaction_hash AS l2_transaction_hash, proven_l1_contract_events.transaction_hash AS proven_l1_transaction_hash, finalized_l1_contract_events.transaction_hash AS finalized_l1_transaction_hash")

	initiatedJoinQuery := withdrawalsQuery.Joins("LEFT JOIN l2_contract_events ON bridge_withdrawals.initiated_l2_event_guid = l2_contract_events.guid")
	finalizedJoinQuery := initiatedJoinQuery.Joins("LEFT JOIN l1_contract_events AS finalized_l1_contract_events ON bridge_withdrawals.finalized_l1_event_guid = finalized_l1_contract_events.guid")

	provenJoinQuery := initiatedJoinQuery.Joins("LEFT JOIN transaction_withdrawals ON bridge_withdrawals.withdrawal_hash = transaction_withdrawals.withdrawal_hash")
	provenJoinQuery = provenJoinQuery.Joins("LEFT JOIN l1_contract_events AS proven_l1_contract_events ON transaction_withdrawals.proven_l1_event_guid = proven_l1_contract_events.guid")

	// add in cursoring options
	filteredQuery := finalizedJoinQuery.Where(&Transaction{FromAddress: address}).Order("bridge_withdrawals.timestamp DESC").Limit(100)

	withdrawals := make([]*BridgeWithdrawalWithTransactionHashes, 100)
	result := filteredQuery.Scan(&withdrawals)
	if result.Error != nil {
		if errors.Is(result.Error, gorm.ErrRecordNotFound) {
			return nil, nil
		}

		return nil, result.Error
	}

	return withdrawals, nil
}

func (db *bridgeTransfersDB) BridgeWithdrawalByMessageNonce(nonce *big.Int) (*BridgeWithdrawal, error) {
	var withdrawal BridgeWithdrawal
	result := db.gorm.Where(&BridgeWithdrawal{CrossDomainMessengerNonce: U256{Int: nonce}}).Take(&withdrawal)
	if result.Error != nil {
		if errors.Is(result.Error, gorm.ErrRecordNotFound) {
			return nil, nil
		}

		return nil, result.Error
	}

	return &withdrawal, nil
}

func (db *bridgeTransfersDB) LatestBridgeWithdrawalMessageNonce() (*big.Int, error) {
	var withdrawal BridgeWithdrawal
	result := db.gorm.Order("sent_message_nonce DESC").Take(&withdrawal)
	if result.Error != nil {
		if errors.Is(result.Error, gorm.ErrRecordNotFound) {
			return nil, nil
		}

		return nil, result.Error
	}

	return withdrawal.CrossDomainMessengerNonce.Int, nil
}
