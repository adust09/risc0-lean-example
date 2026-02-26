/-
  Ethereum Consensus Layer — Block Operation Containers

  Reference: https://eth2book.info/latest/part3/containers/operations/
-/
import Guest.Eth2.Types
import Guest.Eth2.Containers.Validator
import Guest.Eth2.Containers.Misc

namespace Eth2

-- Evidence of a proposer signing two different blocks at the same slot
structure ProposerSlashing where
  signedHeader1 : SignedBeaconBlockHeader
  signedHeader2 : SignedBeaconBlockHeader
  deriving Repr, Inhabited

-- Evidence of a validator making conflicting attestations
structure AttesterSlashing where
  attestation1 : IndexedAttestation
  attestation2 : IndexedAttestation
  deriving Repr, Inhabited

-- Aggregated attestation from a committee
structure Attestation where
  aggregationBits : ByteArray  -- Bitlist[MAX_VALIDATORS_PER_COMMITTEE]
  data            : AttestationData
  signature       : BLSSignature
  deriving Repr, Inhabited

-- Deposit data (what gets deposited on the Eth1 chain)
structure DepositData where
  pubkey                : BLSPubkey
  withdrawalCredentials : Bytes32
  amount                : Gwei
  signature             : BLSSignature
  deriving Repr, Inhabited

-- Deposit proof from the Eth1 deposit contract
structure Deposit where
  proof : Array Bytes32  -- DEPOSIT_CONTRACT_TREE_DEPTH + 1 elements
  data  : DepositData
  deriving Repr, Inhabited

-- Voluntary exit request
structure VoluntaryExit where
  epoch          : Epoch
  validatorIndex : ValidatorIndex
  deriving Repr, Inhabited

-- Signed voluntary exit
structure SignedVoluntaryExit where
  message   : VoluntaryExit
  signature : BLSSignature
  deriving Repr, Inhabited

-- Request to change BLS withdrawal credentials to execution address (Capella)
structure BLSToExecutionChange where
  validatorIndex     : ValidatorIndex
  fromBlsPubkey      : BLSPubkey
  toExecutionAddress : ExecutionAddress
  deriving Repr, Inhabited

-- Signed BLS-to-execution change
structure SignedBLSToExecutionChange where
  message   : BLSToExecutionChange
  signature : BLSSignature
  deriving Repr, Inhabited

end Eth2
