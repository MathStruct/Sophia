#comparison #reference

> How the operating system hinders or helps a project like Sophia (SR-68). Sophia is a store of structured, content-addressed objects with names as labels and annotations as attributes. That is close to what Lisp-machine and database operating systems provided natively, and far from what Unix provides: byte-stream files, hierarchical paths, processes that talk through bytes. This note looks at Strandh's LispOS (CLOSOS), at the database operating systems that were built, and at what they mean for Sophia. **Conclusion:** run on a conventional OS (Linux first) and pay the boundary costs knowingly. Borrow three of CLOSOS's ideas inside Sophia: per-user environments, attribute-based lookup, and capabilities as proofs. A Sophia-native OS is a long-term research direction, not a prerequisite.
>
> Sources: Strandh, *CLOSOS: Specification of a Lisp Operating System*, 2013 (metamodular.com/lispos.pdf) and *Lisp Operating System* (metamodular.com/Common-Lisp/lispos.html); Soltis, *Inside the AS/400*, 1996; Giampaolo, *Practical File System Design with the Be File System*, 1999; Shapiro, Smith and Farber, *EROS: A Fast Capability System*, SOSP 1999; Skiadopoulos et al., *DBOS: A DBMS-Oriented Operating System*, PVLDB 15(1), 2021; Mezzano (github.com/froggey/Mezzano), a Common Lisp OS; Wirth and Gutknecht, *Project Oberon*, 1992; Goldberg and Robson, *Smalltalk-80*, 1983 (images); Watson et al., *CHERI*, IEEE S&P 2015. code: none.

## 1. What CLOSOS proposes

Robert Strandh's LispOS is "not just another operating system that happens to be written in Lisp". It is an operating system whose interface to users and applications is a Lisp environment. Its criticisms of Unix, and its answers:

| Unix concept | Strandh's criticism | CLOSOS answer |
|---|---|---|
| **process** (a separate address space per program) | "an unfortunate necessity due to the limited address space of … the 1970s". Programs exchange complex data by serialising it to bytes through pipes and parsing it again | **a single address space**. Pointers are globally valid, and applications share objects by passing references. Protection comes from a "trusted compiler" that cannot produce code which forges pointers |
| **hierarchical file system** | "most things are not naturally hierarchical", and "a file is only a sequence of bytes with no structure" | **an object store with tags** (key/value attributes: category, author, language, format, date …). Searches mostly use the tags. A "directory" is just another object |
| **primary vs secondary memory** | the save-or-lose conundrum | **a single memory abstraction**: data is permanent, and RAM is a cache for the disk. **Checkpointing** (EROS-style, or log-structured) gives crash consistency |
| **monolithic kernel** | updates need reboots, and modules have full access | no kernel in the classic sense (as in Multics): system code is ordinary, replaceable functions |
| **access control** | — | **capabilities**: tagged pointers whose top 4 bits are access bits, set by the object store from access-control lists. Code compiled by an untrusted compiler runs in user mode in its own address space |
| **one global environment** | users interfere with each other's methods | **multiple first-class global environments**. Each user can add methods to standard generic functions (`print-object`) without affecting others. Immutable objects are shared between environments |
| **garbage collection** | Unix gets away with reference counting, because files have no cycles | a **two-level GC**: a per-thread local heap of about cache size (~4 MiB), collected by its thread in milliseconds, plus a global non-moving mark-and-sweep heap collected **concurrently** with the threads, using write barriers |

Strandh also proposes a practical path: build the system first as a Unix process, single-user and then multi-user, and only later make it bootable.

## 2. Database operating systems that were built

| System | What it did | Lesson for Sophia |
|---|---|---|
| **IBM System/38 → AS/400 → IBM i** (1978 onwards) | a **single-level store** (one 64-bit, later 128-bit, address space over RAM and disk); every object is typed; a relational database integrated into the OS; programs compiled to a machine-independent interface (TIMI) and re-translated when hardware changed | it shipped, and ran businesses for decades. A machine-independent stored form plus re-translation is Sophia's "core in the store, compile on demand" at OS scale |
| **Pick** (1965 onwards) | the OS *is* a multi-valued database; files are database tables | a database as the primary abstraction is workable for whole application ecosystems |
| **BeOS BFS** (1999) | extended attributes on files, indexed and queryable ("live queries") | attribute queries over an object store are fast enough for desktop use; close to CLOSOS's tags and Sophia's names and annotations |
| **WinFS** (cancelled 2006) | a relational store under the Windows file system | failed on performance and scope: the cost of putting *everything* in a database, at once, with backward compatibility. A warning about scope (SR-73) |
| **Smalltalk-80 images, Symbolics Genera** | the whole environment is persistent objects; code is objects; the editor, compiler and debugger are in the same space | the closest existing experience of "code as live objects in a store". Their weakness was sharing and versioning between images, which is exactly what content addressing solves |
| **KeyKOS / EROS** | capability OS with transparent persistence by checkpointing the whole system | persistence and capabilities fit together. CLOSOS borrows EROS's checkpointing |
| **Mezzano** | a working Common Lisp OS (x86-64), booting on real hardware and in virtual machines | a single-address-space Lisp OS can be built today by a small team |
| **DBOS** (MIT and Stanford, 2020 onwards) | OS state (scheduling, files, messages) kept in a distributed DBMS (VoltDB / FoundationDB) | provenance and queryability of *all* system state; the cost was measured and found acceptable for cloud workloads |

