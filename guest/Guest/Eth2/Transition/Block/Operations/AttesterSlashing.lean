/-
  Block Processing — Attester Slashing

  Reference: https://eth2book.info/latest/part3/transition/block/#attester-slashings
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Crypto
import Guest.Eth2.Transition.Block.Header

namespace Eth2

-- Check if two attestation data are slashable
-- (double vote or surround vote)
private def isSlashableAttestationData (data1 data2 : AttestationData) : Bool :=
  -- Double vote: same target epoch, different data
  (data1.target.epoch == data2.target.epoch &&
   !(data1.slot == data2.slot && data1.index == data2.index &&
     data1.beaconBlockRoot == data2.beaconBlockRoot &&
     data1.source.epoch == data2.source.epoch &&
     data1.source.root == data2.source.root &&
     data1.target.root == data2.target.root)) ||
  -- Surround vote: att1 surrounds att2
  (data1.source.epoch < data2.source.epoch && data2.target.epoch < data1.target.epoch)

def processAttesterSlashing (state : BeaconState) (slashing : AttesterSlashing) : STFResult BeaconState :=
  let att1 := slashing.attestation1
  let att2 := slashing.attestation2
  if !isSlashableAttestationData att1.data att2.data then
    .error "attester slashing: not slashable"
  else
    -- Stub: skip signature verification on indexed attestations
    -- Find intersection of attesting indices
    let mut state := state
    let mut slashedAny := false
    for idx1 in att1.attestingIndices do
      for idx2 in att2.attestingIndices do
        if idx1 == idx2 then
          let i := idx1.toNat
          if h : i < state.validators.size then
            if isSlashableValidator state.validators[i] (getCurrentEpoch state) then
              state := slashValidator state idx1 none
              slashedAny := true
    if slashedAny then .ok state
    else .error "attester slashing: no validators slashed"

end Eth2
