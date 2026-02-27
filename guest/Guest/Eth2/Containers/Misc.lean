/-
  Ethereum Consensus Layer — Miscellaneous Containers

  Reference: https://eth2book.info/latest/part3/containers/dependencies/
-/
import Guest.Eth2.Types

namespace Eth2

-- Eth1 chain data snapshot for deposit tracking
structure Eth1Data where
  depositRoot  : Root
  depositCount : UInt64
  blockHash    : Hash32
  deriving Repr, Inhabited

-- Block header (without body, used for double-proposal checks)
structure BeaconBlockHeader where
  slot          : Slot
  proposerIndex : ValidatorIndex
  parentRoot    : Root
  stateRoot     : Root
  bodyRoot      : Root
  deriving Repr, Inhabited

-- Signed wrapper for BeaconBlockHeader
structure SignedBeaconBlockHeader where
  message   : BeaconBlockHeader
  signature : BLSSignature
  deriving Repr, Inhabited

-- Signing data for domain separation
structure SigningData where
  objectRoot : Root
  domain     : Domain
  deriving Repr, Inhabited

-- Sync committee: 512 validators for light client support (Altair)
structure SyncCommittee where
  pubkeys        : Array BLSPubkey    -- SYNC_COMMITTEE_SIZE elements
  aggregatePubkey : BLSPubkey
  deriving Repr, Inhabited

-- Aggregated sync committee signature (Altair)
structure SyncAggregate where
  syncCommitteeBits      : ByteArray  -- Bitvector[SYNC_COMMITTEE_SIZE]
  syncCommitteeSignature : BLSSignature
  deriving Repr, Inhabited

-- Withdrawal from consensus to execution layer (Capella)
structure Withdrawal where
  index          : WithdrawalIndex
  validatorIndex : ValidatorIndex
  address        : ExecutionAddress
  amount         : Gwei
  deriving Repr, Inhabited

-- Historical summary for deep history access (Capella)
structure HistoricalSummary where
  blockSummaryRoot : Root
  stateSummaryRoot : Root
  deriving Repr, Inhabited

end Eth2
