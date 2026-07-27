# Attestation Error Codes

This document lists every typed [`EscrowError`](../escrow/src/lib.rs) code that the
attestation entrypoints can emit, the exact condition that triggers each one, and how to avoid it.

All codes are **stable and append-only** — SDKs must branch on the numeric
`ContractError(code)`, not on panic-string text.

## Covered entrypoints

| Entrypoint | Auth role | Description |
| --- | --- | --- |
| `bind_primary_attestation_hash(digest)` | Admin | Single-write 32-byte primary attestation digest. Once bound, cannot be overwritten. |
| `append_attestation_digest(digest)` | Admin | Append a 32-byte digest to the bounded append log. |
| `revoke_attestation_digest(index)` | Admin | Mark a single append-log entry as revoked. |
| `revoke_attestation_digests(indices)` | Admin | Atomically revoke multiple entries by index. |
| `unrevoke_attestation_digest(index)` | Admin | Clear the revocation flag on an append-log entry. |

### Constants referenced by error codes

| Constant | Value | Description |
| --- | ---: | --- |
| `MAX_ATTESTATION_APPEND_ENTRIES` | 32 | Maximum entries in the append log. |
| `MAX_ATTESTATION_REVOKE_BATCH` | 32 | Maximum indices per `revoke_attestation_digests` call. |

---

## Error Reference

### Pre-condition: escrow not initialised

| Code | Variant | Entrypoint(s) | When it fires | How to avoid it |
| ---: | --- | --- | --- | --- |
| 20 | `EscrowNotInitialized` | all attestation entrypoints | `DataKey::Escrow` is absent from storage — the contract has not been initialised. | Call `init` before any attestation entrypoint. |

### Primary attestation hash errors

| Code | Variant | Entrypoint(s) | When it fires | How to avoid it |
| ---: | --- | --- | --- | --- |
| 50 | `PrimaryAttestationAlreadyBound` | `bind_primary_attestation_hash` | `DataKey::PrimaryAttestationHash` already exists in storage. The primary hash is **single-write**: once set it cannot be replaced. | Check `get_primary_attestation_hash()` before calling. If a value is returned, the primary digest is already final for this escrow instance. |

### Append-log errors

| Code | Variant | Entrypoint(s) | When it fires | How to avoid it |
| ---: | --- | --- | --- | --- |
| 51 | `AttestationAppendLogCapacityReached` | `append_attestation_digest` | The append log already contains `MAX_ATTESTATION_APPEND_ENTRIES` (32) entries. New digests cannot be appended once the cap is hit. | Query `get_attestation_append_log()` to check current log length before appending. If the log is full, no further appends are possible on this escrow instance. |

### Index validation errors (single-index operations)

| Code | Variant | Entrypoint(s) | When it fires | How to avoid it |
| ---: | --- | --- | --- | --- |
| 52 | `AttestationIndexOutOfRange` | `revoke_attestation_digest`, `unrevoke_attestation_digest` | `index >= log.len()` — the requested index is beyond the end of the current append log. | Read `get_attestation_append_log()` to determine valid indices (`0` to `log.len() - 1`). |
| 53 | `AttestationAlreadyRevoked` | `revoke_attestation_digest` | The entry at `index` already has `DataKey::AttestationRevoked(index)` set to `true`. | Check `is_attestation_revoked(index)` before calling. If `true`, the entry is already revoked. |
| 56 | `AttestationNotRevoked` | `unrevoke_attestation_digest` | The entry at `index` does not have a revocation flag (`DataKey::AttestationRevoked(index)` is absent). | Check `is_attestation_revoked(index)` before calling. If `false`, the entry is not revoked and cannot be unrevoked. |

### Batch-revoke errors

`revoke_attestation_digests` validates the entire batch before mutating any state. If **any**
per-index check fails, the whole call is rolled back atomically — no partial revocation occurs.

| Code | Variant | Entrypoint(s) | When it fires | How to avoid it |
| ---: | --- | --- | --- | --- |
| 54 | `AttestationBatchEmpty` | `revoke_attestation_digests` | `indices` vector has length 0. | Always pass at least one index. Guard the call client-side if your list may be empty. |
| 55 | `AttestationBatchTooLarge` | `revoke_attestation_digests` | `indices.len() > MAX_ATTESTATION_REVOKE_BATCH` (32). | Split large index lists into chunks of ≤ 32 and call `revoke_attestation_digests` once per chunk. |
| 52 | `AttestationIndexOutOfRange` | `revoke_attestation_digests` | Any entry in `indices` satisfies `index >= log.len()`. | Validate every index against the current log length before building the batch. |
| 53 | `AttestationAlreadyRevoked` | `revoke_attestation_digests` | Any entry in `indices` is already revoked, **or** `indices` contains a duplicate (the second occurrence of a duplicate fails here because the first occurrence revoking that index makes it already-revoked). | Pre-check each index with `is_attestation_revoked`. De-duplicate the `indices` list before calling. |

