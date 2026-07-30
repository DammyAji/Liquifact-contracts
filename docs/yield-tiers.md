# Yield Tier Selection and Rounding Specification

This document details how investor contribution amounts map to yield tiers in the Liquifact Escrow contract (`contracts/escrow/src/lib.rs`). It defines the tier table structure, the tier lookup algorithm, boundary rules, and payout rounding behavior.

---

## 1. Tier Table Structure

The yield tier table is configured during contract initialization (`init`) or table configuration entrypoints and stored as `DataKey::YieldTierTable`.

Each entry in the table follows the `YieldTier` structure (defined in `contracts/escrow/src/types.rs`):

| Field | Type | Description |
| :--- | :--- | :--- |
| `min_amount` | `i128` | Minimum contribution amount required to qualify for this tier (in token base units). |
| `yield_bps` | `u32` | Yield rate expressed in Basis Points ($1\text{ BPS} = 0.01\% = 0.0001$). |
| `committed_lock_secs` | `u64` | Minimum lock duration associated with this tier (if applicable). |

### Configuration Invariants (`validate_yield_tiers_table`)

When the tier table is initialized, `validate_yield_tiers_table` enforces the following rules:
1. **Monotonicity**: Tiers must be sorted in strictly ascending order by `min_amount`.
2. **Non-Decreasing Yield**: Tiers with higher `min_amount` must have equal or higher `yield_bps`.
3. **Non-Negative Amounts**: `min_amount` for any tier must be greater than or equal to `0`.

---

## 2. Selection Algorithm and Entrypoints

### Cross-Referenced Contract Entrypoints (`contracts/escrow/src/lib.rs`)

* `init(env, ...)`: Accepts and validates the `YieldTierTable` configuration via `validate_yield_tiers_table`.
* `fund_with_commitment(env, investor, amount, lock_secs)`: Reads `YieldTierTable` and calls `effective_yield_for_commitment` to evaluate the contribution against configured tiers.
* `get_yield_tiers(env)`: Read-only view returning the active ordered tier list.

### Lookup Logic (`effective_yield_for_commitment`)

When a user contributes an `amount`, the contract iterates through the sorted `YieldTierTable` to find the highest qualifying tier:

```rust
// Logical equivalent of effective_yield_for_commitment tier evaluation
pub fn select_yield_tier(tiers: &[YieldTier], amount: i128) -> Option<YieldTier> {
    tiers.iter()
         .filter(|tier| amount >= tier.min_amount)
         .last()
         .cloned()
}
