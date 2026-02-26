/-
  Block Processing — Sync Aggregate (Altair)

  Distribute rewards/penalties based on sync committee participation.
  Reference: https://eth2book.info/latest/part3/transition/block/#sync-aggregate
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Transition.Block.Header

namespace Eth2

def processSyncAggregate (state : BeaconState) (aggregate : SyncAggregate) : STFResult BeaconState :=
  -- Stub: skip BLS aggregate signature verification
  let totalActiveBalance := getTotalActiveBalance state
  let totalParticipants := SYNC_COMMITTEE_SIZE
  -- Count participating committee members
  let mut numParticipants : UInt64 := 0
  for byteIdx in [:aggregate.syncCommitteeBits.size] do
    if h : byteIdx < aggregate.syncCommitteeBits.size then
      let byte := aggregate.syncCommitteeBits.get ⟨byteIdx, h⟩
      for bitPos in [:8] do
        if (byte.toNat >>> bitPos) &&& 1 == 1 then
          numParticipants := numParticipants + 1
  -- Compute reward per participant
  let totalIncrement := EFFECTIVE_BALANCE_INCREMENT * totalParticipants.toUInt64
  let baseRewardPerIncrement := getBaseRewardPerIncrement state
  let totalBaseReward := baseRewardPerIncrement * totalIncrement / EFFECTIVE_BALANCE_INCREMENT
  let maxParticipantReward := totalBaseReward * SYNC_REWARD_WEIGHT / WEIGHT_DENOMINATOR / totalParticipants.toUInt64
  -- Apply rewards/penalties to sync committee members
  let mut state := state
  let committeePubkeys := state.currentSyncCommittee.pubkeys
  for memberIdx in [:committeePubkeys.size] do
    if h : memberIdx < committeePubkeys.size then
      -- Find validator index by pubkey (simplified linear search)
      let pubkey := committeePubkeys[memberIdx]
      for valIdx in [:state.validators.size] do
        if h2 : valIdx < state.validators.size then
          if state.validators[valIdx].pubkey == pubkey then
            -- Check if this member participated
            let byteIdx := memberIdx / 8
            let bitPos := memberIdx % 8
            let participated :=
              if h3 : byteIdx < aggregate.syncCommitteeBits.size then
                (aggregate.syncCommitteeBits.get ⟨byteIdx, h3⟩).toNat >>> bitPos &&& 1 == 1
              else false
            if participated then
              state := increaseBalance state valIdx.toUInt64 maxParticipantReward
            else
              state := decreaseBalance state valIdx.toUInt64 maxParticipantReward
            -- Proposer reward for each participating member
            if participated then
              let proposerReward := maxParticipantReward * PROPOSER_WEIGHT / (WEIGHT_DENOMINATOR - PROPOSER_WEIGHT)
              let proposerIndex := getBeaconProposerIndex state
              state := increaseBalance state proposerIndex proposerReward
            break
  .ok state

end Eth2
