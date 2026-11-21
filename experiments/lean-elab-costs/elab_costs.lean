/-
B-035 part 2 (SPEC-001 §8a, SR-152): the elaboration cost of each command of
one Lean file, as Lean's own frontend spends it. See `elab_costs.md`.

    lake env lean --run elab_costs.lean FILE MODULE   (from the project root)

It elaborates FILE the way `lean FILE` does, but command by command and
synchronously (`Elab.async = false`, so a proof's time is charged to its
command), and prints one TSV row per source command:

    index  line  kind  decl_kind  constants_added  errors  ns  heartbeats

`kind` is the command's syntax kind; `decl_kind` the declaration keyword's
syntax kind for a `declaration`, through `... in` prefixes. A final row
`#header` gives the import time. A macro-expanded command is timed with the
source command it came from, unlike the trace profiler, which prints each
expansion as its own top-level node.
-/
import Lean
open Lean Elab Frontend

/-- The syntax kind of a declaration's keyword, looking through `... in`. -/
partial def declKind (stx : Syntax) : Name :=
  if stx.getKind == ``Lean.Parser.Command.in then declKind stx[2]
  else if stx.getKind == ``Lean.Parser.Command.declaration then stx[1].getKind
  else stx.getKind

def errorCount (log : MessageLog) : Nat :=
  log.toList.foldl (fun n m => if m.severity == .error then n + 1 else n) 0

unsafe def main (args : List String) : IO UInt32 := do
  let [file, modName] := args | IO.eprintln "usage: elab_costs FILE MODULE"; return 2
  enableInitializersExecution
  initSearchPath (← findSysroot)
  let input ← IO.FS.readFile file
  let inputCtx := Parser.mkInputContext input file
  let mainModule := modName.toName
  let t0 ← IO.monoNanosNow
  let (header, parserState, messages) ← Parser.parseHeader inputCtx
  let opts := Elab.async.set {} false
  let (env, messages) ← processHeader header opts messages inputCtx (trustLevel := 1024) (leakEnv := true) (mainModule := mainModule)
  let env := env.setMainModule mainModule
  let t1 ← IO.monoNanosNow
  let out ← IO.getStdout
  out.putStrLn s!"#header\t1\t-\t-\t0\t{errorCount messages}\t{t1 - t0}\t0"
  let ctx : Frontend.Context := { inputCtx }
  let mut st : Frontend.State :=
    { commandState := Command.mkState env messages opts, parserState, cmdPos := parserState.pos }
  let mut i := 0
  repeat
    let before := st.commandState
    let pos := st.parserState.pos
    let h0 ← IO.getNumHeartbeats
    let n0 ← IO.monoNanosNow
    let (done, st') ← (processCommand.run ctx).run st
    -- Work the command started but did not finish (kernel replay of lazily
    -- realized constants, snapshot tasks) is charged to it: wait for it.
    let _ ← IO.wait st'.commandState.env.checked
    for t in st'.commandState.snapshotTasks do
      let tree ← IO.wait t.task
      let _ ← IO.wait (← tree.waitAll)
    let n1 ← IO.monoNanosNow
    let h1 ← IO.getNumHeartbeats
    st := st'
    let cmd := st.commands.back!
    let line := (inputCtx.fileMap.toPosition pos).line
    let nLocal (e : Environment) := e.constants.foldStage2 (fun n _ _ => n + 1) 0
    let added := nLocal st.commandState.env - nLocal before.env
    -- `elabCommandTopLevel` resets the message log per command, so the log
    -- now holds this command's messages only.
    let errs := errorCount st.commandState.messages
    if !done then
      out.putStrLn s!"{i}\t{line}\t{cmd.getKind}\t{declKind cmd}\t{added}\t{errs}\t{n1 - n0}\t{h1 - h0}"
    i := i + 1
    if done then break
  return 0
