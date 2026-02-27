/-
  Block Processing — Execution Payload (Stub)

  In production, this would verify the execution payload against the execution engine.
  Here, we only store the payload header in the state.
  Reference: https://eth2book.info/latest/part3/transition/block/#execution-payload
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Crypto
import Guest.Eth2.Transition.Block.Header

namespace Eth2

def processExecutionPayload (state : BeaconState) (payload : ExecutionPayload) : STFResult BeaconState :=
  -- Stub: skip timestamp, random, and execution engine verification
  -- Store the payload header in state
  let header : ExecutionPayloadHeader := {
    parentHash := payload.parentHash
    feeRecipient := payload.feeRecipient
    stateRoot := payload.stateRoot
    receiptsRoot := payload.receiptsRoot
    logsBloom := payload.logsBloom
    prevRandao := payload.prevRandao
    blockNumber := payload.blockNumber
    gasLimit := payload.gasLimit
    gasUsed := payload.gasUsed
    timestamp := payload.timestamp
    extraData := payload.extraData
    baseFeePerGas := payload.baseFeePerGas
    blockHash := payload.blockHash
    transactionsRoot := hashTreeRoot (ByteArray.mk #[])  -- stub
    withdrawalsRoot := hashTreeRoot (ByteArray.mk #[])    -- stub
  }
  .ok { state with latestExecutionPayloadHeader := header }

end Eth2
