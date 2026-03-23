# Escrow Error Taxonomy Security Notes

## Scope

This note covers security assumptions and threat scenarios for structured error handling in the escrow contract.

## Security Assumptions

- Callers rely on explicit error semantics for off-chain orchestration.
- Input validation failures should fail fast and fail closed.
- Contract IDs, amounts, ratings, and participant roles are externally supplied and untrusted.

## Threat Scenarios and Controls

1. Silent failure masking with boolean returns.
Control: return `EscrowError` variants for each validation class.

2. Invalid or malformed funding/release requests.
Control: reject zero contract IDs and invalid milestone IDs.

3. Business-rule bypass through malformed values.
Control: enforce positive amounts, bounded rating range, and non-empty milestones.

4. Role confusion in contract setup.
Control: reject creation where client equals freelancer.

## Residual Risk

- Escrow state persistence and authorization logic are still placeholder-level in this module and should be expanded in later hardening work.
