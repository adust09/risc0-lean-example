/-
  Epoch Processing — Reset Functions

  Three reset functions that rotate circular buffers at epoch boundaries:
  - eth1_data_reset: clear eth1 votes at voting period boundary
  - slashings_reset: zero out the current epoch's slashings slot
  - randao_mixes_reset: copy current mix to next epoch slot

  Reference: https://eth2book.info/latest/part3/transition/epoch/
-/
import Guest.Eth2.Helpers

namespace Eth2

-- Reset eth1 data votes at the start of each voting period
def processEth1DataReset (state : BeaconState) : BeaconState :=
  let nextEpoch := getCurrentEpoch state + 1
  if nextEpoch % EPOCHS_PER_ETH1_VOTING_PERIOD == 0 then
    { state with eth1DataVotes := #[] }
  else state

-- Zero out the next epoch's slashings accumulator
def processSlashingsReset (state : BeaconState) : BeaconState :=
  let nextEpoch := getCurrentEpoch state + 1
  let idx := (nextEpoch % EPOCHS_PER_SLASHINGS_VECTOR).toNat
  if idx < state.slashings.size then
    { state with slashings := state.slashings.set! idx 0 }
  else state

-- Copy current epoch's randao mix to next epoch's slot
def processRandaoMixesReset (state : BeaconState) : BeaconState :=
  let currentEpoch := getCurrentEpoch state
  let nextEpoch := currentEpoch + 1
  let currentMix := getRandaoMix state currentEpoch
  let idx := (nextEpoch % EPOCHS_PER_HISTORICAL_VECTOR).toNat
  if idx < state.randaoMixes.size then
    { state with randaoMixes := state.randaoMixes.set! idx currentMix }
  else state

end Eth2
