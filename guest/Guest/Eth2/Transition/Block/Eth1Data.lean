/-
  Block Processing — Eth1 Data

  Record the Eth1 data vote from the block.
  Reference: https://eth2book.info/latest/part3/transition/block/#eth1-data
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Transition.Block.Header

namespace Eth2

def processEth1Data (state : BeaconState) (body : BeaconBlockBody) : STFResult BeaconState :=
  let votes := state.eth1DataVotes.push body.eth1Data
  -- Count matching votes
  let mut count : UInt64 := 0
  for vote in votes do
    if vote.depositRoot == body.eth1Data.depositRoot &&
       vote.depositCount == body.eth1Data.depositCount &&
       vote.blockHash == body.eth1Data.blockHash then
      count := count + 1
  -- Update eth1_data if majority
  let threshold := EPOCHS_PER_ETH1_VOTING_PERIOD * SLOTS_PER_EPOCH
  let state :=
    if count * 2 > threshold then
      { state with eth1Data := body.eth1Data, eth1DataVotes := votes }
    else
      { state with eth1DataVotes := votes }
  .ok state

end Eth2
