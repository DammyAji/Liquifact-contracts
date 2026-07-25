# Escrow — SME Collateral Commitment

## Overview

The LiquiFact escrow contract supports **metadata-only** collateral pledge recording.
No tokens are moved or reserved by these operations; they exist solely for indexers
and dashboards to surface off-chain pledge intent alongside an invoice's on-chain state.

---

## Entrypoints

### `record_sme_collateral_commitment(env, asset, amount) -> SmeCollateralCommitment`

Records an off-chain collateral pledge against the escrow's invoice.

- **Auth**: SME address (`sme_address` from the escrow record).
- **Storage**: writes `DataKey::SmeCollateralPledge` (instance storage).
- **Event**: emits `CollateralRecordedEvt { name, invoice_id, amount, prior_amount }`
  under topic `(coll_rec, …)`.
- **Idempotency**: calling again overwrites the previous amount (monotonic `recorded_at`).
- **Token movement**: none.

### `get_sme_collateral_commitment(env) -> Option<SmeCollateralCommitment>`

Returns the current pledge record, or `None` if none has been recorded (or it was cleared).

- **Auth**: none required (read-only).

### `clear_sme_collateral_commitment(env) -> ()`

Retires a previously recorded pledge, removing it from storage.

- **Auth**: SME address (`sme_address` from the escrow record).
- **Storage**: removes `DataKey::SmeCollateralPledge` (instance storage).
- **Event**: emits exactly one `CollateralClearedEvt { name, invoice_id, asset, amount, recorded_at }`
  under topic `(coll_clr, invoice_id)`.
- **Error**: returns typed `NoCollateralToClear` (code `169`) when no pledge exists.
- **Token movement**: none.

---

## Guard ordering (ADR-002)

`clear_sme_collateral_commitment` applies guards in this order to keep auth
checks from masking informative errors:

1. **Read-only existence check** — return `NoCollateralToClear` immediately if
   `DataKey::SmeCollateralPledge` is absent (no auth consumed).
2. **`require_auth`** — assert the caller is the SME address via `load_escrow_require_sme`.
3. **Mutation** — remove the storage entry and emit a single `CollateralClearedEvt`.

---

## Data types

```rust
pub struct SmeCollateralCommitment {
    pub asset: Symbol,
    pub amount: i128,
    pub recorded_at: u64,
}

pub struct CollateralRecordedEvt {
    pub name: Symbol,       // hardcoded coll_rec topic
    pub invoice_id: Symbol,
    pub amount: i128,
    pub prior_amount: i128, // 0 on first record
}

pub struct CollateralClearedEvt {
    pub name: Symbol,       // hardcoded coll_clr topic
    pub invoice_id: Symbol,
    pub asset: Symbol,      // carried from the pledge at the time of removal
    pub amount: i128,       // carried from the pledge at the time of removal
    pub recorded_at: u64,   // original pledge ledger timestamp
}
```

---

## Error codes

| Code | Variant               | Trigger                                          |
|------|-----------------------|--------------------------------------------------|
| 169  | `NoCollateralToClear` | `clear_sme_collateral_commitment` with no pledge |

Related record-path errors (unchanged numbering): `CollateralAmountNotPositive`,
`CollateralAssetEmpty`, `CollateralTimestampBackwards`.

---

## Test Coverage

The scenarios below are covered by the focused collateral suite in
[`escrow/src/tests/coverage.rs`](../escrow/src/tests/coverage.rs):

| Test | Scenario |
|------|----------|
| `test_clear_without_record_rejected` | Clear with no prior commitment → `NoCollateralToClear`. |
| `test_record_then_clear_removes_commitment` | Record then clear; getter returns `None`. |
| `test_double_clear_rejected` | Second clear after a successful clear → `NoCollateralToClear`. |
| `test_clear_emits_exactly_one_coll_clr_event` | Exactly one `CollateralClearedEvt` with expected payload. |
| `test_clear_non_sme_caller_rejected` | Non-SME / missing auth rejected; storage unchanged. |
| `test_clear_after_settle_succeeds` | Clear still works after settlement. |
| `test_clear_after_cancel_funding_succeeds` | Clear still works after funding cancellation. |
| `test_overwrite_then_clear` | Replace then clear removes the latest pledge. |
| `test_collateral_first_record_returns_correct_fields_and_prior_amount_is_zero` | First record returns the correct asset/amount/timestamp. |
| `test_collateral_non_sme_caller_rejected` | Non-SME record caller rejected. |
| `test_collateral_record_does_not_change_token_balances` | No token balances change on record. |

Additional collateral scenarios are also exercised in:
- [`escrow/src/tests/admin.rs`](../escrow/src/tests/admin.rs) — collateral record in admin-flow baselines.
- [`escrow/src/tests/integration.rs`](../escrow/src/tests/integration.rs) — record event payload verification.

---

## Security notes

- **Metadata-only**: neither `record_sme_collateral_commitment` nor
  `clear_sme_collateral_commitment` transfers or locks tokens. This is
  **not proof of custody** — the contract does not verify off-chain asset control.
- **SME-only writes**: all mutating operations require `sme_address.require_auth()`.
- **No status dependency**: collateral metadata can be cleared regardless of escrow
  status (open / funded / settled / cancelled), allowing clean-up after settlement or cancellation.
- **No double-clear risk**: the existence check on entry ensures a second clear call
  returns `NoCollateralToClear` rather than silently succeeding.
- **Single retirement event**: each successful clear publishes exactly one `coll_clr`
  (`CollateralClearedEvt`) so indexers do not see duplicated clear notifications.

---

## Example flow

```
SME calls record_sme_collateral_commitment(USDC, 5_000_0000000)
  → DataKey::SmeCollateralPledge stored
  → CollateralRecordedEvt emitted

[invoice settled or cancelled; off-chain pledge released]

SME calls clear_sme_collateral_commitment()
  → DataKey::SmeCollateralPledge removed
  → CollateralClearedEvt { name: "coll_clr", invoice_id: "INV001", asset: "USDC", amount: 5_000_0000000, recorded_at } emitted
```
