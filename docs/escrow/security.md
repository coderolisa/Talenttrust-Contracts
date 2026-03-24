# Escrow Security Notes

This document summarizes security assumptions and threat scenarios for access-control enforcement.

## Security Controls

- Explicit role checks on all mutating methods.
- Mandatory auth calls (`require_auth`) for role-bearing callers.
- Role-aware release gating based on `ReleaseAuthorization`.
- Strict state transition validation (`Created` -> `Funded` -> `Completed`).
- Defensive checks for invalid milestone IDs and duplicate actions.
- Checked arithmetic for milestone total and reputation accumulation.

## Threat Scenarios and Mitigations

- Unauthorized deposit, approval, release, or reputation issuance:
  - Mitigated by caller-role matching against contract state.
- Freelancer impersonation in reputation flow:
  - Mitigated by explicit freelancer-address equality check with contract record.
- Arbiter misuse or ambiguous arbiter identity:
  - Mitigated by arbiter distinctness checks and required-arbiter mode validation.
- Replay or duplicate approvals/releases:
  - Mitigated by per-milestone approval flags and release-state guard.
- Invalid state progression:
  - Mitigated by explicit status guards before each mutation.

## Residual Assumptions

- Fund transfers are represented logically in state; token transfer integration is out of scope.
- Dispute workflow (`Disputed`) remains reserved for future implementation.
- On-chain fee/resource precision should be validated via network simulation tools before production.
