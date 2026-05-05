#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod builders_dao {
    use ink::prelude::{string::String, vec::Vec};
    use ink::storage::Mapping;

    pub type ProposalId = u64;

    #[derive(scale::Decode, scale::Encode, Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ProposalStatus {
        Active,
        Passed,
        Rejected,
        Executed,
    }

    #[derive(scale::Decode, scale::Encode, Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotMember,
        ProposalNotFound,
        AlreadyVoted,
        ProposalClosed,
        InsufficientTreasury,
        NotPassed,
        AlreadyExecuted,
    }

    #[derive(scale::Decode, scale::Encode, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Proposal {
        id: ProposalId,
        title: String,
        recipient: AccountId,
        amount: Balance,
        yes_votes: u32,
        no_votes: u32,
        status: ProposalStatus,
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
        pub fn submit_proposal(
            &mut self,
            title: String,
            recipient: AccountId,
            amount: Balance,
        ) -> Result<ProposalId, Error> {
            let caller = self.env().caller();
            if !self.members.get(caller).unwrap_or(false) {
                return Err(Error::NotMember);
            }

            self.proposals_count = self.proposals_count.saturating_add(1);
            let proposal_id = self.proposals_count;
            self.proposals.insert(
                proposal_id,
                &Proposal {
                    id: proposal_id,
                    title,
                    recipient,
                    amount,
                    yes_votes: 0,
                    no_votes: 0,
                    status: ProposalStatus::Active,
                },
            );
            Ok(proposal_id)
        }

        #[ink(message)]
        pub fn vote(&mut self, proposal_id: ProposalId, approve: bool) -> Result<(), Error> {
            let voter = self.env().caller();
            if !self.members.get(voter).unwrap_or(false) {
                return Err(Error::NotMember);
            }
            if self.voted.get((proposal_id, voter)).unwrap_or(false) {
                return Err(Error::AlreadyVoted);
            }

            if let Some(mut proposal) = self.proposals.get(proposal_id) {
                if proposal.status != ProposalStatus::Active {
                    return Err(Error::ProposalClosed);
                }
                if approve {
                    proposal.yes_votes = proposal.yes_votes.saturating_add(1);
                } else {
                    proposal.no_votes = proposal.no_votes.saturating_add(1);
                }

                // 2-of-3 style majority rule (generalized as > member_count/2)
                if proposal.yes_votes > self.member_count / 2 {
                    proposal.status = ProposalStatus::Passed;
                } else if proposal.no_votes >= self.member_count / 2 + 1 {
                    proposal.status = ProposalStatus::Rejected;
                }

                self.proposals.insert(proposal_id, &proposal);
                self.voted.insert((proposal_id, voter), &true);
                return Ok(());
            }
            Err(Error::ProposalNotFound)
        }

        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: ProposalId) -> Result<(), Error> {
            if let Some(mut proposal) = self.proposals.get(proposal_id) {
                if proposal.status == ProposalStatus::Executed {
                    return Err(Error::AlreadyExecuted);
                }
                if proposal.status != ProposalStatus::Passed {
                    return Err(Error::NotPassed);
                }
                if self.treasury_balance < proposal.amount {
                    return Err(Error::InsufficientTreasury);
                }
                self.treasury_balance = self.treasury_balance.saturating_sub(proposal.amount);
                // For MVP this marks on-chain execution state and updates treasury ledger.
                // A transfer call can be plugged in when we wire runtime token transfer.
                proposal.status = ProposalStatus::Executed;
                self.proposals.insert(proposal_id, &proposal);
                return Ok(());
            }
            Err(Error::ProposalNotFound)
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

        #[ink(message)]
        pub fn list_proposals(&self) -> Vec<Proposal> {
            let mut out = Vec::new();
            let mut id = 1;
            while id <= self.proposals_count {
                if let Some(p) = self.proposals.get(id) {
                    out.push(p);
                }
                id += 1;
            }
            out
        }

        #[ink(message)]
        pub fn is_member(&self, account: AccountId) -> bool {
            self.members.get(account).unwrap_or(false)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        fn setup() -> BuildersDao {
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            BuildersDao::new(
                String::from("BuildersDAO"),
                vec![accounts.alice, accounts.bob, accounts.charlie],
                333,
                1_000,
            )
        }

        #[ink::test]
        fn creates_dao_with_members() {
            let dao = setup();
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();
            assert!(dao.is_member(accounts.alice));
            assert!(dao.is_member(accounts.bob));
            assert!(dao.is_member(accounts.charlie));
            assert_eq!(dao.get_chain_state().2, 3);
        }

        #[ink::test]
        fn proposal_pass_and_execute() {
            let mut dao = setup();
            let accounts = test::default_accounts::<ink::env::DefaultEnvironment>();

            test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            let pid = dao
                .submit_proposal(String::from("Fund dev team with 500 POT"), accounts.django, 500)
                .expect("submit should work");

            dao.vote(pid, true).expect("alice vote yes");
            test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            dao.vote(pid, true).expect("bob vote yes");

            let proposal = dao.get_proposal(pid).expect("proposal exists");
            assert_eq!(proposal.status, ProposalStatus::Passed);

            dao.execute_proposal(pid).expect("execute should work");
            let proposal_after = dao.get_proposal(pid).expect("proposal exists");
            assert_eq!(proposal_after.status, ProposalStatus::Executed);
            assert_eq!(dao.get_chain_state().1, 500);
        }
    }
}
