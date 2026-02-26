/-
  Block Processing — Withdrawals (Capella)

  Process validator withdrawals from execution payload.
  Reference: https://eth2book.info/latest/part3/transition/block/#withdrawals
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Transition.Block.Header

namespace Eth2

-- Check if a validator has ETH1 withdrawal credentials (0x01 prefix)
private def hasEth1WithdrawalCredential (validator : Validator) : Bool :=
  validator.withdrawalCredentials.size > 0 &&
  validator.withdrawalCredentials.get! 0 == 0x01

-- Check if a validator is fully withdrawable
private def isFullyWithdrawable (validator : Validator) (balance : Gwei) (epoch : Epoch) : Bool :=
  hasEth1WithdrawalCredential validator &&
  validator.withdrawableEpoch <= epoch &&
  balance > 0

-- Check if a validator is partially withdrawable
private def isPartiallyWithdrawable (validator : Validator) (balance : Gwei) : Bool :=
  hasEth1WithdrawalCredential validator &&
  validator.effectiveBalance == MAX_EFFECTIVE_BALANCE &&
  balance > MAX_EFFECTIVE_BALANCE

-- Compute expected withdrawals (spec: get_expected_withdrawals)
def getExpectedWithdrawals (state : BeaconState) : Array Withdrawal :=
  let epoch := getCurrentEpoch state
  let mut withdrawalIndex := state.nextWithdrawalIndex
  let mut validatorIndex := state.nextWithdrawalValidatorIndex
  let mut withdrawals : Array Withdrawal := #[]
  let bound := min state.validators.size MAX_VALIDATORS_PER_WITHDRAWALS_SWEEP.toNat
  let mut numChecked : Nat := 0
  while numChecked < bound && withdrawals.size < MAX_WITHDRAWALS_PER_PAYLOAD do
    let i := validatorIndex.toNat
    if h : i < state.validators.size then
      let validator := state.validators[i]
      let balance := if h2 : i < state.balances.size then state.balances[i] else 0
      if isFullyWithdrawable validator balance epoch then
        let addr := if validator.withdrawalCredentials.size >= 32
          then validator.withdrawalCredentials.extract 12 32
          else ByteArray.mk (Array.mkArray 20 0)
        withdrawals := withdrawals.push {
          index := withdrawalIndex
          validatorIndex := validatorIndex
          address := addr
          amount := balance
        }
        withdrawalIndex := withdrawalIndex + 1
      else if isPartiallyWithdrawable validator balance then
        let addr := if validator.withdrawalCredentials.size >= 32
          then validator.withdrawalCredentials.extract 12 32
          else ByteArray.mk (Array.mkArray 20 0)
        withdrawals := withdrawals.push {
          index := withdrawalIndex
          validatorIndex := validatorIndex
          address := addr
          amount := balance - MAX_EFFECTIVE_BALANCE
        }
        withdrawalIndex := withdrawalIndex + 1
    validatorIndex := if validatorIndex + 1 >= state.validators.size.toUInt64
      then 0
      else validatorIndex + 1
    numChecked := numChecked + 1
  withdrawals

def processWithdrawals (state : BeaconState) (payload : ExecutionPayload) : STFResult BeaconState :=
  let expectedWithdrawals := getExpectedWithdrawals state
  -- Verify withdrawal count matches
  if payload.withdrawals.size != expectedWithdrawals.size then
    .error "withdrawals: count mismatch"
  else
    let mut state := state
    for withdrawal in expectedWithdrawals do
      state := decreaseBalance state withdrawal.validatorIndex withdrawal.amount
    -- Update withdrawal indices
    let nextIndex := if expectedWithdrawals.size > 0 then
        match expectedWithdrawals.back? with
        | some w => w.index + 1
        | none => state.nextWithdrawalIndex
      else state.nextWithdrawalIndex
    let numChecked := min state.validators.size MAX_VALIDATORS_PER_WITHDRAWALS_SWEEP.toNat
    let nextValidatorIdx :=
      if state.validators.size == 0 then 0
      else (state.nextWithdrawalValidatorIndex.toNat + numChecked) % state.validators.size |>.toUInt64
    .ok { state with
      nextWithdrawalIndex := nextIndex
      nextWithdrawalValidatorIndex := nextValidatorIdx
    }

end Eth2
