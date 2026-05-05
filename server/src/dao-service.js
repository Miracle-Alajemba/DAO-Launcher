export const PROPOSAL_STATUS = {
  ACTIVE: "Active",
  PASSED: "Passed",
  REJECTED: "Rejected",
  EXECUTED: "Executed",
};

function makeTxHash() {
  return `0x${Math.random().toString(16).slice(2).padEnd(64, "0").slice(0, 64)}`;
}

export class InMemoryDaoService {
  constructor() {
    this.state = {
      dao: null,
      proposals: [],
      events: [],
    };
  }

  now() {
    return new Date().toISOString();
  }

  addEvent(type, payload) {
    this.state.events.push({
      id: this.state.events.length + 1,
      type,
      at: this.now(),
      payload,
    });
  }

  getHealth() {
    return { status: "ok", service: "buildersdao-server", mode: "simulation", at: this.now() };
  }

  createDao({ name = "BuildersDAO", members = ["Alice", "Bob", "Charlie"], totalTokens = 1000 }) {
    const dao = {
      name,
      members,
      totalTokens,
      treasuryBalance: 1200,
      createdAt: this.now(),
    };
    this.state.dao = dao;
    this.state.proposals = [];
    this.state.events = [];
    this.addEvent("DAO_CREATED", { dao });
    return { dao };
  }

  submitProposal({ title = "Fund dev team with 500 POT", amount = 500, recipient = "DevTeamWallet", proposer = "Alice" }) {
    if (!this.state.dao) return { error: "Create DAO first.", status: 400 };
    if (!this.state.dao.members.includes(proposer)) {
      return { error: "Only DAO members can create proposals.", status: 403 };
    }

    const proposal = {
      id: this.state.proposals.length + 1,
      title,
      recipient,
      proposer,
      amount,
      yesVotes: 0,
      noVotes: 0,
      status: PROPOSAL_STATUS.ACTIVE,
      votes: [],
      createdAt: this.now(),
    };
    this.state.proposals.push(proposal);
    this.addEvent("PROPOSAL_CREATED", { proposalId: proposal.id, proposer, recipient, amount, title });
    return { proposal, status: 201 };
  }

  vote({ proposalId, member = "Alice", approve = true }) {
    const proposal = this.state.proposals.find((p) => p.id === Number(proposalId));
    if (!proposal) return { error: "Proposal not found.", status: 404 };
    if (!this.state.dao) return { error: "Create DAO first.", status: 400 };
    if (proposal.status !== PROPOSAL_STATUS.ACTIVE) {
      return { error: "Proposal is no longer active.", status: 409 };
    }
    if (!this.state.dao.members.includes(member)) {
      return { error: "Only DAO members can vote.", status: 403 };
    }
    if (proposal.votes.some((vote) => vote.member === member)) {
      return { error: "Member already voted.", status: 409 };
    }

    proposal.votes.push({ member, approve, at: this.now() });
    if (approve) proposal.yesVotes += 1;
    else proposal.noVotes += 1;

    const threshold = Math.floor(this.state.dao.members.length / 2) + 1;
    if (proposal.yesVotes >= threshold) {
      proposal.status = PROPOSAL_STATUS.PASSED;
    } else if (proposal.noVotes >= threshold) {
      proposal.status = PROPOSAL_STATUS.REJECTED;
    }

    this.addEvent("PROPOSAL_VOTED", {
      proposalId: proposal.id,
      member,
      approve,
      yesVotes: proposal.yesVotes,
      noVotes: proposal.noVotes,
      status: proposal.status,
    });

    return { proposal };
  }

  executeProposal({ proposalId }) {
    const proposal = this.state.proposals.find((p) => p.id === Number(proposalId));
    if (!proposal) return { error: "Proposal not found.", status: 404 };
    if (!this.state.dao) return { error: "Create DAO first.", status: 400 };
    if (proposal.status !== PROPOSAL_STATUS.PASSED) {
      return { error: "Only passed proposals can be executed.", status: 400 };
    }
    if (this.state.dao.treasuryBalance < proposal.amount) {
      return { error: "Insufficient treasury balance.", status: 400 };
    }

    this.state.dao.treasuryBalance -= proposal.amount;
    proposal.status = PROPOSAL_STATUS.EXECUTED;
    proposal.executedAt = this.now();
    proposal.txHash = makeTxHash();

    this.addEvent("PROPOSAL_EXECUTED", {
      proposalId: proposal.id,
      amount: proposal.amount,
      recipient: proposal.recipient,
      txHash: proposal.txHash,
      treasuryBalance: this.state.dao.treasuryBalance,
    });

    return { proposal, dao: this.state.dao };
  }

  getChainState() {
    return {
      dao: this.state.dao,
      proposals: this.state.proposals,
      events: this.state.events,
      immutableNote: "All proposal actions are append-only events in this MVP simulation.",
    };
  }
}

