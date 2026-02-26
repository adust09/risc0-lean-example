/-
  Epoch Processing — Participation Flag Updates (Altair)

  Rotate participation: current → previous, reset current to zeros.
  Reference: https://eth2book.info/latest/part3/transition/epoch/#participation-flag-updates
-/
import Guest.Eth2.Helpers

namespace Eth2

def processParticipationFlagUpdates (state : BeaconState) : BeaconState :=
  { state with
    previousEpochParticipation := state.currentEpochParticipation
    currentEpochParticipation := Array.replicate state.validators.size 0
  }

end Eth2
