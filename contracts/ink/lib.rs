#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod builders_dao {
    use ink::prelude::{string::String, vec::Vec};
    use ink::storage::Mapping;

    pub type ProposalId = u64;

    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Proposal {
        title: String,
        amount: Balance,
        yes_votes: u32,
        no_votes: u32,
        passed: bool,
        executed: bool,
    }

    #[ink(storage)]
    pub struct BuildersDao {
        owner: AccountId,
        dao_name: String,
        total_supply: Balance,
        treasury_balance: Balance,
        member_count: u32,
        proposals_count: ProposalId,
        members: Mapping<AccountId, bool>,
        balances: Mapping<AccountId, Balance>,
        proposals: Mapping<ProposalId, Proposal>,
        voted: Mapping<(ProposalId, AccountId), bool>,
    }

    impl BuildersDao {
        #[ink(constructor)]
        pub fn new(
            dao_name: String,
            members: Vec<AccountId>,
            mint_per_member: Balance,
            initial_treasury: Balance,
        ) -> Self {
            let caller = Self::env().caller();
            let mut instance = Self {
                owner: caller,
                dao_name,
                total_supply: 0,
                treasury_balance: initial_treasury,
                member_count: 0,
                proposals_count: 0,
                members: Mapping::default(),
                balances: Mapping::default(),
                proposals: Mapping::default(),
                voted: Mapping::default(),
            };

            for account in members {
                if !instance.members.get(account).unwrap_or(false) {
                    instance.members.insert(account, &true);
                    instance.member_count = instance.member_count.saturating_add(1);
                    instance.balances.insert(account, &mint_per_member);
                    instance.total_supply = instance.total_supply.saturating_add(mint_per_member);
                }
            }

            instance
        }

        #[ink(message)]
        pub fn submit_proposal(&mut self, title: String, amount: Balance) -> ProposalId {
            self.proposals_count = self.proposals_count.saturating_add(1);
            let proposal_id = self.proposals_count;
            self.proposals.insert(
                proposal_id,
                &Proposal {
                    title,
                    amount,
                    yes_votes: 0,
                    no_votes: 0,
                    passed: false,
                    executed: false,
                },
            );
            proposal_id
        }

        #[ink(message)]
        pub fn vote(&mut self, proposal_id: ProposalId, approve: bool) {
            let voter = self.env().caller();
            if !self.members.get(voter).unwrap_or(false) {
                return;
            }
            if self.voted.get((proposal_id, voter)).unwrap_or(false) {
                return;
            }

            if let Some(mut proposal) = self.proposals.get(proposal_id) {
                if proposal.executed {
                    return;
                }
                if approve {
                    proposal.yes_votes = proposal.yes_votes.saturating_add(1);
                } else {
                    proposal.no_votes = proposal.no_votes.saturating_add(1);
                }
                // Simple majority rule for MVP
                proposal.passed = proposal.yes_votes > self.member_count / 2;
                self.proposals.insert(proposal_id, &proposal);
                self.voted.insert((proposal_id, voter), &true);
            }
        }

        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: ProposalId) -> bool {
            if let Some(mut proposal) = self.proposals.get(proposal_id) {
                if proposal.executed || !proposal.passed {
                    return false;
                }
                if self.treasury_balance < proposal.amount {
                    return false;
                }
                self.treasury_balance = self.treasury_balance.saturating_sub(proposal.amount);
                proposal.executed = true;
                self.proposals.insert(proposal_id, &proposal);
                return true;
            }
            false
        }

        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: ProposalId) -> Option<Proposal> {
            self.proposals.get(proposal_id)
        }

        #[ink(message)]
        pub fn get_chain_state(&self) -> (String, Balance, u32, ProposalId) {
            (
                self.dao_name.clone(),
                self.treasury_balance,
                self.member_count,
                self.proposals_count,
            )
        }
    }
}
