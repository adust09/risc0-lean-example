/-
  Epoch Processing — Justification and Finalization

  Implements the Casper FFG finality gadget.
  Reference: https://eth2book.info/latest/part3/transition/epoch/#justification-and-finalization
-/
import Guest.Eth2.Helpers

namespace Eth2

-- Get bit from justification bitvector (bit 0 = current epoch - 1)
private def getJustificationBit (bits : ByteArray) (index : Nat) : Bool :=
  if 0 < bits.size then
    let byte := bits.get! 0
    (byte.toNat >>> index) &&& 1 == 1
  else false

-- Set bit in justification bitvector
private def setJustificationBit (bits : ByteArray) (index : Nat) : ByteArray :=
  if 0 < bits.size then
    let byte := bits.get! 0
    let newByte := byte ||| (1 <<< index).toUInt8
    bits.set! 0 newByte
  else bits

-- Shift justification bits left by 1 (new epoch enters at bit 0)
private def shiftJustificationBits (bits : ByteArray) : ByteArray :=
  if 0 < bits.size then
    let byte := bits.get! 0
    -- Shift left by 1, mask to 4 bits
    let newByte := (byte <<< 1) &&& 0x0F
    bits.set! 0 newByte
  else bits

-- Compute total balance of validators with a specific participation flag set
private def getParticipatingBalance (state : BeaconState) (epoch : Epoch)
    (flagIndex : Nat) : Gwei := Id.run do
  let participation := if epoch == getCurrentEpoch state
    then state.currentEpochParticipation
    else state.previousEpochParticipation
  let activeIndices := getActiveValidatorIndices state epoch
  let mut total : Gwei := 0
  for idx in activeIndices do
    let i := idx.toNat
    if i < participation.size then
      if hasFlag participation[i]! flagIndex then
        if i < state.validators.size then
          total := total + state.validators[i]!.effectiveBalance
  if total < EFFECTIVE_BALANCE_INCREMENT then EFFECTIVE_BALANCE_INCREMENT else total

def processJustificationAndFinalization (state : BeaconState) : BeaconState :=
  let currentEpoch := getCurrentEpoch state
  -- Skip for first two epochs
  if currentEpoch <= 1 then state
  else
    let previousEpoch := getPreviousEpoch state
    let totalActiveBalance := getTotalActiveBalance state
    -- Target balance = validators that attested to correct target
    let previousTargetBalance := getParticipatingBalance state previousEpoch TIMELY_TARGET_FLAG_INDEX
    let currentTargetBalance := getParticipatingBalance state currentEpoch TIMELY_TARGET_FLAG_INDEX
    -- Shift justification bits
    let bits := shiftJustificationBits state.justificationBits
    let state := { state with justificationBits := bits }
    -- Justify previous epoch if 2/3 supermajority
    let state :=
      if previousTargetBalance * 3 >= totalActiveBalance * 2 then
        { state with
          justificationBits := setJustificationBit state.justificationBits 1
          previousJustifiedCheckpoint := {
            epoch := previousEpoch
            root := getBlockRoot state previousEpoch
          }
        }
      else state
    -- Justify current epoch if 2/3 supermajority
    let state :=
      if currentTargetBalance * 3 >= totalActiveBalance * 2 then
        { state with
          justificationBits := setJustificationBit state.justificationBits 0
          currentJustifiedCheckpoint := {
            epoch := currentEpoch
            root := getBlockRoot state currentEpoch
          }
        }
      else state
    -- Finalization rules (check 2/3/4 epoch chains)
    let bits := state.justificationBits
    -- Rule 1: epochs (n-3, n-2, n-1, n) all justified → finalize n-3
    let state :=
      if getJustificationBit bits 1 && getJustificationBit bits 2 && getJustificationBit bits 3 then
        if state.previousJustifiedCheckpoint.epoch + 3 == currentEpoch then
          { state with finalizedCheckpoint := state.previousJustifiedCheckpoint }
        else state
      else state
    -- Rule 2: epochs (n-2, n-1, n) justified → finalize n-2
    let state :=
      if getJustificationBit bits 1 && getJustificationBit bits 2 then
        if state.previousJustifiedCheckpoint.epoch + 2 == currentEpoch then
          { state with finalizedCheckpoint := state.previousJustifiedCheckpoint }
        else state
      else state
    -- Rule 3: epochs (n-2, n-1, n) with current justified → finalize n-2
    let state :=
      if getJustificationBit bits 0 && getJustificationBit bits 1 && getJustificationBit bits 2 then
        if state.currentJustifiedCheckpoint.epoch + 2 == currentEpoch then
          { state with finalizedCheckpoint := state.currentJustifiedCheckpoint }
        else state
      else state
    -- Rule 4: epochs (n-1, n) justified → finalize n-1
    let state :=
      if getJustificationBit bits 0 && getJustificationBit bits 1 then
        if state.currentJustifiedCheckpoint.epoch + 1 == currentEpoch then
          { state with finalizedCheckpoint := state.currentJustifiedCheckpoint }
        else state
      else state
    state

end Eth2
