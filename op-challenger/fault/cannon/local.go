package cannon

import (
	"context"
	"fmt"
	"math/big"

	"github.com/ethereum-optimism/optimism/op-bindings/bindings"
	"github.com/ethereum-optimism/optimism/op-challenger/config"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/ethclient"
)

type localGameInputs struct {
	l1Head        common.Hash
	l2ChainId     *big.Int
	l2Head        common.Hash
	l2OutputRoot  common.Hash
	l2Claim       common.Hash
	l2BlockNumber *big.Int
}

func fetchLocalInputs(ctx context.Context, cfg *config.Config, l1Client bind.ContractCaller, l2Client *ethclient.Client) (localGameInputs, error) {
	caller, err := bindings.NewFaultDisputeGameCaller(cfg.GameAddress, l1Client)
	if err != nil {
		return localGameInputs{}, fmt.Errorf("create caller for game %v: %w", cfg.GameAddress, err)
	}
	opts := &bind.CallOpts{Context: ctx}
	l1Head, err := caller.L1Head(opts)
	if err != nil {
		return localGameInputs{}, fmt.Errorf("fetch L1 head for game %v: %w", cfg.GameAddress, err)
	}
	l2BlockNumber, err := caller.L2BlockNumber(opts)
	if err != nil {
		return localGameInputs{}, fmt.Errorf("fetch L2 block number for game %v: %w", cfg.GameAddress, err)
	}
	l2Block, err := l2Client.HeaderByNumber(ctx, l2BlockNumber)
	if err != nil {
		return localGameInputs{}, fmt.Errorf("fetch L2 block header %v: %w", l2BlockNumber, err)
	}
	l2Head := l2Block.Hash()

	l2ChainId, err := l2Client.ChainID(ctx)
	if err != nil {
		return localGameInputs{}, fmt.Errorf("fetch L2 chain ID: %w", err)
	}
	l2ooAddr, err := caller.L2OUTPUTORACLE(opts)
	if err != nil {
		return localGameInputs{}, fmt.Errorf("fetch l2oo address for game %v: %w", cfg.GameAddress, err)
	}
	l2oo, err := bindings.NewL2OutputOracleCaller(l2ooAddr, l1Client)
	if err != nil {
		return localGameInputs{}, fmt.Errorf("create l2oo caller at addr %v: %w", l2ooAddr, err)
	}

	// Assuming that l2BlockNumber is the agreed starting point and we're challenging the output root after it
	// This may not be correct and may be something we want to change.
	l2ooIndex, err := l2oo.GetL2OutputIndexAfter(opts, l2BlockNumber)
	if err != nil {
		return localGameInputs{}, fmt.Errorf("create l2oo caller at addr %v: %w", l2ooAddr, err)
	}
	agreedOutput, err := l2oo.GetL2Output(opts, l2ooIndex)
	if err != nil {
		return localGameInputs{}, fmt.Errorf("get agreed output from l2oo at %v: %w", l2ooAddr, err)
	}
	claimedOutput, err := l2oo.GetL2Output(opts, new(big.Int).Add(l2ooIndex, common.Big1))
	if err != nil {
		return localGameInputs{}, fmt.Errorf("get agreed output from l2oo at %v: %w", l2ooAddr, err)
	}

	return localGameInputs{
		l1Head:        l1Head,
		l2ChainId:     l2ChainId,
		l2Head:        l2Head,
		l2OutputRoot:  agreedOutput.OutputRoot,
		l2Claim:       claimedOutput.OutputRoot,
		l2BlockNumber: claimedOutput.L2BlockNumber,
	}, nil
}
