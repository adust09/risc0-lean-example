/-
  Epoch Processing — Slashings

  Apply slashing penalties proportional to total slashed balance.
  Reference: https://eth2book.info/latest/part3/transition/epoch/#slashings
-/
import Guest.Eth2.Helpers

namespace Eth2

def processSlashings (state : BeaconState) : BeaconState := Id.run do
  let currentEpoch := getCurrentEpoch state
  let totalBalance := getTotalActiveBalance state
  -- Sum all slashings across the vector
  let mut totalSlashings : Gwei := 0
  for s in state.slashings do
    totalSlashings := totalSlashings + s
  -- Apply adjusted slashing penalty
  let adjustedTotalSlashingBalance :=
    let scaled := totalSlashings * PROPORTIONAL_SLASHING_MULTIPLIER_BELLATRIX
    if scaled > totalBalance then totalBalance else scaled
  let mut state := state
  for i in [:state.validators.size] do
    if i < state.validators.size then
      let validator := state.validators[i]!
      if validator.slashed &&
         currentEpoch + EPOCHS_PER_SLASHINGS_VECTOR / 2 == validator.withdrawableEpoch then
        let increment := EFFECTIVE_BALANCE_INCREMENT
        let penaltyNumerator := validator.effectiveBalance / increment * adjustedTotalSlashingBalance
        let penalty := penaltyNumerator / totalBalance * increment
        state := decreaseBalance state i.toUInt64 penalty
  return state

end Eth2
