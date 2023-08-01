package contracts

import (
	"fmt"

	"github.com/ethereum/go-ethereum/common"
	"github.com/urfave/cli/v2"
)

var (
	// EIP1976ImplementationSlot
	EIP1967ImplementationSlot = common.HexToHash("0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc")
	// EIP1967AdminSlot
	EIP1967AdminSlot = common.HexToHash("0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103")
)

// parseAddress will parse a [common.Address] from a [cli.Context] and return
// an error if the configured address is not correct.
func parseAddress(ctx *cli.Context, name string) (common.Address, error) {
	value := ctx.String(name)
	if value == "" {
		return common.Address{}, nil
	}
	if !common.IsHexAddress(value) {
		return common.Address{}, fmt.Errorf("invalid address: %s", value)
	}
	return common.HexToAddress(value), nil
}
