/-
  Block Processing — Operations Dispatcher

  Process all operations in the block body.
  Reference: https://eth2book.info/latest/part3/transition/block/#operations
-/
import Guest.Eth2.Transition.Block.Operations.ProposerSlashing
import Guest.Eth2.Transition.Block.Operations.AttesterSlashing
import Guest.Eth2.Transition.Block.Operations.Attestation
import Guest.Eth2.Transition.Block.Operations.Deposit
import Guest.Eth2.Transition.Block.Operations.VoluntaryExit
import Guest.Eth2.Transition.Block.Operations.BlsToExecution

namespace Eth2

-- Helper to fold over an array with a fallible operation
private def foldSTF (state : BeaconState) (items : Array α)
    (f : BeaconState → α → STFResult BeaconState) : STFResult BeaconState :=
  items.foldlM (init := state) fun s item => f s item

def processOperations (state : BeaconState) (body : BeaconBlockBody) : STFResult BeaconState := do
  -- Verify deposit count (stub: skip since we don't track eth1 deposit tree)
  -- Process all operations in spec order
  let state ← foldSTF state body.proposerSlashings processProposerSlashing
  let state ← foldSTF state body.attesterSlashings processAttesterSlashing
  let state ← foldSTF state body.attestations processAttestation
  let state ← foldSTF state body.deposits processDeposit
  let state ← foldSTF state body.voluntaryExits processVoluntaryExit
  let state ← foldSTF state body.blsToExecutionChanges processBlsToExecutionChange
  .ok state

end Eth2
