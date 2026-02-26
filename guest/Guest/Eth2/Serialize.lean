/-
  Ethereum Consensus Layer — Serialization

  Simple little-endian binary format for BeaconState and BeaconBlock.
  This is NOT SSZ — it's a minimal format for zkVM input/output.

  Format conventions:
  - UInt64: 8 bytes LE
  - Bool: 1 byte (0 or 1)
  - ByteArray: 4 bytes LE length prefix + raw bytes
  - Array T: 4 bytes LE count + each element serialized
  - Structures: fields serialized in declaration order
-/
import Guest.Eth2.Types
import Guest.Eth2.Containers

namespace Eth2

-- ═══════════════════════════════════════════════
-- Encoding primitives
-- ═══════════════════════════════════════════════

namespace Encode

def uint64 (buf : ByteArray) (v : UInt64) : ByteArray :=
  let n := v.toNat
  buf
  |>.push (n &&& 0xFF).toUInt8
  |>.push ((n >>> 8) &&& 0xFF).toUInt8
  |>.push ((n >>> 16) &&& 0xFF).toUInt8
  |>.push ((n >>> 24) &&& 0xFF).toUInt8
  |>.push ((n >>> 32) &&& 0xFF).toUInt8
  |>.push ((n >>> 40) &&& 0xFF).toUInt8
  |>.push ((n >>> 48) &&& 0xFF).toUInt8
  |>.push ((n >>> 56) &&& 0xFF).toUInt8

def uint32 (buf : ByteArray) (v : UInt32) : ByteArray :=
  let n := v.toNat
  buf
  |>.push (n &&& 0xFF).toUInt8
  |>.push ((n >>> 8) &&& 0xFF).toUInt8
  |>.push ((n >>> 16) &&& 0xFF).toUInt8
  |>.push ((n >>> 24) &&& 0xFF).toUInt8

def uint8 (buf : ByteArray) (v : UInt8) : ByteArray :=
  buf.push v

def bool (buf : ByteArray) (v : Bool) : ByteArray :=
  buf.push (if v then 1 else 0)

def bytes (buf : ByteArray) (v : ByteArray) : ByteArray :=
  let buf := uint32 buf v.size.toUInt32
  buf ++ v

def arrayOf (enc : ByteArray → α → ByteArray) (buf : ByteArray) (arr : Array α) : ByteArray :=
  let buf := uint32 buf arr.size.toUInt32
  arr.foldl enc buf

end Encode

-- ═══════════════════════════════════════════════
-- Decoding primitives
-- ═══════════════════════════════════════════════

-- Decoder state: (data, offset). Returns (result, new_offset) or none on failure.
abbrev DecodeM (α : Type) := ByteArray → Nat → Option (α × Nat)

namespace Decode

def uint64 : DecodeM UInt64 := fun data off =>
  if off + 8 > data.size then none
  else
    let v :=
      data.get! off       |>.toNat       |||
      data.get! (off + 1) |>.toNat <<< 8  |||
      data.get! (off + 2) |>.toNat <<< 16 |||
      data.get! (off + 3) |>.toNat <<< 24 |||
      data.get! (off + 4) |>.toNat <<< 32 |||
      data.get! (off + 5) |>.toNat <<< 40 |||
      data.get! (off + 6) |>.toNat <<< 48 |||
      data.get! (off + 7) |>.toNat <<< 56
    some (v.toUInt64, off + 8)

def uint32 : DecodeM UInt32 := fun data off =>
  if off + 4 > data.size then none
  else
    let v :=
      data.get! off       |>.toNat       |||
      data.get! (off + 1) |>.toNat <<< 8  |||
      data.get! (off + 2) |>.toNat <<< 16 |||
      data.get! (off + 3) |>.toNat <<< 24
    some (v.toUInt32, off + 4)

def uint8 : DecodeM UInt8 := fun data off =>
  if off < data.size then some (data.get! off, off + 1)
  else none

