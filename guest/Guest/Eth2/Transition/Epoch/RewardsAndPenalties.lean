/-
  Epoch Processing — Rewards and Penalties (Altair)

  Distributes rewards and penalties based on participation flags.
  Reference: https://eth2book.info/latest/part3/transition/epoch/#rewards-and-penalties
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Transition.Epoch.InactivityUpdates

namespace Eth2

-- Flag weights for each participation flag
private def flagWeights : Array (Nat × UInt64) :=
  #[
    (TIMELY_SOURCE_FLAG_INDEX, TIMELY_SOURCE_WEIGHT),
    (TIMELY_TARGET_FLAG_INDEX, TIMELY_TARGET_WEIGHT),
    (TIMELY_HEAD_FLAG_INDEX, TIMELY_HEAD_WEIGHT)
  ]

-- Compute deltas for a single flag index
private def getFlagIndexDeltas (state : BeaconState) (flagIndex : Nat)
    (weight : UInt64) : Array Int × Array Int := Id.run do
  let n := state.validators.size
  let mut rewards := Array.replicate n (0 : Int)
  let mut penalties := Array.replicate n (0 : Int)
  let previousEpoch := getPreviousEpoch state
  let activeIndices := getActiveValidatorIndices state previousEpoch
  let totalActiveBalance := getTotalActiveBalance state
  let inLeak := isInInactivityLeak state
  -- Compute participating balance for this flag
  let mut participatingBalance : Gwei := 0
  for idx in activeIndices do
    let i := idx.toNat
    if i < state.previousEpochParticipation.size then
      if hasFlag state.previousEpochParticipation[i]! flagIndex then
        if i < state.validators.size then
          participatingBalance := participatingBalance + state.validators[i]!.effectiveBalance
  if participatingBalance < EFFECTIVE_BALANCE_INCREMENT then
    participatingBalance := EFFECTIVE_BALANCE_INCREMENT
  -- Compute deltas
  for idx in activeIndices do
    let i := idx.toNat
    let baseReward := getBaseReward state idx
    let participated :=
      if i < state.previousEpochParticipation.size then
        hasFlag state.previousEpochParticipation[i]! flagIndex
      else false
    if participated then
      -- Reward
      if !inLeak then
        if i < rewards.size then
          let rewardNumerator := baseReward * weight * participatingBalance
          let rewardDenominator := totalActiveBalance * WEIGHT_DENOMINATOR
          let reward := if rewardDenominator > 0 then rewardNumerator / rewardDenominator else 0
          rewards := rewards.set! i (rewards[i]! + reward.toNat)
      -- During leak: no rewards (but no penalty either for participating)
    else
      -- Penalty
      if i < penalties.size then
        let penalty := baseReward * weight / WEIGHT_DENOMINATOR
        penalties := penalties.set! i (penalties[i]! + penalty.toNat)
  return (rewards, penalties)

-- Compute inactivity penalty deltas
private def getInactivityPenaltyDeltas (state : BeaconState) : Array Int := Id.run do
  let n := state.validators.size
  let mut penalties := Array.replicate n (0 : Int)
  let previousEpoch := getPreviousEpoch state
  let activeIndices := getActiveValidatorIndices state previousEpoch
  for idx in activeIndices do
    let i := idx.toNat
    let targetParticipated :=
      if i < state.previousEpochParticipation.size then
        hasFlag state.previousEpochParticipation[i]! TIMELY_TARGET_FLAG_INDEX
      else false
    if !targetParticipated then
      if i < penalties.size then
        if i < state.validators.size then
          if i < state.inactivityScores.size then
            let effectiveBal := state.validators[i]!.effectiveBalance
            let score := state.inactivityScores[i]!
            let penalty := effectiveBal * score / INACTIVITY_PENALTY_QUOTIENT_BELLATRIX
            penalties := penalties.set! i (penalties[i]! + penalty.toNat)
  return penalties

def processRewardsAndPenalties (state : BeaconState) : BeaconState := Id.run do
  let currentEpoch := getCurrentEpoch state
  -- Skip genesis epoch
  if currentEpoch == 0 then return state
  -- Accumulate flag deltas
  let mut totalRewards := Array.replicate state.validators.size (0 : Int)
  let mut totalPenalties := Array.replicate state.validators.size (0 : Int)
  for (flagIdx, weight) in flagWeights do
    let (rewards, penalties) := getFlagIndexDeltas state flagIdx weight
    for i in [:state.validators.size] do
      if i < totalRewards.size then
        totalRewards := totalRewards.set! i (totalRewards[i]! + (if i < rewards.size then rewards[i]! else 0))
      if i < totalPenalties.size then
        totalPenalties := totalPenalties.set! i (totalPenalties[i]! + (if i < penalties.size then penalties[i]! else 0))
  -- Add inactivity penalties
  let inactivityPenalties := getInactivityPenaltyDeltas state
  for i in [:state.validators.size] do
    if i < totalPenalties.size then
      totalPenalties := totalPenalties.set! i
        (totalPenalties[i]! + (if i < inactivityPenalties.size then inactivityPenalties[i]! else 0))
  -- Apply deltas to balances
  let mut state := state
  for i in [:state.validators.size] do
    let reward := if i < totalRewards.size then totalRewards[i]! else 0
    let penalty := if i < totalPenalties.size then totalPenalties[i]! else 0
    if reward > 0 then
      state := increaseBalance state i.toUInt64 reward.toNat.toUInt64
    if penalty > 0 then
      state := decreaseBalance state i.toUInt64 penalty.toNat.toUInt64
  return state

end Eth2
