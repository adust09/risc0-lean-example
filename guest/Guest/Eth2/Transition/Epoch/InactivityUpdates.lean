/-
  Epoch Processing — Inactivity Score Updates (Altair)

  Reference: https://eth2book.info/latest/part3/transition/epoch/#inactivity-scores
-/
import Guest.Eth2.Helpers

namespace Eth2

-- Check if the chain is in an inactivity leak
-- (more than MIN_EPOCHS_TO_INACTIVITY_PENALTY epochs since finalization)
def isInInactivityLeak (state : BeaconState) : Bool :=
  let currentEpoch := getCurrentEpoch state
  let finalizedEpoch := state.finalizedCheckpoint.epoch
  currentEpoch > finalizedEpoch + MIN_EPOCHS_TO_INACTIVITY_PENALTY

def processInactivityUpdates (state : BeaconState) : BeaconState := Id.run do
  let currentEpoch := getCurrentEpoch state
  -- Skip genesis epoch
  if currentEpoch == 0 then return state
  let previousEpoch := getPreviousEpoch state
  let activeIndices := getActiveValidatorIndices state currentEpoch
  let inLeak := isInInactivityLeak state
  let mut scores := state.inactivityScores
  for idx in activeIndices do
    let i := idx.toNat
    if i < scores.size then
      let score := scores[i]!
      -- Check if validator had timely target participation in previous epoch
      let participated :=
        if i < state.previousEpochParticipation.size then
          hasFlag state.previousEpochParticipation[i]! TIMELY_TARGET_FLAG_INDEX
        else false
      let newScore :=
        if participated then
          -- Decrease score towards 0
          if score >= INACTIVITY_SCORE_RECOVERY_RATE then
            score - INACTIVITY_SCORE_RECOVERY_RATE
          else 0
        else
          -- Increase score if in leak
          if inLeak then score + INACTIVITY_SCORE_BIAS
          else score
      scores := scores.set! i newScore
  -- Ignore previousEpoch usage warning - it's used conceptually
  return { state with inactivityScores := scores }

end Eth2
