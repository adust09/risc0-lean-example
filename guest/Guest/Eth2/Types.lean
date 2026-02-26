/-
  Ethereum Consensus Layer — Custom Types

  Maps SSZ types to Lean 4 equivalents.
  Reference: https://eth2book.info/latest/part3/config/types/
-/

namespace Eth2

-- Numeric aliases (SSZ uint64 → Lean UInt64)
abbrev Slot := UInt64
abbrev Epoch := UInt64
abbrev CommitteeIndex := UInt64
abbrev ValidatorIndex := UInt64
abbrev Gwei := UInt64
abbrev WithdrawalIndex := UInt64

-- Byte-array aliases
-- In production SSZ these have fixed sizes; here we use ByteArray uniformly.
abbrev Root := ByteArray            -- 32 bytes (Bytes32)
abbrev Bytes32 := ByteArray         -- 32 bytes
abbrev Hash32 := ByteArray          -- 32 bytes
abbrev Version := ByteArray         -- 4 bytes (Bytes4)
abbrev DomainType := ByteArray      -- 4 bytes (Bytes4)
abbrev ForkDigest := ByteArray      -- 4 bytes (Bytes4)
abbrev Domain := ByteArray          -- 32 bytes (Bytes32)
abbrev BLSPubkey := ByteArray       -- 48 bytes (Bytes48)
abbrev BLSSignature := ByteArray    -- 96 bytes (Bytes96)
abbrev ExecutionAddress := ByteArray -- 20 bytes (Bytes20)

-- Participation flags (SSZ uint8)
abbrev ParticipationFlags := UInt8

-- Sentinel value used throughout the spec
def FAR_FUTURE_EPOCH : Epoch := 0xFFFFFFFFFFFFFFFF

end Eth2
