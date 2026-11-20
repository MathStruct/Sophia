1. **Verified — The executable fails E-7’s byte-for-byte criterion, and the comparison has no failure gate.** `vault/Plans/Semantic Boundary Experiments.md:137`, `:144` and `experiments/staged-compilation/lux_train_step.jl:173`, `:218`. All eight saved executables differ. `run` prints the mismatch and continues. `compare` prints 64-bit digest prefixes for the first executable in each directory and accepts zero directories or repeated copies of the same directory. A successful exit can therefore accompany either a mismatch or zero independent comparisons.

   ```sh
   python3 -I -c 'from pathlib import Path; p=list((Path.home()/".cache/sophia-corpora/e7-2").glob("*/*.bin")); print(len(p), len({f.read_bytes() for f in p}))'
   sed -n '173,176p;218,227p' experiments/staged-compilation/lux_train_step.jl
   ```

   Output was `8 8`. The source prints `exe == exe2` and loops over `ARGS[2:end]`, with no assertion or minimum input count.

2. **Verified — The saved thunk differs in the perturbed process, and its size exceeds the reported glue size.** `experiments/staged-compilation/lux_train_step.jl:149`, `:155`, `:171`, `vault/Plans/Semantic Boundary Experiments.md:142`, `:143` and `vault/Storage and Materialization/AD as a Materialization Case.md:150`. The 19,548 B and common glue digest in the log describe a separate serialization of the thunk body. The file consumed by `load` is a 20,456 B serialization of the body and metadata. Its tag changes from `reactant#1194` to `reactant#1259` in the perturbed run.

   ```sh
   cd ~/.cache/sophia-corpora/e7-2
   wc -c {a,b,c,p}/thunk.jls
   sha256sum {a,b,c,p}/thunk.jls
   cmp -l {a,p}/thunk.jls
   ```

   All four files were 20,456 B. The SHA-256 prefix was `6dc0cf0f56c61280` for a/b/c and `f9f4beee7ceff12fc` for p. `cmp` found differences at one-based offsets 280–282.

3. **Verified — The subtraction labelled “XLA compile” mixes two compilation paths.** `experiments/staged-compilation/lux_train_step.jl:120`, `:165`, `vault/Plans/Semantic Boundary Experiments.md:140` and `vault/Storage and Materialization/AD as a Materialization Case.md:149`. Reactant’s `@code_hlo` disables Shardy passes and constructs a `TextualModule`. The timed `@compile serializable=true` also includes MLIR preparation and debug rendering, XLA option construction, HLO retrieval, and Julia thunk generation. The script subtracts the warm trace from the first `@compile`, while the notes use the later `@compile`. Both remainders mix work outside `XLA.compile`.

   ```sh
   sed -n '10,15p;27,32p;44,49p;61,66p' experiments/staged-compilation/results.txt
   p=~/.julia/packages/Reactant/aBNpn/src/compiler
   rg -n 'shardy_passes|TextualModule|drop-unsupported-attributes|show\(IOContext|make_compile_options|XLA.compile\(|get_hlo_modules\(|generate Julia' "$p"/{Macros,Compiler}.jl
   ```

   For a, the log shows 317.4 ms for warm `@code_hlo`, 1,920.8 ms for first `@compile`, a printed remainder of 1,603.4 ms, and 418.9 ms for the later `@compile`. Later compile minus warm trace gives 101.5, 112.0, 101.3 and 88.5 ms across a/b/c/p. The installed Reactant source confirms the additional work on the two paths.

4. **Verified — The restore check establishes numerical equality. The notes claim bitwise equality.** `experiments/staged-compilation/lux_train_step.jl:211`, `experiments/staged-compilation/lux_train_step.md:13`, `vault/Plans/Semantic Boundary Experiments.md:141` and `vault/Storage and Materialization/AD as a Materialization Case.md:149`. Array `==` and zero maximum absolute error both accept a change in the sign of a Float32 zero. The script prints a boolean and continues on mismatch. The four saved trajectories come from `run`, so they cannot independently establish the restored trajectory’s bytes.

   ```sh
   sed -n '202,214p' experiments/staged-compilation/lux_train_step.jl
   sed -n '73,82p' experiments/staged-compilation/results.txt
   rg -n '^==\(x::T, y::T\).*eq_float' /nix/store/*julia*1.12.7*/share/julia/base/float.jl
   python3 -I -c 'import struct; a,b=0.0,-0.0; print(a==b, abs(a-b), struct.pack("<f",a).hex(), struct.pack("<f",b).hex())'
   ```

   The log reports `true` for both comparisons and `max |Δ| = 0.0`. Julia’s source dispatches Float32 `==` to `eq_float`. The counterexample prints `True 0.0 00000000 00000080`. This verifies a gap in the test. The restored trajectory’s bitwise result remains unverified.

