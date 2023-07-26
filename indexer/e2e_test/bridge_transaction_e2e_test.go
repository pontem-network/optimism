package e2e_tests

import (
	"context"
	"math/big"
	"testing"
	"time"

	"github.com/ethereum-optimism/optimism/op-bindings/bindings"
	"github.com/ethereum-optimism/optimism/op-bindings/predeploys"
	op_e2e "github.com/ethereum-optimism/optimism/op-e2e"
	"github.com/ethereum-optimism/optimism/op-node/rollup/derive"
	"github.com/ethereum-optimism/optimism/op-node/withdrawals"
	"github.com/ethereum-optimism/optimism/op-service/client/utils"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/params"

	"github.com/stretchr/testify/require"
)

func TestE2EBridgeTransactions(t *testing.T) {
	testSuite := createE2ETestSuite(t)

	l1Client := testSuite.OpSys.Clients["l1"]
	l2Client := testSuite.OpSys.Clients["sequencer"]

	optimismPortal, err := bindings.NewOptimismPortal(predeploys.DevOptimismPortalAddr, l1Client)
	require.NoError(t, err)

	l2ToL1MessagePasser, err := bindings.NewL2ToL1MessagePasser(predeploys.L2ToL1MessagePasserAddr, l2Client)
	require.NoError(t, err)

	aliceAddr := testSuite.OpCfg.Secrets.Addresses().Alice

	l1Opts, err := bind.NewKeyedTransactorWithChainID(testSuite.OpCfg.Secrets.Alice, testSuite.OpCfg.L1ChainIDBig())
	require.NoError(t, err)
	l2Opts, err := bind.NewKeyedTransactorWithChainID(testSuite.OpCfg.Secrets.Alice, testSuite.OpCfg.L2ChainIDBig())
	require.NoError(t, err)

	// Simply transfer 1ETH using the low level contracts
	l1Opts.Value = big.NewInt(params.Ether)
	l2Opts.Value = big.NewInt(params.Ether)

	// pre-emptively conduct a deposit & withdrawal to speed up the test
	depositTx, err := optimismPortal.DepositTransaction(l1Opts, aliceAddr, l1Opts.Value, 100_000, false, []byte{byte(1)})
	require.NoError(t, err)

	withdrawTx, err := l2ToL1MessagePasser.InitiateWithdrawal(l2Opts, aliceAddr, big.NewInt(100_000), []byte{byte(1)})
	require.NoError(t, err)

	t.Run("indexes OptimismPortal transaction deposits", func(t *testing.T) {
		testCtx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
		defer cancel()

		depositReceipt, err := utils.WaitReceiptOK(testCtx, l1Client, depositTx.Hash())
		require.NoError(t, err)

		var depositHash common.Hash
		var transactionDeposited *bindings.OptimismPortalTransactionDeposited
		for _, log := range depositReceipt.Logs {
			if len(log.Topics) > 0 && log.Topics[0] == derive.DepositEventABIHash {
				transactionDeposited, err = optimismPortal.ParseTransactionDeposited(*log)
				require.NoError(t, err)
				depositTx, err := derive.UnmarshalDepositLogEvent(log)
				require.NoError(t, err)
				depositHash = depositTx.SourceHash
				break
			}
		}

		// wait for processor catchup
		require.NoError(t, utils.WaitFor(testCtx, 500*time.Millisecond, func() (bool, error) {
			l1Header := testSuite.Indexer.L1Processor.LatestProcessedHeader()
			return l1Header != nil && l1Header.Number.Uint64() >= depositReceipt.BlockNumber.Uint64(), nil
		}))

		deposit, err := testSuite.DB.BridgeTransactions.TransactionDepositByHash(depositHash)
		require.NoError(t, err)
		require.Equal(t, big.NewInt(100_000), deposit.GasLimit.Int)
		require.Equal(t, big.NewInt(params.Ether), deposit.Tx.Amount.Int)
		require.Equal(t, aliceAddr, deposit.Tx.FromAddress)
		require.Equal(t, aliceAddr, deposit.Tx.ToAddress)
		require.Equal(t, byte(1), deposit.Tx.Data[0])

		require.Equal(t, transactionDeposited.Version.Uint64(), deposit.Version.Int.Uint64())
		require.ElementsMatch(t, transactionDeposited.OpaqueData, deposit.OpaqueData)

		event, err := testSuite.DB.ContractEvents.L1ContractEvent(deposit.InitiatedL1EventGUID)
		require.NoError(t, err)
		require.Equal(t, event.TransactionHash, depositTx.Hash())

		// TODO L2 inclusion
	})

	t.Run("indexes L2ToL1MessagePasser transaction withdrawals", func(t *testing.T) {
		testCtx, cancel := context.WithTimeout(context.Background(), time.Minute)
		defer cancel()

		withdrawReceipt, err := utils.WaitReceiptOK(testCtx, l2Client, withdrawTx.Hash())
		require.NoError(t, err)

		// wait for processor catchup
		require.NoError(t, utils.WaitFor(testCtx, 500*time.Millisecond, func() (bool, error) {
			l2Header := testSuite.Indexer.L2Processor.LatestProcessedHeader()
			return l2Header != nil && l2Header.Number.Uint64() >= withdrawReceipt.BlockNumber.Uint64(), nil
		}))

		msgPassed, err := withdrawals.ParseMessagePassed(withdrawReceipt)
		require.NoError(t, err)
		withdrawalHash, err := withdrawals.WithdrawalHash(msgPassed)
		require.NoError(t, err)

		withdraw, err := testSuite.DB.BridgeTransactions.TransactionWithdrawalByHash(withdrawalHash)
		require.NoError(t, err)
		require.Equal(t, msgPassed.Nonce.Uint64(), withdraw.Nonce.Int.Uint64())
		require.Equal(t, big.NewInt(100_000), withdraw.GasLimit.Int)
		require.Equal(t, big.NewInt(params.Ether), withdraw.Tx.Amount.Int)
		require.Equal(t, aliceAddr, withdraw.Tx.FromAddress)
		require.Equal(t, aliceAddr, withdraw.Tx.ToAddress)
		require.Equal(t, byte(1), withdraw.Tx.Data[0])

		require.Nil(t, withdraw.ProvenL1EventGUID)
		require.Nil(t, withdraw.FinalizedL1EventGUID)

		event, err := testSuite.DB.ContractEvents.L2ContractEvent(withdraw.InitiatedL2EventGUID)
		require.NoError(t, err)
		require.Equal(t, event.TransactionHash, withdrawTx.Hash())

		// Test Withdrawal Proven
		withdrawParams, proveReceipt := op_e2e.ProveWithdrawal(t, *testSuite.OpCfg, l1Client, testSuite.OpSys.Nodes["sequencer"], testSuite.OpCfg.Secrets.Alice, withdrawReceipt)
		require.NoError(t, utils.WaitFor(testCtx, 500*time.Millisecond, func() (bool, error) {
			l1Header := testSuite.Indexer.L1Processor.LatestProcessedHeader()
			return l1Header != nil && l1Header.Number.Uint64() >= proveReceipt.BlockNumber.Uint64(), nil
		}))

		withdraw, err = testSuite.DB.BridgeTransactions.TransactionWithdrawalByHash(withdrawalHash)
		require.NoError(t, err)
		require.NotNil(t, withdraw.ProvenL1EventGUID)

		proveEvent, err := testSuite.DB.ContractEvents.L1ContractEvent(*withdraw.ProvenL1EventGUID)
		require.NoError(t, err)
		require.Equal(t, proveEvent.TransactionHash, proveReceipt.TxHash)

		require.Nil(t, withdraw.FinalizedL1EventGUID)

		// Test Withdrawal Finalized
		finalizeReceipt := op_e2e.FinalizeWithdrawal(t, *testSuite.OpCfg, l1Client, testSuite.OpCfg.Secrets.Alice, proveReceipt, withdrawParams)
		require.NoError(t, utils.WaitFor(testCtx, 500*time.Millisecond, func() (bool, error) {
			l1Header := testSuite.Indexer.L1Processor.LatestProcessedHeader()
			return l1Header != nil && l1Header.Number.Uint64() >= finalizeReceipt.BlockNumber.Uint64(), nil
		}))

		withdraw, err = testSuite.DB.BridgeTransactions.TransactionWithdrawalByHash(withdrawalHash)
		require.NoError(t, err)
		require.NotNil(t, withdraw.FinalizedL1EventGUID)

		finalizedEvent, err := testSuite.DB.ContractEvents.L1ContractEvent(*withdraw.FinalizedL1EventGUID)
		require.NoError(t, err)
		require.Equal(t, finalizedEvent.TransactionHash, finalizeReceipt.TxHash)
	})
}