---

## Guard-ordering summary

### `bind_primary_attestation_hash(digest)`
1. `load_escrow_require_admin` → `EscrowNotInitialized` (20) if storage missing; Soroban host auth failure if caller is not admin.
2. Primary-hash existence check → `PrimaryAttestationAlreadyBound` (50)

### `append_attestation_digest(digest)`
1. `load_escrow_require_admin` → `EscrowNotInitialized` (20); Soroban host auth failure if not admin.
2. Log-length check → `AttestationAppendLogCapacityReached` (51)

### `revoke_attestation_digest(index)`
1. `get_escrow` → `EscrowNotInitialized` (20)
2. `admin.require_auth()` — Soroban host auth failure
3. Index range check → `AttestationIndexOutOfRange` (52)
4. Already-revoked check → `AttestationAlreadyRevoked` (53)

### `revoke_attestation_digests(indices)`
1. Batch-size guards → `AttestationBatchEmpty` (54) / `AttestationBatchTooLarge` (55)
2. `get_escrow` → `EscrowNotInitialized` (20)
3. `admin.require_auth()` — Soroban host auth failure
4. Per-index (in order, all-or-nothing): `AttestationIndexOutOfRange` (52) then `AttestationAlreadyRevoked` (53)

### `unrevoke_attestation_digest(index)`
1. Index range check → `AttestationIndexOutOfRange` (52) _(note: precedes auth in this entrypoint)_
2. Not-revoked check → `AttestationNotRevoked` (56)
3. `get_escrow` + `admin.require_auth()` — `EscrowNotInitialized` (20) or Soroban host auth failure

---

## Authorization

All attestation-mutating entrypoints require the current `InvoiceEscrow::admin` address. If an
unauthorized caller invokes them, Soroban's native authentication framework rejects the
invocation with a host auth failure — not a typed `EscrowError` code.

Read-only attestation entrypoints require no authentication:

| Entrypoint | Returns |
| --- | --- |
| `get_primary_attestation_hash()` | `Option<BytesN<32>>` — `None` when unbound. |
| `get_attestation_append_log()` | `Vec<BytesN<32>>` — full log (active and revoked entries). |
| `get_attestation_digest_at(index)` | `Option<AttestationDigestInfo>` — `None` when `index >= log.len()`. |
| `is_attestation_revoked(index)` | `bool` — `false` when no revocation key exists. |
| `get_revoked_attestation_digests(start, limit)` | Paginated list of revoked entries. Page size capped at `MAX_ATTESTATION_READ_PAGE` (20). |

---

## Lifecycle example

```
1. Admin calls init(...)                                    — escrow is initialised
2. Admin calls bind_primary_attestation_hash(hash_a)       — primary hash bound; emits PrimaryAttestationBound
3. Admin calls bind_primary_attestation_hash(hash_b)       — reverts: PrimaryAttestationAlreadyBound (50)
4. Admin calls append_attestation_digest(hash_b)           — index 0 added to log
5. Admin calls append_attestation_digest(hash_c)           — index 1 added to log
6. Admin calls revoke_attestation_digest(0)                — index 0 marked revoked
7. Admin calls revoke_attestation_digest(0)                — reverts: AttestationAlreadyRevoked (53)
8. Admin calls revoke_attestation_digests([1, 99])         — reverts (atomically): AttestationIndexOutOfRange (52) for index 99
9. Admin calls unrevoke_attestation_digest(1)              — reverts: AttestationNotRevoked (56) — index 1 was never revoked
10. Admin calls unrevoke_attestation_digest(0)             — clears revocation on index 0
```

---

## Stability policy

All codes listed here are **append-only and will never be renumbered or reassigned**. New
attestation-related failures will receive new codes at the end of the attestation range (50+). See
[`docs/escrow-error-messages.md`](escrow-error-messages.md) for the full contract-wide code table.

## See also

- [`docs/escrow-error-messages.md`](escrow-error-messages.md) — full error code table
- [`docs/escrow-attestations.md`](escrow-attestations.md) — attestation system overview and design
- [`docs/attestation-invariants.md`](attestation-invariants.md) — invariant specification
