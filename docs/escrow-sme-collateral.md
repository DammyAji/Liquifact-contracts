# Escrow — SME Collateral Commitment

## Overview

The LiquiFact escrow contract supports **metadata-only** collateral commitment recording.
No tokens are moved, reserved, or locked by these operations. The stored
`SmeCollateralCommitment` and emitted `CollateralRecordedEvt` are **not proof of custody**,
lien, encumbrance, or asset control — they exist solely for indexers and off-chain risk
teams to surface reported collateral intent alongside an invoice's on-chain state. Risk
teams must verify supporting evidence outside this contract.

---

## Entrypoints

### `record_sme_collateral_commitment(env, asset: Symbol, amount: i128) -> SmeCollateralCommitment`

Records (or replaces) an off-chain collateral commitment against the escrow's invoice.

- **Auth**: SME address (`sme_address` from the escrow record).
- **Storage**: writes `DataKey::SmeCollateralPledge` (instance storage) as an
  `SmeCollateralCommitment { asset, amount, recorded_at }`.
- **Event**: emits `CollateralRecordedEvt { name: "coll_rec", invoice_id, amount, prior_amount }`.
- **Idempotency**: calling again replaces the previous commitment; `prior_amount` on the
  event carries the amount that was overwritten (`0` on the first call).
- **Ordering guard**: a replacement's ledger timestamp must be `>=` the prior commitment's
  `recorded_at`, or the call is rejected with `CollateralTimestampBackwards`.
- **Token movement**: none.

### `get_sme_collateral_commitment(env) -> Option<SmeCollateralCommitment>`

Returns the current commitment, or `None` if none has been recorded (or it was cleared).

- **Auth**: none required (read-only).

### `clear_sme_collateral_commitment(env) -> Result<(), EscrowError>`

Retires a previously recorded commitment, removing it from storage.

## Test Coverage

Collateral scenarios are exercised in:
- [`escrow/src/tests/admin.rs`](../escrow/src/tests/admin.rs) — collateral record in admin-flow baselines.
- [`escrow/src/tests/integration.rs`](../escrow/src/tests/integration.rs) — event-payload verification for record/clear.

## Off-chain Risk-Team Handling

Risk teams and indexers must treat `SmeCollateralCommitment` and `CollateralRecordedEvt` as
**reported metadata only**. They are not proof of custody, lien, encumbrance, or asset
control, and do not alter funding, settlement, withdrawal, investor-claim, compliance-hold,
or treasury-sweep behavior. Supporting evidence must be verified off-chain.

---

## Guard ordering (ADR-002)

`clear_sme_collateral_commitment` applies guards in this order to keep auth
checks from masking informative errors:

1. **Read-only existence check** — return `NoCollateralToClear` immediately if
   `DataKey::SmeCollateralPledge` is absent (no auth consumed).
2. **`require_auth`** — assert the caller is the SME address.
3. **Mutation** — remove the storage entry and emit `CollateralClearedEvt`.

---

## Data types

```rust
pub struct SmeCollateralCommitment {
    pub asset: Symbol,
    pub amount: i128,
    pub recorded_at: u64,
}

pub struct CollateralRecordedEvt {
    pub name: Symbol,       // "coll_rec"
    pub invoice_id: Symbol,
    pub amount: i128,
    pub prior_amount: i128, // 0 on the first record
}

pub struct CollateralClearedEvt {
    pub invoice_id: Symbol,
    pub amount: i128,   // carried from the commitment at the time of removal
}
```

---

## Error codes

| Code | Variant                        | Trigger                                                |
|------|--------------------------------|---------------------------------------------------------|
| 60   | `CollateralAmountNotPositive`  | `record_sme_collateral_commitment` with `amount <= 0`    |
| 61   | `CollateralAssetEmpty`         | `record_sme_collateral_commitment` with empty asset symbol |
| 62   | `CollateralTimestampBackwards` | Replacement timestamp precedes the stored `recorded_at`  |
| 63   | `NoCollateralToClear`          | `clear_sme_collateral_commitment` with no commitment recorded |

---

## Security notes

- **Metadata-only**: neither `record_sme_collateral_commitment` nor
  `clear_sme_collateral_commitment` transfers or locks tokens.
- **SME-only writes**: all mutating operations require `sme_address.require_auth()`.
- **No status dependency**: collateral metadata can be recorded or cleared regardless of
  escrow status (open / funded / settled), allowing clean-up after settlement or cancellation.
- **No double-clear risk**: the existence check on entry ensures a second clear call
  returns `NoCollateralToClear` rather than silently succeeding.

---

## Example flow

```
SME calls record_sme_collateral_commitment("USDC", 5_000_0000000)
  → DataKey::SmeCollateralPledge stored as SmeCollateralCommitment
  → CollateralRecordedEvt { amount: 5_000_0000000, prior_amount: 0 } emitted

[invoice settled off-chain; commitment released]

SME calls clear_sme_collateral_commitment()
  → DataKey::SmeCollateralPledge removed
  → CollateralClearedEvt { invoice_id: "INV001", amount: 5_000_0000000 } emitted
```
