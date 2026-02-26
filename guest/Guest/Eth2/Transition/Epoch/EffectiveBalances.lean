/-
  Epoch Processing — Effective Balance Updates

  Update effective balances with hysteresis.
  Reference: https://eth2book.info/latest/part3/transition/epoch/#effective-balances-updates
-/
import Guest.Eth2.Helpers

namespace Eth2

-- Hysteresis parameters (spec-defined)
private def HYSTERESIS_QUOTIENT : UInt64 := 4
private def HYSTERESIS_DOWNWARD_MULTIPLIER : UInt64 := 1
private def HYSTERESIS_UPWARD_MULTIPLIER : UInt64 := 5

def processEffectiveBalanceUpdates (state : BeaconState) : BeaconState := Id.run do
  let downwardThreshold := EFFECTIVE_BALANCE_INCREMENT * HYSTERESIS_DOWNWARD_MULTIPLIER / HYSTERESIS_QUOTIENT
  let upwardThreshold := EFFECTIVE_BALANCE_INCREMENT * HYSTERESIS_UPWARD_MULTIPLIER / HYSTERESIS_QUOTIENT
  let mut validators := state.validators
  for i in [:validators.size] do
    if i < validators.size then
      let validator := validators[i]!
      if i < state.balances.size then
        let balance := state.balances[i]!
        -- Check if effective balance needs updating (with hysteresis)
        if balance + downwardThreshold < validator.effectiveBalance ||
           validator.effectiveBalance + upwardThreshold < balance then
          let newEffective := min (balance - balance % EFFECTIVE_BALANCE_INCREMENT) MAX_EFFECTIVE_BALANCE
          let newVal := { validator with effectiveBalance := newEffective }
          validators := validators.set! i newVal
  return { state with validators := validators }

end Eth2
