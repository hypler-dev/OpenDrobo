# Spare-unit lab scaffold

This scaffold prepares an offline, non-executing test record for a future disposable Drobo enclosure and disposable member disks. It does not enumerate hardware, open a driver, mount a filesystem, issue a control command, format, reset, erase, replace a drive, or reconstruct a pool.

## Implemented now

`scripts/spare_unit_lab.py` uses only the Python standard library. It can write a manifest template, validate a supplied manifest, and generate a deterministic plan plus bounded evidence receipts from copied JSON files. Input evidence must be a regular, non-symlink JSON file no larger than 1 MiB. Outputs have fixed names inside the explicit `--out-dir` and use exclusive creation, so stale manifests and receipts are never overwritten.

Keep manifests and receipts in a private `artifacts/spare-unit-lab/` directory. They are local operator evidence, not source fixtures, public hardware metadata, or release assets.

The manifest records a stable opaque unit fingerprint, stable disposable member-disk fingerprints, target model/firmware/operating-system/transport fields, protected/home-unit exclusions, and explicit pending unknowns. Fingerprint values must use `sha256:` followed by 64 lowercase hexadecimal digits, derived from durable hardware identifiers. The format does not attest identity or prove that the operator hashed a durable identifier. Raw transient registry IDs, labels, duplicate disks, a missing identity, a protected member disk, and a target matching a protected or home unit are rejected. Real manifests cannot use `pending` or whitespace-only target fields. A manifest boolean cannot authorize destructive work.

## Operator quickstart

```sh
python3 scripts/spare_unit_lab.py init --out-dir artifacts/spare-unit-lab/manifest
# This generic plan works before hardware exists. It contains no unit or disk identity.
python3 scripts/spare_unit_lab.py plan --template --out-dir artifacts/spare-unit-lab/template-plan
# Edit spare-unit-lab-manifest.json only with stable, non-home, disposable identities.
python3 scripts/spare_unit_lab.py validate --manifest artifacts/spare-unit-lab/manifest/spare-unit-lab-manifest.json
python3 scripts/spare_unit_lab.py plan \
  --manifest artifacts/spare-unit-lab/manifest/spare-unit-lab-manifest.json \
  --out-dir artifacts/spare-unit-lab/real-plan \
  --pool-json /path/to/copied-pool.json
```

`plan --template` is mutually exclusive with `--manifest`, refuses copied evidence, and remains `PENDING_HARDWARE` with no identity. A real-manifest plan remains `PENDING_HARDWARE` or `BLOCKED`; it never reports `READY`. A changed unit fingerprint or disposable member inventory blocks the plan. Protected member-disk fingerprints cannot be listed as disposable. Remove neither stale receipts nor exclusions to bypass that block; create a newly reviewed explicit output directory instead.

## Copied evidence envelope

For a real manifest, wrap each copied pool, storage, or scan JSON object in this local envelope before supplying it with `--pool-json`, `--storage-json`, or `--scan-json`:

```json
{
  "capturedAtMs": 1800000000000,
  "unitFingerprint": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "memberDiskFingerprints": ["sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"],
  "target": {
    "model": "<confirmed-model>",
    "firmware": "<confirmed-firmware>",
    "operatingSystem": "<confirmed-operating-system>",
    "transport": "<confirmed-transport>"
  },
  "payload": {"copiedRawCliJson": "replace this object with the copied raw CLI JSON"}
}
```

The hashes in the example are illustrative placeholders, not hardware identities. Replace them and the timestamp with verified local values. The payload must be a nonempty object; this scaffold does not certify its provider-specific protocol contents.

This is a local operator wrapper, not an attestation. The tool records the SHA-256 and byte count of the whole envelope as `operator-asserted-not-attested`; it does not trust a raw label or fabricate a hardware identity. Every supplied envelope must contain all shown fields, have a non-boolean integer timestamp no more than five minutes in the future and no more than 24 hours old, and exactly match the real manifest target, unit, and disk inventory. Missing, stale, future, unbound, or mismatched supplied evidence blocks the plan. No capture command is implemented or needed to create the envelope: copy existing CLI JSON into `payload` manually.

## Future manual research only

The generated sequence reserves controlled format/reset, capacity profiling, drive migration, fault replacement, and recovery for future manual work. Each requires current serial/stable-identity confirmation, current disk inventory, isolation from protected/home units, backup evidence, and separate explicit destructive authorization at execution time. Before any reconstruction, make bit-for-bit member-disk image copies, record hashes, and use a read-only BeyondRAID parser against those copies. This repository does not implement a formatter, reset dispatcher, driver replacement, recovery writer, or hardware execution switch.

## Machine acceptance

```sh
python3 -m unittest tests/test_spare_unit_lab.py
```

Acceptance requires: template creation and a generic no-hardware plan without invented identity; manifest validation rejecting protected/home/protected-disk, duplicate, missing, or transient identity; deterministic no-hardware plans; stale/future/unbound/wrong-target copied envelopes reported as blocked; unsupported execution flags rejected; and stale output refusal. These software checks do not prove a device, disk, driver, filesystem, recovery method, or future destructive authorization.
