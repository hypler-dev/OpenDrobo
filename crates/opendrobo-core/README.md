# opendrobo-core

Portable, standard-library-only decoding for OpenDrobo capacity pages. It does
not open hardware, mount filesystems, access the network, or retain caller
pointers.

## Error codes

The Rust API returns `CapacityError`; C ABI functions return its stable `i32`
code. Zero means success.

| Code | Meaning |
| ---: | --- |
| 0 | Success |
| 1 | Byte units were not confirmed |
| 2 | Record/device binding was not confirmed |
| 3 | Single-volume scope was not confirmed |
| 4 | Observation is in the future |
| 5 | Observation is stale (over 30,000 ms) |
| 6 | Timestamp arithmetic overflowed |
| 7 | Page is truncated |
| 8 | Page/subpage is unsupported |
| 9 | Declared body length is unsupported |
| 10 | Protected total is zero |
| 11 | Protected free/used/total counts disagree |
| 12 | Unprotected total is below protected total |
| 13 | Input page pointer is null |
| 14 | Output record pointer is null |
| 15 | Confirmation flags contain unknown bits |
| 16 | An internal panic was contained at the C boundary |

## C ABI pointer contract

`od_capacity_decode` borrows the input only for the call and copies no more
than 60 bytes. `page` must be non-null and point to at least `min(length, 60)`
valid bytes. `output` must be non-null, aligned, and writable as one
`ODCapacity`. Input and output may overlap: input is copied before output is
cleared. On every decode error with a valid `output`, the output record is
zeroed.
`od_capacity_validate` requires a non-null, aligned `ODCapacity` pointer valid
for the call. It validates freshness first, then nonzero and internally
consistent counts. No C ABI function retains either pointer.

## Offline transport and metadata inspection

`thunderbolt` decodes a copied BAR1 port directory, capabilities, and logical
LUN table, and encodes/decodes request and versioned completion frames. It
checks bounds, counts, arithmetic, and request/completion association. Logical
byte totals are explicitly virtual ceilings; they are not protected capacity.
The codec neither allocates DMA mappings nor dereferences or submits addresses.

`beyondraid` inspects an explicit 8192-byte RMR header candidate using the
recovered zero-seed/no-final-XOR CRC rule. Candidate structure and checksum
matching do not validate a disk or array. Caller words and revision-two pack
identity require separately supplied evidence; location, generation selection,
redundancy, and data reconstruction remain unresolved.

The `opendrobo-probe` workspace binary reads bounded windows from copied regular
files and calls these APIs without requiring the original Drobo driver. It
does not inspect live devices or mount a filesystem. See the
[transport recovery](../../research/THUNDERBOLT_TRANSPORT.md) and
[RMR layout](../../research/BEYONDRAID_LAYOUT.md) notes for exact evidence and
limitations. These APIs have no new C ABI exports; the existing capacity ABI
and its confirmation gate remain unchanged.
