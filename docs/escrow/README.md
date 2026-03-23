# Escrow Contract Docs

## Purpose

This contract currently provides a baseline escrow API and demonstrates structured error handling with an explicit error taxonomy.

## Error-First Convention

Mutating or validating endpoints now return `Result<_, EscrowError>` instead of plain booleans.

Benefits:

- Deterministic failure semantics for integrators.
- Better auditability and incident triage.
- More precise tests for expected failure modes.

## Public Error Codes

- `1` `InvalidContractId`
- `2` `InvalidMilestoneId`
- `3` `InvalidAmount`
- `4` `InvalidRating`
- `5` `EmptyMilestones`
- `6` `InvalidParticipant`

## Validation Rules

- Contract id must be non-zero for operational calls.
- Amounts must be strictly positive.
- Rating must be in `1..=5`.
- Milestone list cannot be empty.
- Client and freelancer must be distinct addresses.
- `u32::MAX` milestone id is reserved as invalid in this placeholder implementation.
