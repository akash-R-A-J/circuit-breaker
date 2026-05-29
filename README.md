# Pinocchio Reserve Circuit Breaker

This is a small Pinocchio Solana program that ports the shape of a complex lending reserve into a focused safety exercise. I used Kamino's open-source KLend as the source-program pattern: a reserve accepts liquidity deposits, tracks borrowed liquidity, and has a privileged authority that can update reserve risk configuration.

The implemented instructions are:

- `InitializeReserve`: creates the reserve state and circuit-breaker limits.
- `DepositLiquidity`: transfers SPL tokens from a user token account into the reserve vault, then records available reserve liquidity after passing a per-slot deposit cap.
- `BorrowLiquidity`: transfers SPL tokens from the reserve vault to a borrower token account and records outstanding borrow after passing borrow-volume and utilization caps.
- `SetCircuitBreaker`: authority-only update for caps, manual halt, deposit disable, borrow disable, and timed halt.

## Why These Instructions

Lending programs are sensitive at exactly these transitions:

- Deposit affects accounting inputs such as total assets and utilization. Large same-slot deposits can distort later risk calculations.
- Borrow is the dangerous path because it removes liquid assets and increases protocol exposure.

The circuit breaker sits before the SPL Token CPI in both paths. If a breaker check fails, the instruction exits before moving tokens or changing reserve balances. If the token transfer fails, Solana rolls the whole instruction back.

## State Layout

The reserve account is a fixed 112-byte account with explicit little-endian fields:

- discriminator and version
- authority address
- available liquidity and borrowed amount
- per-slot deposit/borrow windows
- max deposit per slot
- max borrow per slot
- max utilization in basis points
- halt-until slot
- bit flags for manual halt, deposits disabled, and borrows disabled

No unsafe pointer casts are used for account data. The codec reads and writes explicit byte ranges, which avoids alignment bugs and type-cosplay surprises.

## Circuit Breaker Rules

Deposits are rejected when:

- the reserve is manually halted
- the current slot is before `halt_until_slot`
- deposits are disabled
- the same-slot deposit window would exceed `max_deposit_per_slot`

Borrows are rejected when:

- the reserve is manually halted
- the current slot is before `halt_until_slot`
- borrows are disabled
- the same-slot borrow window would exceed `max_borrow_per_slot`
- the post-borrow utilization would exceed `max_utilization_bps`
- available liquidity is insufficient

Per-slot counters reset when the Clock sysvar slot advances. The program uses `Clock::get()`, so users cannot spoof the slot through instruction data. Token movement uses SPL Token `TransferChecked`, so the mint decimals supplied by the caller are verified by the token program during CPI.

## Account Order

`InitializeReserve`

1. reserve account, writable, owned by this program
2. authority signer

`DepositLiquidity`

1. reserve account, writable, owned by this program
2. depositor signer
3. depositor source token account, writable
4. reserve vault token account, writable
5. mint

`BorrowLiquidity`

1. reserve account, writable, owned by this program
2. borrower signer
3. reserve vault token account, writable
4. borrower destination token account, writable
5. mint
6. vault authority signer, must match reserve authority

`SetCircuitBreaker`

1. reserve account, writable, owned by this program
2. authority signer, must match reserve authority

## Build And Test

```bash
cargo test
cargo build
cargo build-sbf --features bpf-entrypoint
```

## Source References

- Kamino KLend: https://github.com/Kamino-Finance/klend
- Pinocchio: https://github.com/anza-xyz/pinocchio
