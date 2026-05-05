#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod builders_dao {
    use ink::prelude::{string::String, vec::Vec};
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct BuildersDAO {
        name: String,
        members: Vec<AccountId>,
        treasury_balance: Balance,
        governance_tokens: Mapping<AccountId, Balance>,
        total_supply: Balance,
        proposals: Mapping<u32, Proposal>,
        next_proposal_id: u32,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum ProposalStatus {
        Active,
        Executed,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Error {
        NotMember,
        ProposalNotFound,
        InsufficientTreasuryFunds,
        AlreadyVoted,
        ThresholdNotMet,
        ProposalNotActive,
        ProposalAlreadyExecuted,
        InvalidAmount,
        EmptyTitle,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Proposal {
        id: u32,
        title: String,
        recipient: AccountId,
        amount: Balance,
        yes_votes: u32,
        no_votes: u32,
        status: ProposalStatus,
        voters: Vec<AccountId>,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct DAOState {
        pub name: String,
        pub members_count: u32,
        pub treasury_balance: Balance,
        pub total_supply: Balance,
        pub next_proposal_id: u32,
    }

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ProposalView {
        pub id: u32,
        pub title: String,
        pub recipient: AccountId,
        pub amount: Balance,
        pub yes_votes: u32,
        pub no_votes: u32,
        pub status: ProposalStatus,
        pub voters_count: u32,
    }

    #[ink(event)]
    pub struct DAOCreated {
        #[ink(topic)]
        name: String,
        members_count: u32,
        treasury_balance: Balance,
    }

    #[ink(event)]
    pub struct ProposalCreated {
        #[ink(topic)]
        proposal_id: u32,
        title: String,
        amount: Balance,
        #[ink(topic)]
        recipient: AccountId,
    }

    #[ink(event)]
    pub struct VoteCast {
        #[ink(topic)]
        proposal_id: u32,
        #[ink(topic)]
        voter: AccountId,
        support: bool,
    }

    #[ink(event)]
    pub struct ProposalExecuted {
        #[ink(topic)]
        proposal_id: u32,
        #[ink(topic)]
        recipient: AccountId,
        amount: Balance,
        treasury_remaining: Balance,
    }

    #[ink(event)]
    pub struct ErrorEvent {
        reason: String,
    }

    impl BuildersDAO {
        /// Initializes a new DAO instance with exactly three members.
        /// Caller must transfer `total_supply` native POT to prefund treasury.
        #[ink(constructor, payable)]
        pub fn init(
            name: String,
            members: [AccountId; 3],
            total_supply: Balance,
        ) -> Result<Self, Error> {
            if total_supply == 0 {
                Self::emit_error_raw(String::from("InvalidAmount: total_supply must be > 0"));
                return Err(Error::InvalidAmount);
            }

            let transferred = Self::env().transferred_value();
            if transferred != total_supply {
                Self::emit_error_raw(String::from(
                    "InvalidAmount: transferred POT must equal total_supply",
                ));
                return Err(Error::InvalidAmount);
            }

            let members_vec = members.to_vec();
            let mint_per_member = total_supply / 3;
            let mut contract = Self {
                name: name.clone(),
                members: members_vec.clone(),
                treasury_balance: total_supply,
                governance_tokens: Mapping::default(),
                total_supply,
                proposals: Mapping::default(),
                next_proposal_id: 0,
            };

            for member in members_vec.iter() {
                contract.governance_tokens.insert(member, &mint_per_member);
            }

            contract.env().emit_event(DAOCreated {
                name,
                members_count: 3,
                treasury_balance: total_supply,
            });

            Ok(contract)
        }

        /// Creates a new proposal by a member.
        #[ink(message)]
        pub fn create_proposal(
            &mut self,
            title: String,
            recipient: AccountId,
            amount: Balance,
        ) -> Result<u32, Error> {
            let caller = self.env().caller();
            if !self.is_member(caller) {
                self.emit_error(String::from("NotMember: caller is not a DAO member"));
                return Err(Error::NotMember);
            }
            if title.trim().is_empty() {
                self.emit_error(String::from("EmptyTitle: proposal title cannot be empty"));
                return Err(Error::EmptyTitle);
            }
            if amount == 0 {
                self.emit_error(String::from("InvalidAmount: proposal amount must be > 0"));
                return Err(Error::InvalidAmount);
            }
            if amount > self.treasury_balance {
                self.emit_error(String::from(
                    "InsufficientTreasuryFunds: amount exceeds treasury",
                ));
                return Err(Error::InsufficientTreasuryFunds);
            }

            let proposal_id = self.next_proposal_id;
            self.next_proposal_id = self.next_proposal_id.saturating_add(1);

            let proposal = Proposal {
                id: proposal_id,
                title: title.clone(),
                recipient,
                amount,
                yes_votes: 0,
                no_votes: 0,
                status: ProposalStatus::Active,
                voters: Vec::new(),
            };
            self.proposals.insert(proposal_id, &proposal);

            self.env().emit_event(ProposalCreated {
                proposal_id,
                title,
                amount,
                recipient,
            });

            Ok(proposal_id)
        }

        /// Casts a yes/no vote for an active proposal. One vote per member.
        #[ink(message)]
        pub fn vote(&mut self, proposal_id: u32, support: bool) -> Result<(), Error> {
            let caller = self.env().caller();
            if !self.is_member(caller) {
                self.emit_error(String::from("NotMember: caller is not a DAO member"));
                return Err(Error::NotMember);
            }

            let Some(mut proposal) = self.proposals.get(proposal_id) else {
                self.emit_error(String::from("ProposalNotFound: invalid proposal_id"));
                return Err(Error::ProposalNotFound);
            };

            if proposal.status != ProposalStatus::Active {
                self.emit_error(String::from("ProposalNotActive: proposal is not active"));
                return Err(Error::ProposalNotActive);
            }

            if proposal.voters.contains(&caller) {
                self.emit_error(String::from("AlreadyVoted: member already voted"));
                return Err(Error::AlreadyVoted);
            }

            if support {
                proposal.yes_votes = proposal.yes_votes.saturating_add(1);
            } else {
                proposal.no_votes = proposal.no_votes.saturating_add(1);
            }
            proposal.voters.push(caller);
            self.proposals.insert(proposal_id, &proposal);

            self.env().emit_event(VoteCast {
                proposal_id,
                voter: caller,
                support,
            });

            Ok(())
        }

        /// Executes a passed proposal using 2-of-3 threshold.
        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: u32) -> Result<(), Error> {
            let Some(mut proposal) = self.proposals.get(proposal_id) else {
                self.emit_error(String::from("ProposalNotFound: invalid proposal_id"));
                return Err(Error::ProposalNotFound);
            };

            if proposal.status == ProposalStatus::Executed {
                self.emit_error(String::from("ProposalAlreadyExecuted: already executed"));
                return Err(Error::ProposalAlreadyExecuted);
            }

            if proposal.status != ProposalStatus::Active {
                self.emit_error(String::from("ProposalNotActive: proposal is not active"));
                return Err(Error::ProposalNotActive);
            }

            if proposal.yes_votes < 2 {
                self.emit_error(String::from("ThresholdNotMet: requires at least 2 yes votes"));
                return Err(Error::ThresholdNotMet);
            }

            if proposal.amount > self.treasury_balance {
                self.emit_error(String::from(
                    "InsufficientTreasuryFunds: treasury balance too low",
                ));
                return Err(Error::InsufficientTreasuryFunds);
            }

            if self.env().transfer(proposal.recipient, proposal.amount).is_err() {
                self.emit_error(String::from(
                    "InsufficientTreasuryFunds: native transfer failed",
                ));
                return Err(Error::InsufficientTreasuryFunds);
            }

            self.treasury_balance = self.treasury_balance.saturating_sub(proposal.amount);
            proposal.status = ProposalStatus::Executed;
            self.proposals.insert(proposal_id, &proposal);

            self.env().emit_event(ProposalExecuted {
                proposal_id,
                recipient: proposal.recipient,
                amount: proposal.amount,
                treasury_remaining: self.treasury_balance,
            });

            Ok(())
        }

        /// Returns current DAO state.
        #[ink(message)]
        pub fn get_dao_state(&self) -> DAOState {
            DAOState {
                name: self.name.clone(),
                members_count: u32::try_from(self.members.len()).unwrap_or(u32::MAX),
                treasury_balance: self.treasury_balance,
                total_supply: self.total_supply,
                next_proposal_id: self.next_proposal_id,
            }
        }

        /// Returns a single proposal view by id.
        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: u32) -> Option<ProposalView> {
            self.proposals
                .get(proposal_id)
                .map(|proposal| Self::to_proposal_view(&proposal))
        }

        /// Lists all proposals in insertion order.
        #[ink(message)]
        pub fn list_proposals(&self) -> Vec<ProposalView> {
            let mut list = Vec::new();
            let mut id = 0;
            while id < self.next_proposal_id {
                if let Some(proposal) = self.proposals.get(id) {
                    list.push(Self::to_proposal_view(&proposal));
                }
                id = id.saturating_add(1);
            }
            list
        }

        /// Returns governance token balance for a member address.
        #[ink(message)]
        pub fn get_member_token_balance(&self, member: AccountId) -> Balance {
            self.governance_tokens.get(member).unwrap_or(0)
        }

        /// Returns true if the account is a registered member.
        #[ink(message)]
        pub fn is_member(&self, account: AccountId) -> bool {
            self.members.contains(&account)
        }

        fn to_proposal_view(proposal: &Proposal) -> ProposalView {
            ProposalView {
                id: proposal.id,
                title: proposal.title.clone(),
                recipient: proposal.recipient,
                amount: proposal.amount,
                yes_votes: proposal.yes_votes,
                no_votes: proposal.no_votes,
                status: proposal.status.clone(),
                voters_count: u32::try_from(proposal.voters.len()).unwrap_or(u32::MAX),
            }
        }

        fn emit_error(&self, reason: String) {
            self.env().emit_event(ErrorEvent { reason });
        }

        fn emit_error_raw(reason: String) {
            Self::env().emit_event(ErrorEvent { reason });
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::env::test;

        fn accounts() -> test::DefaultAccounts<ink::env::DefaultEnvironment> {
            test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn init_contract(total_supply: Balance) -> BuildersDAO {
            let accts = accounts();
            test::set_value_transferred::<ink::env::DefaultEnvironment>(total_supply);
            BuildersDAO::init(
                String::from("BuildersDAO"),
                [accts.alice, accts.bob, accts.charlie],
                total_supply,
            )
            .expect("init should succeed")
        }

        #[ink::test]
        fn test_1_init_creates_dao_and_tokens() {
            let contract = init_contract(999);
            let accts = accounts();
            let state = contract.get_dao_state();
            assert_eq!(state.name, "BuildersDAO");
            assert_eq!(state.members_count, 3);
            assert_eq!(state.treasury_balance, 999);
            assert_eq!(contract.get_member_token_balance(accts.alice), 333);
            assert_eq!(contract.get_member_token_balance(accts.bob), 333);
            assert_eq!(contract.get_member_token_balance(accts.charlie), 333);
        }

        #[ink::test]
        fn test_2_create_proposal_member_allowed() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("Fund dev team with 500 POT"), accts.django, 500)
                .expect("proposal should be created");
            assert_eq!(id, 0);
        }

        #[ink::test]
        fn test_3_create_proposal_reject_non_member() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.eve);
            let result =
                contract.create_proposal(String::from("Bad caller"), accts.django, 100);
            assert_eq!(result, Err(Error::NotMember));
        }

        #[ink::test]
        fn test_4_create_proposal_reject_if_amount_gt_treasury() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let result = contract.create_proposal(String::from("Too much"), accts.django, 2000);
            assert_eq!(result, Err(Error::InsufficientTreasuryFunds));
        }

        #[ink::test]
        fn test_5_vote_only_members() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("Vote check"), accts.django, 100)
                .unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(accts.eve);
            let result = contract.vote(id, true);
            assert_eq!(result, Err(Error::NotMember));
        }

        #[ink::test]
        fn test_6_vote_prevents_double_voting() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("Double vote"), accts.django, 100)
                .unwrap();

            assert_eq!(contract.vote(id, true), Ok(()));
            assert_eq!(contract.vote(id, true), Err(Error::AlreadyVoted));
        }

        #[ink::test]
        fn test_7_vote_increments_yes_votes() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("Yes vote"), accts.django, 100)
                .unwrap();
            contract.vote(id, true).unwrap();
            let proposal = contract.get_proposal(id).unwrap();
            assert_eq!(proposal.yes_votes, 1);
        }

        #[ink::test]
        fn test_8_vote_increments_no_votes() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("No vote"), accts.django, 100)
                .unwrap();
            contract.vote(id, false).unwrap();
            let proposal = contract.get_proposal(id).unwrap();
            assert_eq!(proposal.no_votes, 1);
        }

        #[ink::test]
        fn test_9_execute_requires_two_yes_votes() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("Threshold"), accts.django, 100)
                .unwrap();
            contract.vote(id, true).unwrap();
            let result = contract.execute_proposal(id);
            assert_eq!(result, Err(Error::ThresholdNotMet));
        }

        #[ink::test]
        fn test_10_execute_transfers_funds() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_account_balance::<ink::env::DefaultEnvironment>(
                test::callee::<ink::env::DefaultEnvironment>(),
                2_000_000,
            );
            let before = test::get_account_balance::<ink::env::DefaultEnvironment>(accts.django)
                .expect("should read");

            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("Transfer"), accts.django, 500)
                .unwrap();
            contract.vote(id, true).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.bob);
            contract.vote(id, true).unwrap();
            contract.execute_proposal(id).unwrap();

            let after = test::get_account_balance::<ink::env::DefaultEnvironment>(accts.django)
                .expect("should read");
            assert!(after >= before + 500);
        }

        #[ink::test]
        fn test_11_execute_decrements_treasury() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_account_balance::<ink::env::DefaultEnvironment>(
                test::callee::<ink::env::DefaultEnvironment>(),
                2_000_000,
            );
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("Treasury down"), accts.django, 500)
                .unwrap();
            contract.vote(id, true).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.bob);
            contract.vote(id, true).unwrap();
            contract.execute_proposal(id).unwrap();
            assert_eq!(contract.get_dao_state().treasury_balance, 500);
        }

        #[ink::test]
        fn test_12_execute_cannot_execute_twice() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_account_balance::<ink::env::DefaultEnvironment>(
                test::callee::<ink::env::DefaultEnvironment>(),
                2_000_000,
            );
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("No double execute"), accts.django, 500)
                .unwrap();
            contract.vote(id, true).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.bob);
            contract.vote(id, true).unwrap();
            contract.execute_proposal(id).unwrap();
            let second = contract.execute_proposal(id);
            assert_eq!(second, Err(Error::ProposalAlreadyExecuted));
        }

        #[ink::test]
        fn test_13_execute_rejects_if_amount_gt_treasury() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_account_balance::<ink::env::DefaultEnvironment>(
                test::callee::<ink::env::DefaultEnvironment>(),
                2_000_000,
            );
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("Drains treasury"), accts.django, 900)
                .unwrap();
            contract.vote(id, true).unwrap();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.bob);
            contract.vote(id, true).unwrap();
            contract.execute_proposal(id).unwrap();

            test::set_caller::<ink::env::DefaultEnvironment>(accts.charlie);
            let id2 = contract
                .create_proposal(String::from("Too high now"), accts.django, 500)
                .unwrap_err();
            assert_eq!(id2, Error::InsufficientTreasuryFunds);
        }

        #[ink::test]
        fn test_14_get_dao_state_correct() {
            let contract = init_contract(1000);
            let state = contract.get_dao_state();
            assert_eq!(state.name, "BuildersDAO");
            assert_eq!(state.members_count, 3);
            assert_eq!(state.treasury_balance, 1000);
            assert_eq!(state.total_supply, 1000);
            assert_eq!(state.next_proposal_id, 0);
        }

        #[ink::test]
        fn test_15_get_proposal_correct() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            let id = contract
                .create_proposal(String::from("Get proposal"), accts.django, 100)
                .unwrap();
            let p = contract.get_proposal(id).unwrap();
            assert_eq!(p.id, 0);
            assert_eq!(p.title, "Get proposal");
            assert_eq!(p.amount, 100);
            assert_eq!(p.status, ProposalStatus::Active);
        }

        #[ink::test]
        fn test_16_list_proposals_shows_all() {
            let mut contract = init_contract(1000);
            let accts = accounts();
            test::set_caller::<ink::env::DefaultEnvironment>(accts.alice);
            contract
                .create_proposal(String::from("P1"), accts.django, 100)
                .unwrap();
            contract
                .create_proposal(String::from("P2"), accts.eve, 200)
                .unwrap();
            let all = contract.list_proposals();
            assert_eq!(all.len(), 2);
            assert_eq!(all[0].title, "P1");
            assert_eq!(all[1].title, "P2");
        }

        #[ink::test]
        fn test_17_is_member_works() {
            let contract = init_contract(1000);
            let accts = accounts();
            assert!(contract.is_member(accts.alice));
            assert!(!contract.is_member(accts.eve));
        }
    }
}
