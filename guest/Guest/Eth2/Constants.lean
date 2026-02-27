/-
  Ethereum Consensus Layer — Preset & Configuration Constants (Mainnet)

  Reference: https://eth2book.info/latest/part3/config/preset/
             https://eth2book.info/latest/part3/config/configuration/
-/
import Guest.Eth2.Types

namespace Eth2

-- ═══════════════════════════════════════════════
-- Misc
-- ═══════════════════════════════════════════════
def DEPOSIT_CONTRACT_TREE_DEPTH : Nat := 32
def JUSTIFICATION_BITS_LENGTH : Nat := 4

-- ═══════════════════════════════════════════════
-- Core parameters
-- ═══════════════════════════════════════════════
def SLOTS_PER_EPOCH : UInt64 := 32
def MAX_COMMITTEES_PER_SLOT : Nat := 64
def TARGET_COMMITTEE_SIZE : Nat := 128
def MAX_VALIDATORS_PER_COMMITTEE : Nat := 2048
def SHUFFLE_ROUND_COUNT : Nat := 90

-- ═══════════════════════════════════════════════
-- Balance & deposits (Gwei)
-- ═══════════════════════════════════════════════
def MIN_DEPOSIT_AMOUNT : Gwei := 1000000000          -- 1 ETH
def MAX_EFFECTIVE_BALANCE : Gwei := 32000000000       -- 32 ETH
def EFFECTIVE_BALANCE_INCREMENT : Gwei := 1000000000  -- 1 ETH
def EJECTION_BALANCE : Gwei := 16000000000            -- 16 ETH

-- ═══════════════════════════════════════════════
-- Time parameters (in slots/epochs)
-- ═══════════════════════════════════════════════
def MIN_ATTESTATION_INCLUSION_DELAY : UInt64 := 1
def MIN_SEED_LOOKAHEAD : UInt64 := 1
def MAX_SEED_LOOKAHEAD : UInt64 := 4
def MIN_EPOCHS_TO_INACTIVITY_PENALTY : UInt64 := 4
def EPOCHS_PER_ETH1_VOTING_PERIOD : UInt64 := 64
def SLOTS_PER_HISTORICAL_ROOT : UInt64 := 8192
def SECONDS_PER_SLOT : UInt64 := 12
def SECONDS_PER_ETH1_BLOCK : UInt64 := 14
def ETH1_FOLLOW_DISTANCE : UInt64 := 2048
def MIN_VALIDATOR_WITHDRAWABILITY_DELAY : UInt64 := 256
def SHARD_COMMITTEE_PERIOD : UInt64 := 256

-- ═══════════════════════════════════════════════
-- State list bounds
-- ═══════════════════════════════════════════════
def EPOCHS_PER_HISTORICAL_VECTOR : UInt64 := 65536
def EPOCHS_PER_SLASHINGS_VECTOR : UInt64 := 8192
def HISTORICAL_ROOTS_LIMIT : Nat := 16777216
def VALIDATOR_REGISTRY_LIMIT : Nat := 1099511627776

-- ═══════════════════════════════════════════════
-- Block operation limits
-- ═══════════════════════════════════════════════
def MAX_PROPOSER_SLASHINGS : Nat := 16
def MAX_ATTESTER_SLASHINGS : Nat := 2
def MAX_ATTESTATIONS : Nat := 128
def MAX_DEPOSITS : Nat := 16
def MAX_VOLUNTARY_EXITS : Nat := 16
def MAX_BLS_TO_EXECUTION_CHANGES : Nat := 16

-- ═══════════════════════════════════════════════
-- Rewards & penalties
-- ═══════════════════════════════════════════════
def BASE_REWARD_FACTOR : UInt64 := 64
def WHISTLEBLOWER_REWARD_QUOTIENT : UInt64 := 512
def PROPOSER_REWARD_QUOTIENT : UInt64 := 8
def INACTIVITY_PENALTY_QUOTIENT_BELLATRIX : UInt64 := 16777216
def MIN_SLASHING_PENALTY_QUOTIENT_BELLATRIX : UInt64 := 32
def PROPORTIONAL_SLASHING_MULTIPLIER_BELLATRIX : UInt64 := 3

-- Participation flag indices (Altair)
def TIMELY_SOURCE_FLAG_INDEX : Nat := 0
def TIMELY_TARGET_FLAG_INDEX : Nat := 1
def TIMELY_HEAD_FLAG_INDEX : Nat := 2

-- Participation flag weights (Altair)
def TIMELY_SOURCE_WEIGHT : UInt64 := 14
def TIMELY_TARGET_WEIGHT : UInt64 := 26
def TIMELY_HEAD_WEIGHT : UInt64 := 14
def SYNC_REWARD_WEIGHT : UInt64 := 2
def PROPOSER_WEIGHT : UInt64 := 8
def WEIGHT_DENOMINATOR : UInt64 := 64

-- ═══════════════════════════════════════════════
-- Sync committee (Altair)
-- ═══════════════════════════════════════════════
def SYNC_COMMITTEE_SIZE : Nat := 512
def EPOCHS_PER_SYNC_COMMITTEE_PERIOD : UInt64 := 256

-- ═══════════════════════════════════════════════
-- Execution & withdrawals (Bellatrix / Capella)
-- ═══════════════════════════════════════════════
def MAX_BYTES_PER_TRANSACTION : Nat := 1073741824
def MAX_TRANSACTIONS_PER_PAYLOAD : Nat := 1048576
def BYTES_PER_LOGS_BLOOM : Nat := 256
def MAX_EXTRA_DATA_BYTES : Nat := 32
def MAX_WITHDRAWALS_PER_PAYLOAD : Nat := 16
def MAX_VALIDATORS_PER_WITHDRAWALS_SWEEP : UInt64 := 16384

-- ═══════════════════════════════════════════════
-- Validator cycle
-- ═══════════════════════════════════════════════
def MIN_PER_EPOCH_CHURN_LIMIT : UInt64 := 4
def CHURN_LIMIT_QUOTIENT : UInt64 := 65536
def MIN_GENESIS_ACTIVE_VALIDATOR_COUNT : Nat := 16384
def MIN_GENESIS_TIME : UInt64 := 1606824000
def GENESIS_DELAY : UInt64 := 604800

-- Inactivity scoring (Altair)
def INACTIVITY_SCORE_BIAS : UInt64 := 4
def INACTIVITY_SCORE_RECOVERY_RATE : UInt64 := 16

end Eth2
