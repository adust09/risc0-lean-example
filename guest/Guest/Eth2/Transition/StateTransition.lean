/-
  State Transition — Top-level

  Implements state_transition, process_slots, process_slot.
  Reference: https://eth2book.info/latest/part3/transition/
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Crypto
import Guest.Eth2.Transition.Epoch
import Guest.Eth2.Transition.Block

namespace Eth2

-- Process a single slot: cache state root, block root, epoch boundary
def processSlot (state : BeaconState) : BeaconState :=
  -- Cache state root
  let previousStateRoot := hashTreeRoot (ByteArray.mk #[])  -- stub
  let idx := (state.slot % SLOTS_PER_HISTORICAL_ROOT).toNat
  let state :=
    if idx < state.stateRoots.size then
      { state with stateRoots := state.stateRoots.set! idx previousStateRoot }
    else state
  -- Cache latest block header's state root (if empty)
  let state :=
    if state.latestBlockHeader.stateRoot == ByteArray.mk (Array.replicate 32 0) then
      { state with latestBlockHeader := { state.latestBlockHeader with stateRoot := previousStateRoot } }
    else state
  -- Cache block root
  let previousBlockRoot := hashTreeRoot (ByteArray.mk #[])  -- stub
  let state :=
    if idx < state.blockRoots.size then
      { state with blockRoots := state.blockRoots.set! idx previousBlockRoot }
    else state
  state

-- Advance state through slots up to (but not including) the target slot
-- Process epoch transitions at epoch boundaries
partial def processSlots (state : BeaconState) (targetSlot : Slot) : STFResult BeaconState :=
  if targetSlot <= state.slot then
    .error "process_slots: target_slot <= state.slot"
  else
    let result := Id.run do
      let mut state := state
      while state.slot < targetSlot do
        state := processSlot state
        -- Process epoch at epoch boundary (slot + 1 is start of new epoch)
        if (state.slot + 1) % SLOTS_PER_EPOCH == 0 then
          state := processEpoch state
        state := { state with slot := state.slot + 1 }
      return state
    .ok result

-- Full state transition: advance slots + process block
def stateTransition (state : BeaconState) (signedBlock : SignedBeaconBlock)
    (validateResult : Bool := true) : STFResult BeaconState := do
  let block := signedBlock.message
  -- Process slots up to the block's slot
  let state ← processSlots state block.slot
  -- Stub: skip block signature verification
  -- Process the block
  let state ← processBlock state block
  -- Stub: skip state root verification (would compare block.stateRoot with hashTreeRoot(state))
  .ok state

end Eth2
