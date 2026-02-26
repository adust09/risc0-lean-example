/-
  Block Processing — Voluntary Exit

  Reference: https://eth2book.info/latest/part3/transition/block/#voluntary-exits
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Transition.Block.Header

namespace Eth2

def processVoluntaryExit (state : BeaconState) (exit : SignedVoluntaryExit) : STFResult BeaconState :=
  let voluntaryExit := exit.message
  let validatorIdx := voluntaryExit.validatorIndex.toNat
  if h : validatorIdx < state.validators.size then
    let validator := state.validators[validatorIdx]
    let currentEpoch := getCurrentEpoch state
    -- Verify validator is active
    if !isActiveValidator validator currentEpoch then
      .error "voluntary exit: validator not active"
    -- Verify exit has not been initiated
    else if validator.exitEpoch != FAR_FUTURE_EPOCH then
      .error "voluntary exit: already initiated"
    -- Verify minimum epoch
    else if currentEpoch < voluntaryExit.epoch then
      .error "voluntary exit: epoch in future"
    -- Verify validator has been active long enough
    else if currentEpoch < validator.activationEpoch + SHARD_COMMITTEE_PERIOD then
      .error "voluntary exit: not long enough"
    else
      -- Stub: skip BLS signature verification
      .ok (initiateValidatorExit state voluntaryExit.validatorIndex)
  else .error "voluntary exit: index out of range"

end Eth2
