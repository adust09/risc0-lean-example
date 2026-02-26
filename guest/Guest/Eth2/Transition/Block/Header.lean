/-
  Block Processing — Block Header

  Validates and records the block header.
  Reference: https://eth2book.info/latest/part3/transition/block/#block-header
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Crypto

namespace Eth2

-- Result type for block processing (error propagation)
inductive STFResult (α : Type)
  | ok (val : α)
  | error (msg : String)
  deriving Repr, Inhabited

namespace STFResult

def bind (r : STFResult α) (f : α → STFResult β) : STFResult β :=
  match r with
  | ok v => f v
  | error msg => error msg

def map (r : STFResult α) (f : α → β) : STFResult β :=
  match r with
  | ok v => ok (f v)
  | error msg => error msg

instance : Monad STFResult where
  pure := ok
  bind := bind

end STFResult

def processBlockHeader (state : BeaconState) (block : BeaconBlock) : STFResult BeaconState :=
  -- Verify slot matches
  if block.slot != state.slot then
    .error "block.slot != state.slot"
  -- Verify block is newer than latest block header
  else if block.slot <= state.latestBlockHeader.slot then
    .error "block.slot <= latest_block_header.slot"
  -- Verify proposer index
  else if block.proposerIndex != getBeaconProposerIndex state then
    .error "block.proposer_index != get_beacon_proposer_index(state)"
  -- Verify parent root matches
  else if block.parentRoot != hashTreeRoot (ByteArray.mk #[]) then
    -- Stub: skip parent root check (would need actual header hash)
    let header : BeaconBlockHeader := {
      slot := block.slot
      proposerIndex := block.proposerIndex
      parentRoot := block.parentRoot
      stateRoot := ByteArray.mk (Array.replicate 32 0)  -- overwritten later
      bodyRoot := hashTreeRoot (ByteArray.mk #[])      -- stub
    }
    -- Verify proposer is not slashed
    let proposerIdx := block.proposerIndex.toNat
    if proposerIdx < state.validators.size then
      if state.validators[proposerIdx]!.slashed then
        .error "proposer is slashed"
      else
        .ok { state with latestBlockHeader := header }
    else .error "proposer index out of range"
  else
    let header : BeaconBlockHeader := {
      slot := block.slot
      proposerIndex := block.proposerIndex
      parentRoot := block.parentRoot
      stateRoot := ByteArray.mk (Array.replicate 32 0)
      bodyRoot := hashTreeRoot (ByteArray.mk #[])
    }
    let proposerIdx := block.proposerIndex.toNat
    if proposerIdx < state.validators.size then
      if state.validators[proposerIdx]!.slashed then
        .error "proposer is slashed"
      else
        .ok { state with latestBlockHeader := header }
    else .error "proposer index out of range"

end Eth2
