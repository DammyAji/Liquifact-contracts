# SME Collateral Commitment Metadata

`record_sme_collateral_commitment(asset, amount)` in [`escrow/src/lib.rs`](../escrow/src/lib.rs) is a **metadata-only** Soroban escrow entrypoint. It allows the configured SME address to report collateral metadata for off-chain risk review, but it does **not** move, reserve, escrow, freeze, or verify any asset on-chain.

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

### `clear_sme_collateral_commitment(env)`

Retires a previously recorded commitment, removing it from storage.

## Test Coverage

Collateral scenarios are exercised in:
- [`escrow/src/tests/admin.rs`](../escrow/src/tests/admin.rs) — collateral record in admin-flow baselines.
- [`escrow/src/tests/integration.rs`](../escrow/src/tests/integration.rs) — event-payload verification for record/clear.

---

## Off-chain Risk-Team Handling

Risk teams and indexers must treat `SmeCollateralCommitment` and `CollateralRecordedEvt` as
**reported metadata only**. They are not proof of custody, lien, encumbrance, or asset
control, and do not alter funding, settlement, withdrawal, investor-claim, compliance-hold,
or treasury-sweep behavior. Supporting evidence must be verified off-chain.

---

### Recommended verification procedures

1. **Verify Signer Context:** Confirm the transaction was signed by the correct SME address linked to the invoice (`InvoiceEscrow::sme_address`).
2. **Resolve Asset Symbol:** Ensure the reported `asset` symbol maps to the correct physical asset or token contract. The contract performs no on-chain validation of the symbol.
3. **Verify Custody Separately:** Confirm custody accounts, statements, and security perfection outside the blockchain. The escrow contract makes no assertion about the SME's ownership.
4. **Reconcile Independently:** Implement any asset-control or settlement actions in separate off-chain systems or dedicated contracts, completely detached from this metadata escrow record.
5. **Clear Labeling:** Label all indexed database fields as `reported_collateral_metadata` rather than implying locked balances or enforceable claims. Never use `collateral_locked`, `collateral_pledged`, or similar language that implies on-chain enforcement.
6. **Monitor Replacements:** Track `prior_amount` → `amount` transitions in `CollateralRecordedEvt` events to detect collateral amount changes over time.
7. **Track Clears:** Monitor `CollateralClearedEvt` events to detect when the SME retires a commitment.

### Event indexing guidance

When indexing `CollateralRecordedEvt` events:

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

When indexing `CollateralClearedEvt` / `CollateralCommitmentCleared` events:

```text
Topic 0 (fixed): collateral_cleared_evt / collateral_commitment_cleared
Topic 1:         name = "coll_clr"
Data:
  invoice_id: Symbol     — join key for escrow state
  asset:      Symbol     — asset symbol from the cleared commitment
  amount:     i128       — amount from the cleared commitment
  recorded_at: u64       — original ledger timestamp from the cleared commitment
```

| Code | Variant                        | Trigger                                                |
|------|--------------------------------|---------------------------------------------------------|
| 60   | `CollateralAmountNotPositive`  | `record_sme_collateral_commitment` with `amount <= 0`    |
| 61   | `CollateralAssetEmpty`         | `record_sme_collateral_commitment` with empty asset symbol |
| 62   | `CollateralTimestampBackwards` | Replacement timestamp precedes the stored `recorded_at`  |
| 63   | `NoCollateralToClear`          | `clear_sme_collateral_commitment` with no commitment recorded |

---

## Error Codes

- **Metadata-only**: neither `record_sme_collateral_commitment` nor
  `clear_sme_collateral_commitment` transfers or locks tokens.
- **SME-only writes**: all mutating operations require `sme_address.require_auth()`.
- **No status dependency**: collateral metadata can be recorded or cleared regardless of
  escrow status (open / funded / settled), allowing clean-up after settlement or cancellation.
- **No double-clear risk**: the existence check on entry ensures a second clear call
  returns `NoCollateralToClear` rather than silently succeeding.

---

## Example Flow

```
SME calls record_sme_collateral_commitment("USDC", 5_000_0000000)
  → DataKey::SmeCollateralPledge stored as SmeCollateralCommitment
  → CollateralRecordedEvt { amount: 5_000_0000000, prior_amount: 0 } emitted

[invoice settled off-chain; commitment released]

SME calls clear_sme_collateral_commitment()
  → DataKey::SmeCollateralPledge removed
  → CollateralClearedEvt { name: "coll_clr", invoice_id: "INV001", asset: "GOLD", amount: 7000, recorded_at: <ts2> }
  → CollateralCommitmentCleared emitted with same payload
```

---

## Cross-references

- **Rustdoc:** See [`LiquifactEscrow::record_sme_collateral_commitment`] and [`LiquifactEscrow::clear_sme_collateral_commitment`] in [`escrow/src/lib.rs`](../escrow/src/lib.rs).
- **Struct definition:** [`SmeCollateralCommitment`] struct with `asset`, `amount`, and `recorded_at` fields.
- **Event schema:** [`CollateralRecordedEvt`], [`CollateralClearedEvt`], and [`CollateralCommitmentCleared`] in [`docs/EVENT_SCHEMA.md`](EVENT_SCHEMA.md).
- **Error codes:** Codes 60–62 and `NoCollateralToClear` in [`docs/escrow-error-messages.md`](escrow-error-messages.md).
- **Storage model:** [`DataKey::SmeCollateralPledge`] in [`docs/escrow-data-model.md`](escrow-data-model.md).
- **Read API:** [`get_sme_collateral_commitment()`] in [`docs/escrow-read-api.md`](escrow-read-api.md).
- **Security checklist:** Section 5.8 in [`docs/escrow-security-checklist.md`](escrow-security-checklist.md).
- **Audit handoff:** Section 6.3 in [`docs/audit-handoff-escrow.md`](audit-handoff-escrow.md).
- **Indexer guidance:** [`docs/escrow-indexer.md`](escrow-indexer.md).
- **CLI simulation:** [`docs/escrow-sim-stellar-cli.md`](escrow-sim-stellar-cli.md).
- **Glossary:** [`docs/glossary.md`](glossary.md) — definitions for `SmeCollateralCommitment` and related terms.

---

## Security Notes

- The contract writes/removes metadata only — there is **no token-transfer code path** reachable from these entrypoints.
- The `recorded_at` monotonicity check ensures replacement timestamps do not regress, providing defense-in-depth against stale replay attacks.
- The `asset` symbol is stored as an arbitrary `Symbol` with no on-chain resolution. Integrators must map it to real-world assets off-chain.
- A compromised SME key could write arbitrary collateral amounts or clear records, but since the record is metadata-only and does not gate any contract flow, the blast radius is limited to off-chain reporting inaccuracies.
- These records must **never** be used as the sole input for automated liquidation, margin calls, or asset-freeze decisions.
- **No double-clear risk:** the existence check in `clear_sme_collateral_commitment` ensures a second clear call returns `NoCollateralToClear` rather than silently succeeding.
