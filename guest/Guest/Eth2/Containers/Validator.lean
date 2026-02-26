/-
  Ethereum Consensus Layer — Validator-related Containers

  Reference: https://eth2book.info/latest/part3/containers/dependencies/
-/
import Guest.Eth2.Types

namespace Eth2

-- Fork context for domain separation across chain upgrades
structure Fork where
  previousVersion : Version
  currentVersion  : Version
  epoch           : Epoch
  deriving Repr, Inhabited

-- Fork data used in domain computation
structure ForkData where
  currentVersion        : Version
  genesisValidatorsRoot : Root
  deriving Repr, Inhabited

-- Checkpoint: epoch + block root, used for finality
structure Checkpoint where
  epoch : Epoch
  root  : Root
  deriving Repr, Inhabited

-- Validator record in the registry
structure Validator where
  pubkey                     : BLSPubkey
  withdrawalCredentials      : Bytes32
  effectiveBalance           : Gwei
  slashed                    : Bool
  activationEligibilityEpoch : Epoch
  activationEpoch            : Epoch
  exitEpoch                  : Epoch
  withdrawableEpoch          : Epoch
  deriving Repr, Inhabited

-- Attestation vote data
structure AttestationData where
  slot            : Slot
  index           : CommitteeIndex
  beaconBlockRoot : Root
  source          : Checkpoint
  target          : Checkpoint
  deriving Repr, Inhabited

-- Attestation with committee indices resolved (used for slashing checks)
structure IndexedAttestation where
  attestingIndices : Array ValidatorIndex
  data             : AttestationData
  signature        : BLSSignature
  deriving Repr, Inhabited

end Eth2
