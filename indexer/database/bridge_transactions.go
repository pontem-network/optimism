package database

import (
	"errors"
	"fmt"

	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/common/hexutil"
)

/**
 * Types
 */

type Transaction struct {
	FromAddress common.Address `gorm:"serializer:json"`
	ToAddress   common.Address `gorm:"serializer:json"`
	Amount      U256
	Data        hexutil.Bytes `gorm:"serializer:json"`
	Timestamp   uint64
}

type TransactionDeposit struct {
	DepositHash common.Hash `gorm:"serializer:json;primaryKey"`

	InitiatedL1EventGUID uuid.UUID
	//InclusionL2BlockHash *common.Hash `gorm:"serializer:json"`

	Version    U256
	OpaqueData hexutil.Bytes `gorm:"serializer:json"`

	Tx       Transaction `gorm:"embedded"`
	GasLimit U256
}

type TransactionWithdrawal struct {
	WithdrawalHash common.Hash `gorm:"serializer:json;primaryKey"`

	InitiatedL2EventGUID uuid.UUID

	ProvenL1EventGUID    *uuid.UUID
	FinalizedL1EventGUID *uuid.UUID

	Nonce U256

	Tx       Transaction `gorm:"embedded"`
	GasLimit U256
}

type BridgeTransactionsView interface {
	TransactionWithdrawalByHash(common.Hash) (*TransactionWithdrawal, error)
}

type BridgeTransactionsDB interface {
	BridgeTransactionsView

	StoreTransactionDeposits([]*TransactionDeposit) error
	//MarkTransactionDepositInclusion(common.Hash, common.Hash) error

	StoreTransactionWithdrawals([]*TransactionWithdrawal) error
	MarkTransactionWithdrawalProvenEvent(common.Hash, uuid.UUID) error
	MarkTransactionWithdrawalFinalizedEvent(common.Hash, uuid.UUID) error
}

/**
 * Implementation
 */

type bridgeTransactionsDB struct {
	gorm *gorm.DB
}

func newBridgeTransactionsDB(db *gorm.DB) BridgeTransactionsDB {
	return &bridgeTransactionsDB{gorm: db}
}

func (db *bridgeTransactionsDB) StoreTransactionDeposits(deposits []*TransactionDeposit) error {
	result := db.gorm.Create(&deposits)
	return result.Error
}

/*
func (db *bridgeTransactionsDB) MarkTransactionDepositInclusion(depositHash common.Hash, l2BlockHash common.Hash) error {
	var deposit TransactionDeposit
	result := db.gorm.Where(&TransactionDeposit{DepositHash: depositHash}).Take(&deposit)
	if result.Error != nil {
		return result.Error
	}

	deposit.InclusionL2BlockHash = &l2BlockHash
	result = db.gorm.Save(&deposit)
	return result.Error
}
*/

func (db *bridgeTransactionsDB) StoreTransactionWithdrawals(withdrawals []*TransactionWithdrawal) error {
	result := db.gorm.Create(&withdrawals)
	return result.Error
}

func (db *bridgeTransactionsDB) TransactionWithdrawalByHash(withdrawalHash common.Hash) (*TransactionWithdrawal, error) {
	var withdrawal TransactionWithdrawal
	result := db.gorm.Where(&BridgeWithdrawal{WithdrawalHash: withdrawalHash}).Take(&withdrawal)
	if result.Error != nil {
		if errors.Is(result.Error, gorm.ErrRecordNotFound) {
			return nil, nil
		}

		return nil, result.Error
	}

	return &withdrawal, nil
}

func (db *bridgeTransactionsDB) MarkTransactionWithdrawalProvenEvent(withdrawalHash common.Hash, provenL1EventGuid uuid.UUID) error {
	var withdrawal TransactionWithdrawal
	result := db.gorm.Where(&BridgeWithdrawal{WithdrawalHash: withdrawalHash}).Take(&withdrawal)
	if result.Error != nil {
		return result.Error
	}

	withdrawal.ProvenL1EventGUID = &provenL1EventGuid
	result = db.gorm.Save(&withdrawal)
	return result.Error
}

func (db *bridgeTransactionsDB) MarkTransactionWithdrawalFinalizedEvent(withdrawalHash common.Hash, finalizedL1EventGuid uuid.UUID) error {
	var withdrawal TransactionWithdrawal
	result := db.gorm.Where(&BridgeWithdrawal{WithdrawalHash: withdrawalHash}).Take(&withdrawal)
	if result.Error != nil {
		return result.Error
	}

	if withdrawal.ProvenL1EventGUID == nil {
		return fmt.Errorf("cannot mark unproven withdrawal %s as finalized", withdrawal.WithdrawalHash)
	}

	withdrawal.FinalizedL1EventGUID = &finalizedL1EventGuid
	result = db.gorm.Save(&withdrawal)
	return result.Error
}
