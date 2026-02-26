/-
  Ethereum Consensus Layer — Helper Functions

  Pure helper functions used across epoch/block processing.
  Reference: https://eth2book.info/latest/part3/helper/
-/
import Guest.Eth2.Types
import Guest.Eth2.Constants
import Guest.Eth2.Crypto
import Guest.Eth2.Containers

namespace Eth2

-- ═══════════════════════════════════════════════
-- Epoch / Slot conversions
-- ═══════════════════════════════════════════════

def computeEpochAtSlot (slot : Slot) : Epoch :=
  slot / SLOTS_PER_EPOCH

def computeStartSlotAtEpoch (epoch : Epoch) : Slot :=
  epoch * SLOTS_PER_EPOCH

def getCurrentEpoch (state : BeaconState) : Epoch :=
  computeEpochAtSlot state.slot

def getPreviousEpoch (state : BeaconState) : Epoch :=
  let currentEpoch := getCurrentEpoch state
  if currentEpoch > 0 then currentEpoch - 1 else currentEpoch

-- ═══════════════════════════════════════════════
-- Validator predicates
-- ═══════════════════════════════════════════════

def isActiveValidator (validator : Validator) (epoch : Epoch) : Bool :=
  validator.activationEpoch <= epoch && epoch < validator.exitEpoch

def isEligibleForActivationQueue (validator : Validator) : Bool :=
  validator.activationEligibilityEpoch == FAR_FUTURE_EPOCH &&
  validator.effectiveBalance == MAX_EFFECTIVE_BALANCE

def isEligibleForActivation (state : BeaconState) (validator : Validator) : Bool :=
  validator.activationEligibilityEpoch <= state.finalizedCheckpoint.epoch &&
  validator.activationEpoch == FAR_FUTURE_EPOCH

def isSlashableValidator (validator : Validator) (epoch : Epoch) : Bool :=
  !validator.slashed &&
  validator.activationEpoch <= epoch &&
  epoch < validator.withdrawableEpoch

-- ═══════════════════════════════════════════════
-- Active validator queries
-- ═══════════════════════════════════════════════

def getActiveValidatorIndices (state : BeaconState) (epoch : Epoch) : Array ValidatorIndex := Id.run do
  let mut result : Array ValidatorIndex := #[]
  for i in [:state.validators.size] do
    if isActiveValidator state.validators[i]! epoch then
      result := result.push i.toUInt64
  return result

def getActiveValidatorCount (state : BeaconState) (epoch : Epoch) : UInt64 :=
  (getActiveValidatorIndices state epoch).size.toUInt64

-- ═══════════════════════════════════════════════
-- Balance helpers
-- ═══════════════════════════════════════════════

def getTotalBalance (state : BeaconState) (indices : Array ValidatorIndex) : Gwei := Id.run do
  let mut total : Gwei := 0
  for idx in indices do
    let i := idx.toNat
    if i < state.validators.size then
      total := total + state.validators[i]!.effectiveBalance
  -- Spec: return max(EFFECTIVE_BALANCE_INCREMENT, total_balance)
  return if total < EFFECTIVE_BALANCE_INCREMENT then EFFECTIVE_BALANCE_INCREMENT else total

def getTotalActiveBalance (state : BeaconState) : Gwei :=
  getTotalBalance state (getActiveValidatorIndices state (getCurrentEpoch state))

def increaseBalance (state : BeaconState) (index : ValidatorIndex) (delta : Gwei) : BeaconState :=
  let i := index.toNat
  if i < state.balances.size then
    { state with balances := state.balances.set! i (state.balances[i]! + delta) }
  else state

def decreaseBalance (state : BeaconState) (index : ValidatorIndex) (delta : Gwei) : BeaconState :=
  let i := index.toNat
  if i < state.balances.size then
    let current := state.balances[i]!
    let newBal := if current >= delta then current - delta else 0
    { state with balances := state.balances.set! i newBal }
  else state

-- ═══════════════════════════════════════════════
-- Reward computation (Altair)
-- ═══════════════════════════════════════════════

-- Integer square root (spec: integer_squareroot)
partial def integerSquareroot (n : UInt64) : UInt64 := Id.run do
  if n == 0 then return 0
  let mut x := n
  let mut y := (x + 1) / 2
  while y < x do
    x := y
    y := (x + n / x) / 2
  return x

def getBaseRewardPerIncrement (state : BeaconState) : Gwei :=
  let totalBalance := getTotalActiveBalance state
  let sqrtBalance := integerSquareroot totalBalance
  if sqrtBalance == 0 then 0
  else EFFECTIVE_BALANCE_INCREMENT * BASE_REWARD_FACTOR / sqrtBalance

def getBaseReward (state : BeaconState) (index : ValidatorIndex) : Gwei :=
  let i := index.toNat
  if i < state.validators.size then
    let increments := state.validators[i]!.effectiveBalance / EFFECTIVE_BALANCE_INCREMENT
    increments * getBaseRewardPerIncrement state
  else 0

-- ═══════════════════════════════════════════════
-- Validator lifecycle
-- ═══════════════════════════════════════════════

-- Compute validator churn limit
def getValidatorChurnLimit (state : BeaconState) : UInt64 :=
  let activeCount := getActiveValidatorCount state (getCurrentEpoch state)
  let churn := activeCount / CHURN_LIMIT_QUOTIENT
  if churn < MIN_PER_EPOCH_CHURN_LIMIT then MIN_PER_EPOCH_CHURN_LIMIT else churn

