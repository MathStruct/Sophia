#experiment

> E-7 (B-028): says how two serialised XLA CPU executables differ. It finds the run of object-file entries (each an ELF object and its kernel name), sorts them, and compares each file with the first, byte by byte, naming the byte before every byte that still differs. Source: `executable_objects.py`. Used by [[lux_train_step]].
>
> Sources: the protobuf wire format (varints, length-delimited fields); the ELF header. code: `executable_objects.py`

The executable is a protobuf message that the PJRT client of Reactant 0.2.295 (CPU build) writes. Its schema is not read here. The script recognises an entry by its shape: a length-delimited field whose payload is exactly `{1: bytes starting with \x7fELF, 2: name}`. It asserts that the entries are contiguous, that they share one field tag (so sorting does not mix fields), and that no ELF object lies outside them. Sorting the raw entries gives a canonical order. The bytes outside the entries are left as they are. A remaining difference is reported with the byte before it. On E-7's files that byte is a protobuf tag (the module id's field), which Codex checked against Reactant's schema; the script does not know it.

Run with `python3 -I` (no site packages, nothing from the current directory).

**Limits.** It fits this XLA's layout. Without the schema it cannot know that the order of the entries carries no meaning; that was checked once, for these files. Another Reactant_jll may nest the objects differently. Then the assertions or the search for the first entry may fail, but a run that happens to match the shape would be sorted all the same: on another version a match is a hint, not a result. Files of different lengths are only reported as such.
