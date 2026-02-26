/-
  Block Processing — Deposit Processing

  Reference: https://eth2book.info/latest/part3/transition/block/#deposits
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Crypto
import Guest.Eth2.Transition.Block.Header

namespace Eth2

-- Apply a deposit: either add a new validator or top up an existing one
private def applyDeposit (state : BeaconState) (deposit : Deposit) : BeaconState :=
  let pubkey := deposit.data.pubkey
  let amount := deposit.data.amount
  -- Search for existing validator with this pubkey
  let mut existingIdx : Option Nat := none
  for i in [:state.validators.size] do
    if h : i < state.validators.size then
      if state.validators[i].pubkey == pubkey then
        existingIdx := some i
        break
  match existingIdx with
  | some idx =>
    -- Top up existing validator
    increaseBalance state idx.toUInt64 amount
  | none =>
    -- Add new validator
    let validator : Validator := {
      pubkey := pubkey
      withdrawalCredentials := deposit.data.withdrawalCredentials
      effectiveBalance := min (amount - amount % EFFECTIVE_BALANCE_INCREMENT) MAX_EFFECTIVE_BALANCE
      slashed := false
      activationEligibilityEpoch := FAR_FUTURE_EPOCH
      activationEpoch := FAR_FUTURE_EPOCH
      exitEpoch := FAR_FUTURE_EPOCH
      withdrawableEpoch := FAR_FUTURE_EPOCH
    }
    { state with
      validators := state.validators.push validator
      balances := state.balances.push amount
      previousEpochParticipation := state.previousEpochParticipation.push 0
      currentEpochParticipation := state.currentEpochParticipation.push 0
      inactivityScores := state.inactivityScores.push 0
    }

def processDeposit (state : BeaconState) (deposit : Deposit) : STFResult BeaconState :=
  -- Stub: skip Merkle proof verification
  -- Stub: skip BLS signature verification for new deposits
  let state := applyDeposit state deposit
  let state := { state with eth1DepositIndex := state.eth1DepositIndex + 1 }
  .ok state

end Eth2