def bool : DecodeM Bool := fun data off =>
  if off < data.size then some (data.get! off != 0, off + 1)
  else none

def bytes : DecodeM ByteArray := fun data off => do
  let (len, off) ← uint32 data off
  let n := len.toNat
  if off + n > data.size then none
  else
    let slice := data.extract off (off + n)
    some (slice, off + n)

def arrayOf (dec : DecodeM α) : DecodeM (Array α) := fun data off => do
  let (count, off) ← uint32 data off
  let mut arr : Array α := #[]
  let mut pos := off
  for _ in [:count.toNat] do
    let (elem, newPos) ← dec data pos
    arr := arr.push elem
    pos := newPos
  some (arr, pos)

end Decode

-- ═══════════════════════════════════════════════
-- Container encoders
-- ═══════════════════════════════════════════════

namespace Encode

def fork (buf : ByteArray) (f : Fork) : ByteArray :=
  buf |> (bytes · f.previousVersion) |> (bytes · f.currentVersion) |> (uint64 · f.epoch)

def checkpoint (buf : ByteArray) (c : Checkpoint) : ByteArray :=
  buf |> (uint64 · c.epoch) |> (bytes · c.root)

def eth1Data (buf : ByteArray) (e : Eth1Data) : ByteArray :=
  buf |> (bytes · e.depositRoot) |> (uint64 · e.depositCount) |> (bytes · e.blockHash)

def beaconBlockHeader (buf : ByteArray) (h : BeaconBlockHeader) : ByteArray :=
  buf |> (uint64 · h.slot) |> (uint64 · h.proposerIndex)
      |> (bytes · h.parentRoot) |> (bytes · h.stateRoot) |> (bytes · h.bodyRoot)

def validator (buf : ByteArray) (v : Validator) : ByteArray :=
  buf |> (bytes · v.pubkey) |> (bytes · v.withdrawalCredentials)
      |> (uint64 · v.effectiveBalance) |> (bool · v.slashed)
      |> (uint64 · v.activationEligibilityEpoch) |> (uint64 · v.activationEpoch)
      |> (uint64 · v.exitEpoch) |> (uint64 · v.withdrawableEpoch)

def syncCommittee (buf : ByteArray) (sc : SyncCommittee) : ByteArray :=
  buf |> (arrayOf bytes · sc.pubkeys) |> (bytes · sc.aggregatePubkey)

def executionPayloadHeader (buf : ByteArray) (h : ExecutionPayloadHeader) : ByteArray :=
  buf |> (bytes · h.parentHash) |> (bytes · h.feeRecipient)
      |> (bytes · h.stateRoot) |> (bytes · h.receiptsRoot)
      |> (bytes · h.logsBloom) |> (bytes · h.prevRandao)
      |> (uint64 · h.blockNumber) |> (uint64 · h.gasLimit)
      |> (uint64 · h.gasUsed) |> (uint64 · h.timestamp)
      |> (bytes · h.extraData) |> (uint64 · h.baseFeePerGas)
      |> (bytes · h.blockHash) |> (bytes · h.transactionsRoot)
      |> (bytes · h.withdrawalsRoot)

def historicalSummary (buf : ByteArray) (hs : HistoricalSummary) : ByteArray :=
  buf |> (bytes · hs.blockSummaryRoot) |> (bytes · hs.stateSummaryRoot)

