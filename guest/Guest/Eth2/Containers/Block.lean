/-
  Ethereum Consensus Layer — Block Containers

  Reference: https://eth2book.info/latest/part3/containers/blocks/
             https://eth2book.info/latest/part3/containers/execution/
-/
import Guest.Eth2.Types
import Guest.Eth2.Containers.Validator
import Guest.Eth2.Containers.Misc
import Guest.Eth2.Containers.Operations

namespace Eth2

-- Execution layer payload (Bellatrix / Capella)
structure ExecutionPayload where
  parentHash    : Hash32
  feeRecipient  : ExecutionAddress
  stateRoot     : Bytes32
  receiptsRoot  : Bytes32
  logsBloom     : ByteArray           -- BYTES_PER_LOGS_BLOOM bytes
  prevRandao    : Bytes32
  blockNumber   : UInt64
  gasLimit      : UInt64
  gasUsed       : UInt64
  timestamp     : UInt64
  extraData     : ByteArray           -- max MAX_EXTRA_DATA_BYTES
  baseFeePerGas : UInt64              -- simplified from uint256
  blockHash     : Hash32
  transactions  : Array ByteArray     -- List[Transaction, ...]
  withdrawals   : Array Withdrawal    -- List[Withdrawal, ...] (Capella)
  deriving Repr, Inhabited

-- Execution layer payload header (stored in state)
structure ExecutionPayloadHeader where
  parentHash       : Hash32
  feeRecipient     : ExecutionAddress
  stateRoot        : Bytes32
  receiptsRoot     : Bytes32
  logsBloom        : ByteArray
  prevRandao       : Bytes32
  blockNumber      : UInt64
  gasLimit         : UInt64
  gasUsed          : UInt64
  timestamp        : UInt64
  extraData        : ByteArray
  baseFeePerGas    : UInt64           -- simplified from uint256
  blockHash        : Hash32
  transactionsRoot : Root
  withdrawalsRoot  : Root
  deriving Repr, Inhabited

-- Beacon block body containing all operations
structure BeaconBlockBody where
  randaoReveal          : BLSSignature
  eth1Data              : Eth1Data
  graffiti              : Bytes32
  proposerSlashings     : Array ProposerSlashing
  attesterSlashings     : Array AttesterSlashing
  attestations          : Array Attestation
  deposits              : Array Deposit
  voluntaryExits        : Array SignedVoluntaryExit
  syncAggregate         : SyncAggregate
  executionPayload      : ExecutionPayload
  blsToExecutionChanges : Array SignedBLSToExecutionChange
  deriving Repr, Inhabited

-- Beacon block (unsigned)
structure BeaconBlock where
  slot          : Slot
  proposerIndex : ValidatorIndex
  parentRoot    : Root
  stateRoot     : Root
  body          : BeaconBlockBody
  deriving Repr, Inhabited

-- Signed beacon block (block + BLS signature)
structure SignedBeaconBlock where
  message   : BeaconBlock
  signature : BLSSignature
  deriving Repr, Inhabited

end Eth2
