/-
  Epoch Processing — Registry Updates

  Processes activation queue and ejections.
  Reference: https://eth2book.info/latest/part3/transition/epoch/#registry-updates
-/
import Guest.Eth2.Helpers

namespace Eth2

def processRegistryUpdates (state : BeaconState) : BeaconState :=
  let currentEpoch := getCurrentEpoch state
  let mut state := state
  -- Process eligibility and ejections
  for i in [:state.validators.size] do
    if h : i < state.validators.size then
      let validator := state.validators[i]
      -- Mark eligible for activation
      if isEligibleForActivationQueue validator then
        let newVal := { validator with activationEligibilityEpoch := currentEpoch + 1 }
        state := { state with validators := state.validators.set ⟨i, h⟩ newVal }
      -- Eject if balance too low
      if isActiveValidator validator currentEpoch &&
         validator.effectiveBalance <= EJECTION_BALANCE then
        state := initiateValidatorExit state i.toUInt64
  -- Process activation queue
  let churnLimit := getValidatorChurnLimit state
  -- Collect eligible validators with their indices
  let mut eligible : Array (Nat × Epoch) := #[]
  for i in [:state.validators.size] do
    if h : i < state.validators.size then
      if isEligibleForActivation state state.validators[i] then
        eligible := eligible.push (i, state.validators[i].activationEligibilityEpoch)
  -- Sort by activation_eligibility_epoch then index
  let eligible := eligible.qsort fun (i1, e1) (i2, e2) =>
    if e1 != e2 then e1 < e2 else i1 < i2
  -- Activate up to churn limit
  let limit := min eligible.size churnLimit.toNat
  for j in [:limit] do
    if h : j < eligible.size then
      let (i, _) := eligible[j]
      if h2 : i < state.validators.size then
        let validator := state.validators[i]
        let newVal := { validator with activationEpoch := currentEpoch + 1 }
        state := { state with validators := state.validators.set ⟨i, h2⟩ newVal }
  state

end Eth2
