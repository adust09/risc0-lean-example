/-
  Block Processing — RANDAO

  Mix the block's RANDAO reveal into the state.
  Reference: https://eth2book.info/latest/part3/transition/block/#randao
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Crypto
import Guest.Eth2.Transition.Block.Header

namespace Eth2

-- XOR two byte arrays of equal length
private def xorBytes (a b : ByteArray) : ByteArray := Id.run do
  let len := min a.size b.size
  let mut result := ByteArray.emptyWithCapacity len
  for i in [:len] do
    result := result.push (a.get! i ^^^ b.get! i)
  return result

def processRandao (state : BeaconState) (body : BeaconBlockBody) : STFResult BeaconState :=
  -- Stub: skip RANDAO reveal BLS verification
  -- In production: verify BLS signature of epoch by proposer
  let currentEpoch := getCurrentEpoch state
  let mix := getRandaoMix state currentEpoch
  -- Mix in the reveal (XOR with hash of reveal)
  let revealHash := hashTreeRoot body.randaoReveal
  let newMix := xorBytes mix revealHash
  let idx := (currentEpoch % EPOCHS_PER_HISTORICAL_VECTOR).toNat
  if idx < state.randaoMixes.size then
    .ok { state with randaoMixes := state.randaoMixes.set! idx newMix }
  else .error "randao mix index out of range"

end Eth2
