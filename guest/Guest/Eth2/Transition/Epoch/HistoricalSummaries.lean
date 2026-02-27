/-
  Epoch Processing — Historical Summaries Update (Capella)

  At the end of each history accumulation period, append a summary.
  Reference: https://eth2book.info/latest/part3/transition/epoch/#historical-summaries-updates
-/
import Guest.Eth2.Helpers
import Guest.Eth2.Crypto

namespace Eth2

def processHistoricalSummariesUpdate (state : BeaconState) : BeaconState :=
  let nextEpoch := getCurrentEpoch state + 1
  -- SLOTS_PER_HISTORICAL_ROOT / SLOTS_PER_EPOCH = number of epochs per history period
  let epochsPerPeriod := SLOTS_PER_HISTORICAL_ROOT / SLOTS_PER_EPOCH
  if nextEpoch % epochsPerPeriod == 0 then
    -- Compute Merkle roots of block_roots and state_roots (stubbed)
    let blockSummaryRoot := hashTreeRoot (ByteArray.mk #[])  -- stub
    let stateSummaryRoot := hashTreeRoot (ByteArray.mk #[])  -- stub
    let summary : HistoricalSummary := {
      blockSummaryRoot := blockSummaryRoot
      stateSummaryRoot := stateSummaryRoot
    }
    { state with historicalSummaries := state.historicalSummaries.push summary }
  else state

end Eth2