## 3. How a Unix-like OS hinders Sophia

1. **Bytes at every boundary.** Sophia's objects are trees and DAGs. Every tool boundary, whether a compiler invocation, the language server protocol, a Moonkale process, git, or an LLM tool call, serialises them to bytes and parses them back. That is CLOSOS's pipe criticism, and it is Sophia's daily cost. *Mitigation:* keep the store **in-process** where possible (an embedded database, not a server), and use one canonical binary framing everywhere ([[SPEC-001 Sophia Store v0]] §4.1).
2. **Files as the unit of tools.** Compilers, package managers and version control all expect files (Moonkale ADR-0028). *Mitigation:* projections print files when an outside tool needs them. The store is the truth, and files are exports.
3. **Separate address spaces.** Moonkale, Sophia, compilers and language servers are separate processes, so object identity does not survive crossing between them. *Mitigation:* content ids *are* process-independent pointers. The 32-byte id is Sophia's globally valid "pointer", available in every process and on every machine. That is the part of the single-address-space idea that works on Unix.
4. **Volatile vs persistent memory.** The store is on disk and the working set is in RAM. *Mitigation:* the database's write-ahead log and memory-mapped reads (LMDB-style) approximate a single-level store.
5. **GC on another core** (SR-56) needs CPU affinity, real-time priorities and memory barriers. Linux provides these (thread pinning, `isolcpus`, cgroups), Windows and macOS less directly. *Mitigation:* the GC runtime has an OS layer, and the benchmark suite measures pause times per OS.

## 4. How the OS could help, and what Sophia borrows now

| CLOSOS idea | Its Sophia counterpart, now |
|---|---|
| **multiple global environments** (per-user methods on standard generic functions) | **per-environment method tables.** A nominal generic function's method table is a versioned relation (SPEC-001 §4.3). Making it versioned *per environment* (per user, per branch, per project) is the same idea, and it eases [[SP-006 Open method tables versus content addressing]]: a user's extra `print` method does not invalidate anyone else's compiled code |
| **tags instead of a hierarchy** | names are paths in v0, but annotations and facts are attributes. Querying by attributes ("all definitions by this author, in this language, with a stale proof") is the `sql`/`datalog` dialect. A tag-style namespace view is a later option |
| **capabilities via a trusted compiler** | **proofs as capabilities.** Sophialang's safe level is a proof that a section cannot forge or misuse memory (SPEC-002 §2, §5). A store that only *runs* code whose `Unsafe` sections are discharged by trusted checkers is CLOSOS's trusted-compiler protection, made explicit and auditable |
| **two-level GC with a concurrent global collector** | a design candidate for sophialang's GC (SPEC-002 §5). Thread-local heaps sized to the cache line up with [[Block Size and Hash Overhead]] |
| **checkpointing** | the store's transaction log already gives crash consistency for code. For *running* programs, checkpointing is out of scope until there is an interpreter |
| **object store as the OS's file system** | long-term (SR-74 and beyond): if Moonkale and its tools all used Sophia as their store, the "OS" seen by the developer would be Sophia, running as a set of Unix processes as Strandh suggests for the first steps |

## 5. Assessment

- **Now:** Linux first, then macOS, Windows and Android through Moonkale's platforms. No OS work.
- **Medium term:** borrow per-environment method tables (SP-006), attribute queries, and proofs as capabilities.
- **Long term, as research:** a Sophia environment that is itself a CLOSOS-like system (single address space, object store, capabilities from proofs), first as a Unix process. Mezzano shows a bootable version is feasible for a small team, but it is not a goal while the store and the language do not exist.
- **Why it might fail:** WinFS shows that "everything in a database" fails when it must also be backward compatible with everything. Sophia avoids that by *not* replacing files for other languages (ADR-0028 option B), only for its own.

## Related

[[Hardware for Sophia]] · [[SPEC-002 Sophialang Principles]] · [[SP-006 Open method tables versus content addressing]] · [[Minimal Binaries]]
