package e2e_tests

import (
	"context"
	"math/big"
	"testing"
	"time"

	"github.com/ethereum-optimism/optimism/indexer/processor"
	"github.com/ethereum-optimism/optimism/op-bindings/bindings"
	"github.com/ethereum-optimism/optimism/op-bindings/predeploys"
	op_e2e "github.com/ethereum-optimism/optimism/op-e2e"
	"github.com/ethereum-optimism/optimism/op-node/rollup/derive"
	"github.com/ethereum-optimism/optimism/op-node/withdrawals"
	"github.com/ethereum-optimism/optimism/op-service/client/utils"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/params"

	"github.com/stretchr/testify/require"
)

func TestE2EBridgeTransfers(t *testing.T) {
	testSuite := createE2ETestSuite(t)

	l1Client := testSuite.OpSys.Clients["l1"]
	l2Client := testSuite.OpSys.Clients["sequencer"]

	l1StandardBridge, err := bindings.NewL1StandardBridge(predeploys.DevL1StandardBridgeAddr, l1Client)
	require.NoError(t, err)

	l2StandardBridge, err := bindings.NewL2StandardBridge(predeploys.L2StandardBridgeAddr, l2Client)
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
	depositTx, err := l1StandardBridge.DepositETH(l1Opts, 200_000, []byte{byte(1)})
	require.NoError(t, err)

	withdrawTx, err := l2StandardBridge.Withdraw(l2Opts, predeploys.LegacyERC20ETHAddr, l2Opts.Value, 200_000, []byte{byte(1)})
	require.NoError(t, err)

	t.Run("indexes ETH deposits", func(t *testing.T) {
		testCtx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
		defer cancel()

		// Pause the L2Processor so that we can test for finalization separately. A pause is
		// required since deposit inclusion is apart of the L2 block derivation process
		testSuite.Indexer.L2Processor.PauseForTest()

		// (1) Test Deposit Initiation
		depositReceipt, err := utils.WaitReceiptOK(testCtx, l1Client, depositTx.Hash())
		require.NoError(t, err)

		var depositHash common.Hash
		for _, log := range depositReceipt.Logs {
			if len(log.Topics) > 0 && log.Topics[0] == derive.DepositEventABIHash {
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

		aliceDeposits, err := testSuite.DB.BridgeTransfers.BridgeDepositsByAddress(aliceAddr)
		require.NoError(t, err)
		require.Len(t, aliceDeposits, 1)
		require.Equal(t, depositTx.Hash(), aliceDeposits[0].L1TransactionHash)
		require.Empty(t, aliceDeposits[0].FinalizedL2TransactionHash)

		deposit := aliceDeposits[0].BridgeDeposit
		require.Equal(t, depositHash, deposit.DepositHash)
		require.Equal(t, processor.EthAddress, deposit.TokenPair.L1TokenAddress)
		require.Equal(t, processor.EthAddress, deposit.TokenPair.L2TokenAddress)
		require.Equal(t, big.NewInt(params.Ether), deposit.Tx.Amount.Int)
		require.Equal(t, aliceAddr, deposit.Tx.FromAddress)
		require.Equal(t, aliceAddr, deposit.Tx.ToAddress)
		require.Equal(t, byte(1), deposit.Tx.Data[0])
		require.Nil(t, deposit.FinalizedL2EventGUID)

		// (2) Test Deposit Finalization
		testSuite.Indexer.L2Processor.ResumeForTest()

		// finalization hash can be deterministically derived from TransactionDeposited log
		var depositTxHash common.Hash
		for _, log := range depositReceipt.Logs {
			if log.Topics[0] == derive.DepositEventABIHash {
				deposit, err := derive.UnmarshalDepositLogEvent(log)
				require.NoError(t, err)
				depositTxHash = types.NewTx(deposit).Hash()
				break
			}
		}

		// wait for the l2 processor to catch this deposit in the derivation process
		_, err = utils.WaitReceiptOK(testCtx, l2Client, depositTxHash)
		require.NoError(t, err)
		l2Height, err := l2Client.BlockNumber(testCtx)
		require.NoError(t, err)
		require.NoError(t, utils.WaitFor(testCtx, 500*time.Millisecond, func() (bool, error) {
			l2Header := testSuite.Indexer.L2Processor.LatestProcessedHeader()
			return l2Header != nil && l2Header.Number.Uint64() >= l2Height, nil
		}))

		aliceDeposits, err = testSuite.DB.BridgeTransfers.BridgeDepositsByAddress(aliceAddr)
		require.NoError(t, err)
		require.Equal(t, depositTxHash, aliceDeposits[0].FinalizedL2TransactionHash)
		require.NotNil(t, aliceDeposits[0].BridgeDeposit.FinalizedL2EventGUID)
	})

	t.Run("indexes ETH withdrawals", func(t *testing.T) {
		testCtx, cancel := context.WithTimeout(context.Background(), time.Minute)
		defer cancel()

		// (1) Test Withdrawal Initiation
		withdrawalReceipt, err := utils.WaitReceiptOK(testCtx, l2Client, withdrawTx.Hash())
		require.NoError(t, err)

		// wait for processor catchup
		require.NoError(t, utils.WaitFor(testCtx, 500*time.Millisecond, func() (bool, error) {
			l2Header := testSuite.Indexer.L2Processor.LatestProcessedHeader()
			return l2Header != nil && l2Header.Number.Uint64() >= withdrawalReceipt.BlockNumber.Uint64(), nil
		}))

		aliceWithdrawals, err := testSuite.DB.BridgeTransfers.BridgeWithdrawalsByAddress(aliceAddr)
		require.NoError(t, err)
		require.Len(t, aliceWithdrawals, 1)
		require.Equal(t, withdrawTx.Hash(), aliceWithdrawals[0].L2TransactionHash)
		require.Empty(t, aliceWithdrawals[0].ProvenL1TransactionHash)
		require.Empty(t, aliceWithdrawals[0].FinalizedL1TransactionHash)

		msgPassed, err := withdrawals.ParseMessagePassed(withdrawalReceipt)
		require.NoError(t, err)
		withdrawalHash, err := withdrawals.WithdrawalHash(msgPassed)
		require.NoError(t, err)

		withdrawal := aliceWithdrawals[0].BridgeWithdrawal
		require.Equal(t, withdrawalHash, withdrawal.WithdrawalHash)
		require.Equal(t, processor.EthAddress, withdrawal.TokenPair.L1TokenAddress)
		require.Equal(t, processor.EthAddress, withdrawal.TokenPair.L2TokenAddress)
		require.Equal(t, big.NewInt(params.Ether), withdrawal.Tx.Amount.Int)
		require.Equal(t, aliceAddr, withdrawal.Tx.FromAddress)
		require.Equal(t, aliceAddr, withdrawal.Tx.ToAddress)
		require.Equal(t, byte(1), withdrawal.Tx.Data[0])
		require.Nil(t, withdrawal.FinalizedL1EventGUID)

		// (2) Test Withdrawal Proven. Even though `bridge_transactions_e2e_tests` already tests the 2-step withdrawal flow,
		// we do the same here for good measure.

		// prove & wait for processor catchup
		withdrawParams, proveReceipt := op_e2e.ProveWithdrawal(t, *testSuite.OpCfg, l1Client, testSuite.OpSys.Nodes["sequencer"], testSuite.OpCfg.Secrets.Alice, withdrawalReceipt)
		require.NoError(t, utils.WaitFor(testCtx, 500*time.Millisecond, func() (bool, error) {
			l1Header := testSuite.Indexer.L1Processor.LatestProcessedHeader()
			return l1Header != nil && l1Header.Number.Uint64() >= proveReceipt.BlockNumber.Uint64(), nil
		}))

		aliceWithdrawals, err = testSuite.DB.BridgeTransfers.BridgeWithdrawalsByAddress(aliceAddr)
		require.NoError(t, err)
		require.Empty(t, aliceWithdrawals[0].FinalizedL1TransactionHash)
		require.Equal(t, proveReceipt.TxHash, aliceWithdrawals[0].ProvenL1TransactionHash)

		// (3) Test Withdrawal Finalization

		// finalize & wait for processor catchup
		finalizeReceipt := op_e2e.FinalizeWithdrawal(t, *testSuite.OpCfg, l1Client, testSuite.OpCfg.Secrets.Alice, withdrawalReceipt, withdrawParams)
		require.NoError(t, utils.WaitFor(testCtx, 500*time.Millisecond, func() (bool, error) {
			l1Header := testSuite.Indexer.L1Processor.LatestProcessedHeader()
			return l1Header != nil && l1Header.Number.Uint64() >= finalizeReceipt.BlockNumber.Uint64(), nil
		}))

		aliceWithdrawals, err = testSuite.DB.BridgeTransfers.BridgeWithdrawalsByAddress(aliceAddr)
		require.NoError(t, err)
		require.Equal(t, finalizeReceipt.TxHash, aliceWithdrawals[0].FinalizedL1TransactionHash)
	})
}
