/-
  Block Processing — BLS to Execution Change (Capella)

  Reference: https://eth2book.info/latest/part3/transition/block/#bls-to-execution-changes
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Transition.Block.Header

namespace Eth2

-- ETH1 address prefix for withdrawal credentials
private def ETH1_ADDRESS_WITHDRAWAL_PREFIX : UInt8 := 0x01

def processBlsToExecutionChange (state : BeaconState) (change : SignedBLSToExecutionChange) : STFResult BeaconState :=
  let msg := change.message
  let validatorIdx := msg.validatorIndex.toNat
  if h : validatorIdx < state.validators.size then
    let validator := state.validators[validatorIdx]
    -- Verify current withdrawal credentials start with BLS_WITHDRAWAL_PREFIX (0x00)
    if validator.withdrawalCredentials.size > 0 && validator.withdrawalCredentials.get! 0 != 0x00 then
      .error "bls_to_execution: wrong withdrawal prefix"
    else
      -- Stub: skip BLS pubkey match and signature verification
      -- Build new withdrawal credentials: 0x01 + 11 zero bytes + 20-byte address
      let mut newCreds := ByteArray.mkEmpty 32
      newCreds := newCreds.push ETH1_ADDRESS_WITHDRAWAL_PREFIX
      for _ in [:11] do
        newCreds := newCreds.push 0
      newCreds := newCreds ++ msg.toExecutionAddress
      -- Pad to 32 bytes if needed
      while newCreds.size < 32 do
        newCreds := newCreds.push 0
      let newValidator := { validator with withdrawalCredentials := newCreds }
      .ok { state with validators := state.validators.set ⟨validatorIdx, h⟩ newValidator }
  else .error "bls_to_execution: index out of range"

end Eth2
