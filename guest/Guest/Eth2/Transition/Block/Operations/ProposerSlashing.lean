/-
  Block Processing — Proposer Slashing

  Reference: https://eth2book.info/latest/part3/transition/block/#proposer-slashings
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Crypto
import Guest.Eth2.Transition.Block.Header

namespace Eth2

def processProposerSlashing (state : BeaconState) (slashing : ProposerSlashing) : STFResult BeaconState :=
  let header1 := slashing.signedHeader1.message
  let header2 := slashing.signedHeader2.message
  -- Verify headers are different
  if header1.slot == header2.slot &&
     header1.proposerIndex == header2.proposerIndex &&
     header1.parentRoot == header2.parentRoot &&
     header1.stateRoot == header2.stateRoot &&
     header1.bodyRoot == header2.bodyRoot then
    .error "proposer slashing: headers are equal"
  -- Verify same proposer
  else if header1.proposerIndex != header2.proposerIndex then
    .error "proposer slashing: different proposer indices"
  -- Verify same slot
  else if header1.slot != header2.slot then
    .error "proposer slashing: different slots"
  else
    let proposerIdx := header1.proposerIndex.toNat
    if h : proposerIdx < state.validators.size then
      let proposer := state.validators[proposerIdx]
      -- Verify proposer is slashable
      if !isSlashableValidator proposer (getCurrentEpoch state) then
        .error "proposer slashing: validator not slashable"
      else
        -- Stub: skip BLS signature verification
        .ok (slashValidator state header1.proposerIndex none)
    else .error "proposer slashing: index out of range"

end Eth2
