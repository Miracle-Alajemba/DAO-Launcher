# BuildersDAO

BuildersDAO is a Portaldot-focused DAO MVP for hackathon demos.

## Demo Flow

1. Create DAO `BuildersDAO`
2. Mint governance tokens (1000 total, 3 members)
3. Submit proposal: `Fund dev team with 500 POT`
4. Cast votes (2 of 3 YES)
5. Mark proposal passed
6. Execute treasury payment through multisig-style approvals
7. Query chain state (proposals, votes, balances)

## Workspace Layout

- `contracts/ink/` Ink! contract for DAO + treasury + voting
- `server/` API layer for reads/writes and chain-state aggregation
- `client/` Frontend for demo actions and live state views

## Current Status

- Scaffold initialized
- Next: implement contract storage/messages and local dev test flow