-- Initiate validator exit (assigns exit_epoch and withdrawable_epoch)
def initiateValidatorExit (state : BeaconState) (index : ValidatorIndex) : BeaconState := Id.run do
  let i := index.toNat
  if i >= state.validators.size then return state
  let validator := state.validators[i]!
  -- Already initiated
  if validator.exitEpoch != FAR_FUTURE_EPOCH then return state
  -- Find the maximum exit epoch among all validators
  let currentEpoch := getCurrentEpoch state
  let mut exitEpoch := computeEpochAtSlot (computeStartSlotAtEpoch currentEpoch)
  for v in state.validators do
    if v.exitEpoch != FAR_FUTURE_EPOCH && v.exitEpoch > exitEpoch then
      exitEpoch := v.exitEpoch
  -- Bump if at churn limit
  let churnLimit := getValidatorChurnLimit state
  let mut exitCount : UInt64 := 0
  for v in state.validators do
    if v.exitEpoch == exitEpoch then
      exitCount := exitCount + 1
  if exitCount >= churnLimit then
    exitEpoch := exitEpoch + 1
  let withdrawableEpoch := exitEpoch + MIN_VALIDATOR_WITHDRAWABILITY_DELAY
  let newValidator := { validator with
    exitEpoch := exitEpoch
    withdrawableEpoch := withdrawableEpoch
  }
  return { state with validators := state.validators.set! i newValidator }

-- Slash a validator (mark as slashed, initiate exit, apply penalty)
def slashValidator (state : BeaconState) (slashedIndex : ValidatorIndex)
    (whistleblowerIndex : Option ValidatorIndex) : BeaconState :=
  let i := slashedIndex.toNat
  if i >= state.validators.size then state
  else
    let epoch := getCurrentEpoch state
    -- Initiate exit first
    let state := initiateValidatorExit state slashedIndex
    -- Mark as slashed, set withdrawable epoch
    let validator := state.validators[i]!
    let newValidator := { validator with
      slashed := true
      withdrawableEpoch :=
        let we := epoch + EPOCHS_PER_SLASHINGS_VECTOR
        if we > validator.withdrawableEpoch then we else validator.withdrawableEpoch
    }
    let state := { state with validators := state.validators.set! i newValidator }
    -- Record slashing in slashings vector
    let slashingsIdx := (epoch % EPOCHS_PER_SLASHINGS_VECTOR).toNat
    let state :=
      if slashingsIdx < state.slashings.size then
        let newSlashing := state.slashings[slashingsIdx]! + newValidator.effectiveBalance
        { state with slashings := state.slashings.set! slashingsIdx newSlashing }
      else state
    -- Apply minimum penalty
    let penalty := newValidator.effectiveBalance / MIN_SLASHING_PENALTY_QUOTIENT_BELLATRIX
    let state := decreaseBalance state slashedIndex penalty
    -- Proposer reward
    let proposerIndex := whistleblowerIndex.getD 0  -- simplified: use provided or 0
    let whistleblowerReward := newValidator.effectiveBalance / WHISTLEBLOWER_REWARD_QUOTIENT
    let proposerReward := whistleblowerReward * PROPOSER_WEIGHT / WEIGHT_DENOMINATOR
    let state := increaseBalance state proposerIndex proposerReward
    state

-- ═══════════════════════════════════════════════
-- Participation flag helpers (Altair)
-- ═══════════════════════════════════════════════

-- Check if a participation flag is set
def hasFlag (flags : ParticipationFlags) (flagIndex : Nat) : Bool :=
  (flags.toNat >>> flagIndex) &&& 1 == 1

-- Add a participation flag
def addFlag (flags : ParticipationFlags) (flagIndex : Nat) : ParticipationFlags :=
  flags ||| (1 <<< flagIndex).toUInt8

-- ═══════════════════════════════════════════════
-- Misc helpers
-- ═══════════════════════════════════════════════

-- Get the randao mix for a given epoch
def getRandaoMix (state : BeaconState) (epoch : Epoch) : Bytes32 :=
  let idx := (epoch % EPOCHS_PER_HISTORICAL_VECTOR).toNat
  if idx < state.randaoMixes.size then
    state.randaoMixes[idx]!
  else ByteArray.mk (Array.replicate 32 0)

-- Get block root at slot (from circular buffer)
def getBlockRootAtSlot (state : BeaconState) (slot : Slot) : Root :=
  let idx := (slot % SLOTS_PER_HISTORICAL_ROOT).toNat
  if idx < state.blockRoots.size then
    state.blockRoots[idx]!
  else ByteArray.mk (Array.replicate 32 0)

-- Get block root at epoch (returns root at start slot of epoch)
def getBlockRoot (state : BeaconState) (epoch : Epoch) : Root :=
  getBlockRootAtSlot state (computeStartSlotAtEpoch epoch)

-- Get the beacon proposer index (stub: returns slot % validator_count)
-- In production this would use RANDAO-based shuffling
def getBeaconProposerIndex (state : BeaconState) : ValidatorIndex :=
  let activeIndices := getActiveValidatorIndices state (getCurrentEpoch state)
  if activeIndices.size == 0 then 0
  else
    let idx := (state.slot % activeIndices.size.toUInt64).toNat
    if idx < activeIndices.size then activeIndices[idx]!
    else 0

end Eth2