def beaconState (buf : ByteArray) (s : BeaconState) : ByteArray :=
  buf
  -- Versioning
  |> (uint64 · s.genesisTime) |> (bytes · s.genesisValidatorsRoot)
  |> (uint64 · s.slot) |> (fork · s.fork)
  -- History
  |> (beaconBlockHeader · s.latestBlockHeader)
  |> (arrayOf bytes · s.blockRoots) |> (arrayOf bytes · s.stateRoots)
  |> (arrayOf bytes · s.historicalRoots)
  -- Eth1
  |> (eth1Data · s.eth1Data)
  |> (arrayOf eth1Data · s.eth1DataVotes) |> (uint64 · s.eth1DepositIndex)
  -- Registry
  |> (arrayOf validator · s.validators) |> (arrayOf uint64 · s.balances)
  -- Randomness
  |> (arrayOf bytes · s.randaoMixes)
  -- Slashings
  |> (arrayOf uint64 · s.slashings)
  -- Participation
  |> (arrayOf uint8 · s.previousEpochParticipation)
  |> (arrayOf uint8 · s.currentEpochParticipation)
  -- Finality
  |> (bytes · s.justificationBits)
  |> (checkpoint · s.previousJustifiedCheckpoint)
  |> (checkpoint · s.currentJustifiedCheckpoint)
  |> (checkpoint · s.finalizedCheckpoint)
  -- Inactivity
  |> (arrayOf uint64 · s.inactivityScores)
  -- Sync committees
  |> (syncCommittee · s.currentSyncCommittee)
  |> (syncCommittee · s.nextSyncCommittee)
  -- Execution
  |> (executionPayloadHeader · s.latestExecutionPayloadHeader)
  -- Withdrawals
  |> (uint64 · s.nextWithdrawalIndex) |> (uint64 · s.nextWithdrawalValidatorIndex)
  -- Historical summaries
  |> (arrayOf historicalSummary · s.historicalSummaries)

end Encode

-- ═══════════════════════════════════════════════
-- Container decoders
-- ═══════════════════════════════════════════════

namespace Decode

def fork : DecodeM Fork := fun data off => do
  let (pv, off) ← bytes data off
  let (cv, off) ← bytes data off
  let (e, off) ← uint64 data off
  some ({ previousVersion := pv, currentVersion := cv, epoch := e }, off)

def checkpoint : DecodeM Checkpoint := fun data off => do
  let (e, off) ← uint64 data off
  let (r, off) ← bytes data off
  some ({ epoch := e, root := r }, off)

def eth1Data : DecodeM Eth1Data := fun data off => do
  let (dr, off) ← bytes data off
  let (dc, off) ← uint64 data off
  let (bh, off) ← bytes data off
  some ({ depositRoot := dr, depositCount := dc, blockHash := bh }, off)

def beaconBlockHeader : DecodeM BeaconBlockHeader := fun data off => do
  let (sl, off) ← uint64 data off
  let (pi, off) ← uint64 data off
  let (pr, off) ← bytes data off
  let (sr, off) ← bytes data off
  let (br, off) ← bytes data off
  some ({ slot := sl, proposerIndex := pi, parentRoot := pr, stateRoot := sr, bodyRoot := br }, off)

def validator : DecodeM Validator := fun data off => do
  let (pk, off) ← bytes data off
  let (wc, off) ← bytes data off
  let (eb, off) ← uint64 data off
  let (sl, off) ← Decode.bool data off
  let (aee, off) ← uint64 data off
  let (ae, off) ← uint64 data off
  let (ee, off) ← uint64 data off
  let (we, off) ← uint64 data off
  some ({
    pubkey := pk, withdrawalCredentials := wc, effectiveBalance := eb,
    slashed := sl, activationEligibilityEpoch := aee, activationEpoch := ae,
    exitEpoch := ee, withdrawableEpoch := we
  }, off)

def syncCommittee : DecodeM SyncCommittee := fun data off => do
  let (pks, off) ← arrayOf bytes data off
  let (apk, off) ← bytes data off
  some ({ pubkeys := pks, aggregatePubkey := apk }, off)

