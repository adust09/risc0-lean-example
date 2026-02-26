/-
  Block Processing — Attestation Processing (Altair)

  Updates participation flags based on attestation data.
  Reference: https://eth2book.info/latest/part3/transition/block/#attestations
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Crypto
import Guest.Eth2.Transition.Block.Header

namespace Eth2

def processAttestation (state : BeaconState) (attestation : Attestation) : STFResult BeaconState :=
  let data := attestation.data
  let currentEpoch := getCurrentEpoch state
  let previousEpoch := getPreviousEpoch state
  -- Verify target epoch is previous or current
  if data.target.epoch != previousEpoch && data.target.epoch != currentEpoch then
    .error "attestation: invalid target epoch"
  else
    -- Verify slot is within bounds
    let attestationEpoch := computeEpochAtSlot data.slot
    if attestationEpoch != data.target.epoch then
      .error "attestation: slot/epoch mismatch"
    else
      -- Stub: skip committee validation, signature verification
      -- Determine which participation array to update
      let isPreviousEpoch := data.target.epoch == previousEpoch
      let inclusionDelay := state.slot - data.slot
      -- Determine participation flags from inclusion delay and correctness
      let justifiedCheckpoint := if isPreviousEpoch
        then state.previousJustifiedCheckpoint
        else state.currentJustifiedCheckpoint
      let isMatchingSource := data.source.epoch == justifiedCheckpoint.epoch
      let isMatchingTarget := isMatchingSource &&
        data.target.root == getBlockRoot state data.target.epoch
      let isMatchingHead := isMatchingTarget &&
        data.beaconBlockRoot == getBlockRootAtSlot state data.slot
      -- Update participation flags for all attesting validators
      -- Stub: use aggregation_bits length as validator count, indices are 0..n
      let numBits := attestation.aggregationBits.size * 8
      let result := Id.run do
        let mut state := state
        let _proposerRewardNumerator : UInt64 := 0
        -- Simplified: process a single attesting validator (the proposer) as a stand-in
        -- In production, this would iterate over all committee members indicated by aggregation_bits
        let mut proposerRewardNum : UInt64 := 0
        for bitIdx in [:numBits] do
          let byteIdx := bitIdx / 8
          let bitPos := bitIdx % 8
          let isSet := if byteIdx < attestation.aggregationBits.size then
            (attestation.aggregationBits.get! byteIdx).toNat >>> bitPos &&& 1 == 1
          else false
          if isSet then
            -- Stub: map bit index to validator index (simplified as direct mapping)
            let validatorIdx := bitIdx.toUInt64
            let i := validatorIdx.toNat
            -- Update participation flags
            let participation := if isPreviousEpoch
              then state.previousEpochParticipation
              else state.currentEpochParticipation
            if i < participation.size then
              let flags := participation[i]!
              let mut newFlags := flags
              if isMatchingSource && inclusionDelay <= (integerSquareroot SLOTS_PER_EPOCH) then
                if !hasFlag flags TIMELY_SOURCE_FLAG_INDEX then
                  newFlags := addFlag newFlags TIMELY_SOURCE_FLAG_INDEX
                  proposerRewardNum := proposerRewardNum + getBaseReward state validatorIdx * TIMELY_SOURCE_WEIGHT
              if isMatchingTarget then
                if !hasFlag flags TIMELY_TARGET_FLAG_INDEX then
                  newFlags := addFlag newFlags TIMELY_TARGET_FLAG_INDEX
                  proposerRewardNum := proposerRewardNum + getBaseReward state validatorIdx * TIMELY_TARGET_WEIGHT
              if isMatchingHead && inclusionDelay == MIN_ATTESTATION_INCLUSION_DELAY then
                if !hasFlag flags TIMELY_HEAD_FLAG_INDEX then
                  newFlags := addFlag newFlags TIMELY_HEAD_FLAG_INDEX
                  proposerRewardNum := proposerRewardNum + getBaseReward state validatorIdx * TIMELY_HEAD_WEIGHT
              if newFlags != flags then
                let newParticipation := participation.set! i newFlags
                state := if isPreviousEpoch
                  then { state with previousEpochParticipation := newParticipation }
                  else { state with currentEpochParticipation := newParticipation }
        -- Reward proposer
        let proposerReward := proposerRewardNum / (WEIGHT_DENOMINATOR - PROPOSER_WEIGHT) * PROPOSER_WEIGHT / WEIGHT_DENOMINATOR
        let proposerIndex := getBeaconProposerIndex state
        state := increaseBalance state proposerIndex proposerReward
        return state
      .ok result

end Eth2