5. **Verified — The schema-free sorter can report a false match on another layout.** `experiments/staged-compilation/executable_objects.py:22`, `:35`, `:43`, `:57` and `experiments/staged-compilation/executable_objects.md:7`, `:11`. The parser accepts any length-delimited outer field whose payload has the expected shape. It can therefore erase the order of a semantically ordered repeated field or sort entries from different field numbers into the same result. It also labels the byte preceding a difference as a protobuf tag without checking field boundaries. Finally, sorting intact raw entries preserves length, so the note’s varint-length explanation for a length difference does not follow from this operation.

   ```sh
   python3 -I - <<'PY'
   from pathlib import Path
   exec(Path('experiments/staged-compilation/executable_objects.py')
        .read_text().split('h = lambda')[0])

   def entry(tag, value):
       return bytes([tag, 9, 10, 4]) + b'\x7fELF' + bytes([18, 1, value])

   p = b'\x18\x01'
   a, b, c = entry(10, 97), entry(10, 98), entry(18, 98)
   for x, y in [(a+b, b+a), (a+c, c+a)]:
       print(x != y, canonical(p+x+p) == canonical(p+y+p), canonical(p+x+p)[1])

   x, y = bytes.fromhex('0a021802'), bytes.fromhex('0a021804')
   print(Counter((y[i-1], x[i], y[i]) for i in range(len(x)) if x[i] != y[i]))
   PY
   ```

   Output was `True True 2` twice and `Counter({(24, 2, 4): 1})`. The last change lies inside a field-1 byte string, although the formatter would call `0x18` a tag. These cases verify the generic failure mode. A separate wire walk of the supplied blobs against Reactant’s generated schema confirmed that their 33 entries are object-file fields and their 59 residual differences are module-ID fields.

6. **Verified — The startup and restore numbers omit work, and the plans note miscounts processes.** `experiments/staged-compilation/lux_train_step.jl:75`, `:98`, `:107`, `:179`, `:197`, `:213`, `vault/Plans/Semantic Boundary Experiments.md:139`–`:141` and `vault/Storage and Materialization/AD as a Materialization Case.md:147`–`:149`. The reported ~34 s includes an unrelated compile and execution, while excluding package loading, device and fixture setup, and the second trace. The plain interval includes its fixture and optimizer setup. The 71 ms restore excludes reading the executable, deserializing the thunk, and looking up the backend and device, while including the StableHLO read. The 1.6 s transfer interval also includes data generation and model and optimizer initialization. The 32.3 s compile uses default serialization options and runs after restored training, so it measures a different interval from the 2.3 s subtotal through first restored execution. The log records four compile processes plus one restore process.

   ```sh
   rg -n '^== .* (run|load) |3795.8|14335.9|18243.7|317.4|1920.8|400.8|3695.6|71.1|1585.8|640.4|32303.3' experiments/staged-compilation/results.txt
   python3 -I -c 'print((14335.9+18243.7+1920.8+400.8)/1000, (3795.8+14335.9+18243.7+317.4+1920.8+400.8)/1000, (71.1+1585.8+640.4)/1000, (3695.6+71.1+1585.8+640.4)/1000)'
   ```

   The log lists `run a`, `run b`, `run c`, `run p perturb`, and `load a`. The sums are 34.9012 s for the selected a timers, 39.0144 s for all its logged intervals through first execution, and 2.2973 s and 5.9929 s for the restore subtotal before and after including imports. Untimed work remains outside these sums.

7. **Verified — The step timings include scalar readback only on the Reactant side and use the lower middle sample as the median.** `experiments/staged-compilation/lux_train_step.jl:55`, `:84`, `:89`, `:169`, `:210` and `vault/Plans/Semantic Boundary Experiments.md:140`. The compiled path measures the thunk plus conversion of its loss to a host Float32. The plain path already returns a host scalar. The comparison establishes host-observed latency with readback included on the Reactant side. It does not isolate compiled training computation. All three timing paths report sample 100 of 200 as the median, which also requires sample 101.

   ```sh
   rg -n 'Float32\(l\)|@elapsed \(\(ps|div\(end, 2\)' experiments/staged-compilation/lux_train_step.jl
   rg -n 'to_number\(X::ConcretePJRTScalar|XLA.to_host\(get_buffer' ~/.julia/packages/Reactant/aBNpn/src/ConcreteRArray.jl
   python3 -I -c 'import statistics; x=list(range(1,201)); print(x[len(x)//2-1],statistics.median(x))'
   ```

   The source shows `Float32(l)` and `XLA.to_host` on the compiled path and a direct `train_step` call on the plain path. The example prints `100 100.5`.

8. **Verified — The deterministic-program and cache-key claims exceed the tested scope.** `vault/Plans/Semantic Boundary Experiments.md:144`, `vault/Storage and Materialization/AD as a Materialization Case.md:151` and `experiments/staged-compilation/lux_train_step.jl:132`. The saved IR text matches for one fixture and configuration across four compilation processes on one host. The batch-128 build contributes only a byte count. Reactant constructs XLA compile options separately from the saved IR text, and the experiment never recompiles from that text. The proposed program ID plus versions and host therefore remains an untested cache key for compatible executables.

   ```sh
   cmp ~/.cache/sophia-corpora/e7-2/{a,p}/stablehlo.mlir
   rg -n 'cb = |exeb = |write\(' experiments/staged-compilation/lux_train_step.jl
   rg -n 'module_string =|make_compile_options|compile_options=xla_compile_options' ~/.julia/packages/Reactant/aBNpn/src/compiler/Compiler.jl
   ```

   `cmp` exited 0. The script saves the batch-64 IR and executable and reports `length(exeb)` for batch 128. Reactant captures `module_string` at line 1331 and constructs and passes compile options separately at lines 1340 and 1364.

9. **Verified — The StableHLO location count is a count of lines.** `vault/Plans/Semantic Boundary Experiments.md:143`. There are 197 location-bearing lines and 232 printed `loc(...)` forms, including the alias definition. All the forms still resolve to `unknown`.

   ```sh
   python3 -I -c 'from pathlib import Path; from collections import Counter; import re; s=(Path.home()/".cache/sophia-corpora/e7-2/a/stablehlo.mlir").read_text(); print(sum("loc(" in x for x in s.splitlines()), Counter(re.findall(r"loc\(([^\n)]*)\)", s)))'
   ```

   Output was `197 Counter({'#loc': 195, 'unknown': 37})`.