def executionPayloadHeader : DecodeM ExecutionPayloadHeader := fun data off => do
  let (ph, off) ← bytes data off
  let (fr, off) ← bytes data off
  let (sr, off) ← bytes data off
  let (rr, off) ← bytes data off
  let (lb, off) ← bytes data off
  let (pr, off) ← bytes data off
  let (bn, off) ← uint64 data off
  let (gl, off) ← uint64 data off
  let (gu, off) ← uint64 data off
  let (ts, off) ← uint64 data off
  let (ed, off) ← bytes data off
  let (bf, off) ← uint64 data off
  let (bh, off) ← bytes data off
  let (tr, off) ← bytes data off
  let (wr, off) ← bytes data off
  some ({
    parentHash := ph, feeRecipient := fr, stateRoot := sr, receiptsRoot := rr,
    logsBloom := lb, prevRandao := pr, blockNumber := bn, gasLimit := gl,
    gasUsed := gu, timestamp := ts, extraData := ed, baseFeePerGas := bf,
    blockHash := bh, transactionsRoot := tr, withdrawalsRoot := wr
  }, off)

def historicalSummary : DecodeM HistoricalSummary := fun data off => do
  let (bsr, off) ← bytes data off
  let (ssr, off) ← bytes data off
  some ({ blockSummaryRoot := bsr, stateSummaryRoot := ssr }, off)

def beaconState : DecodeM BeaconState := fun data off => do
  -- Versioning
  let (gt, off) ← uint64 data off
  let (gvr, off) ← bytes data off
  let (sl, off) ← uint64 data off
  let (fk, off) ← fork data off
  -- History
  let (lbh, off) ← beaconBlockHeader data off
  let (br, off) ← arrayOf bytes data off
  let (sr, off) ← arrayOf bytes data off
  let (hr, off) ← arrayOf bytes data off
  -- Eth1
  let (e1d, off) ← eth1Data data off
  let (e1v, off) ← arrayOf eth1Data data off
  let (e1i, off) ← uint64 data off
  -- Registry
  let (vals, off) ← arrayOf validator data off
  let (bals, off) ← arrayOf uint64 data off
  -- Randomness
  let (rm, off) ← arrayOf bytes data off
  -- Slashings
  let (sls, off) ← arrayOf uint64 data off
  -- Participation
  let (pep, off) ← arrayOf uint8 data off
  let (cep, off) ← arrayOf uint8 data off
  -- Finality
  let (jb, off) ← bytes data off
  let (pjc, off) ← checkpoint data off
  let (cjc, off) ← checkpoint data off
  let (fc, off) ← checkpoint data off
  -- Inactivity
  let (is, off) ← arrayOf uint64 data off
  -- Sync committees
  let (csc, off) ← syncCommittee data off
  let (nsc, off) ← syncCommittee data off
  -- Execution
  let (leph, off) ← executionPayloadHeader data off
  -- Withdrawals
  let (nwi, off) ← uint64 data off
  let (nwvi, off) ← uint64 data off
  -- Historical summaries
  let (hs, off) ← arrayOf historicalSummary data off
  some ({
    genesisTime := gt, genesisValidatorsRoot := gvr, slot := sl, fork := fk,
    latestBlockHeader := lbh, blockRoots := br, stateRoots := sr, historicalRoots := hr,
    eth1Data := e1d, eth1DataVotes := e1v, eth1DepositIndex := e1i,
    validators := vals, balances := bals,
    randaoMixes := rm, slashings := sls,
    previousEpochParticipation := pep, currentEpochParticipation := cep,
    justificationBits := jb, previousJustifiedCheckpoint := pjc,
    currentJustifiedCheckpoint := cjc, finalizedCheckpoint := fc,
    inactivityScores := is,
    currentSyncCommittee := csc, nextSyncCommittee := nsc,
    latestExecutionPayloadHeader := leph,
    nextWithdrawalIndex := nwi, nextWithdrawalValidatorIndex := nwvi,
    historicalSummaries := hs
  }, off)

end Decode

-- ═══════════════════════════════════════════════
-- Convenience wrappers
-- ═══════════════════════════════════════════════

def serializeBeaconState (state : BeaconState) : ByteArray :=
  Encode.beaconState ByteArray.empty state

def deserializeBeaconState (data : ByteArray) : Option BeaconState :=
  match Decode.beaconState data 0 with
  | some (state, _) => some state
  | none => none

end Eth2
