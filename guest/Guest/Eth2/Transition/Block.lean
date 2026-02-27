/-
  Block Processing — Dispatcher

  Calls all block processing sub-functions in spec order.
  Reference: https://eth2book.info/latest/part3/transition/block/
-/
import Guest.Eth2.Transition.Block.Header
import Guest.Eth2.Transition.Block.Randao
import Guest.Eth2.Transition.Block.Eth1Data
import Guest.Eth2.Transition.Block.Operations
import Guest.Eth2.Transition.Block.SyncAggregate
import Guest.Eth2.Transition.Block.Withdrawals
import Guest.Eth2.Transition.Block.ExecutionPayload

namespace Eth2

def processBlock (state : BeaconState) (block : BeaconBlock) : STFResult BeaconState := do
  let state ← processBlockHeader state block
  let state ← processWithdrawals state block.body.executionPayload
  let state ← processExecutionPayload state block.body.executionPayload
  let state ← processRandao state block.body
  let state ← processEth1Data state block.body
  let state ← processOperations state block.body
  let state ← processSyncAggregate state block.body.syncAggregate
  .ok state

end Eth2
