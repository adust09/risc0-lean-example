/-
  Ethereum Consensus Layer — BeaconState Container

  Full Capella-era BeaconState with all 31 fields.
  Reference: https://eth2book.info/latest/part3/containers/state/
-/
import Guest.Eth2.Types
import Guest.Eth2.Containers.Validator
import Guest.Eth2.Containers.Misc
import Guest.Eth2.Containers.Block

namespace Eth2

structure BeaconState where
  -- Versioning
  genesisTime            : UInt64
  genesisValidatorsRoot  : Root
  slot                   : Slot
  fork                   : Fork

  -- History
  latestBlockHeader      : BeaconBlockHeader
  blockRoots             : Array Root     -- Vector[Root, SLOTS_PER_HISTORICAL_ROOT]
  stateRoots             : Array Root     -- Vector[Root, SLOTS_PER_HISTORICAL_ROOT]
  historicalRoots        : Array Root     -- List[Root, HISTORICAL_ROOTS_LIMIT] (frozen)

  -- Eth1
  eth1Data               : Eth1Data
  eth1DataVotes          : Array Eth1Data -- List[Eth1Data, EPOCHS_PER_ETH1_VOTING_PERIOD * SLOTS_PER_EPOCH]
  eth1DepositIndex       : UInt64

  -- Registry
  validators             : Array Validator
  balances               : Array Gwei

  -- Randomness
  randaoMixes            : Array Bytes32  -- Vector[Bytes32, EPOCHS_PER_HISTORICAL_VECTOR]

  -- Slashings
  slashings              : Array Gwei     -- Vector[Gwei, EPOCHS_PER_SLASHINGS_VECTOR]

  -- Participation (Altair)
  previousEpochParticipation : Array ParticipationFlags
  currentEpochParticipation  : Array ParticipationFlags

  -- Finality
  justificationBits          : ByteArray  -- Bitvector[JUSTIFICATION_BITS_LENGTH]
  previousJustifiedCheckpoint : Checkpoint
  currentJustifiedCheckpoint  : Checkpoint
  finalizedCheckpoint         : Checkpoint

  -- Inactivity (Altair)
  inactivityScores : Array UInt64

  -- Sync committees (Altair)
  currentSyncCommittee  : SyncCommittee
  nextSyncCommittee     : SyncCommittee

  -- Execution (Bellatrix)
  latestExecutionPayloadHeader : ExecutionPayloadHeader

  -- Withdrawals (Capella)
  nextWithdrawalIndex          : WithdrawalIndex
  nextWithdrawalValidatorIndex : ValidatorIndex

  -- Deep history (Capella)
  historicalSummaries : Array HistoricalSummary
  deriving Repr, Inhabited

end Eth2
