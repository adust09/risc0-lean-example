/-
  Epoch Processing — Dispatcher

  Calls all 12 epoch sub-processing functions in spec order.
  Reference: https://eth2book.info/latest/part3/transition/epoch/
-/
import Guest.Eth2.Transition.Epoch.Justification
import Guest.Eth2.Transition.Epoch.InactivityUpdates
import Guest.Eth2.Transition.Epoch.RewardsAndPenalties
import Guest.Eth2.Transition.Epoch.RegistryUpdates
import Guest.Eth2.Transition.Epoch.Slashings
import Guest.Eth2.Transition.Epoch.EffectiveBalances
import Guest.Eth2.Transition.Epoch.Resets
import Guest.Eth2.Transition.Epoch.ParticipationFlags
import Guest.Eth2.Transition.Epoch.SyncCommittee
import Guest.Eth2.Transition.Epoch.HistoricalSummaries

namespace Eth2

-- Process epoch transition: 12 sub-functions in spec order
def processEpoch (state : BeaconState) : BeaconState :=
  state
  |> processJustificationAndFinalization    -- 1
  |> processInactivityUpdates               -- 2
  |> processRewardsAndPenalties             -- 3
  |> processRegistryUpdates                 -- 4
  |> processSlashings                       -- 5
  |> processEth1DataReset                   -- 6
  |> processEffectiveBalanceUpdates         -- 7
  |> processSlashingsReset                  -- 8
  |> processRandaoMixesReset                -- 9
  |> processHistoricalSummariesUpdate       -- 10
  |> processParticipationFlagUpdates        -- 11
  |> processSyncCommitteeUpdates            -- 12

end Eth2
