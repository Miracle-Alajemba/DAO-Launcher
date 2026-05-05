import cors from "cors";
import express from "express";

const app = express();
const port = Number(process.env.PORT || 4010);

app.use(cors());
app.use(express.json());

const state = {
  dao: null,
  proposals: [],
};

const PROPOSAL_STATUS = {
  ACTIVE: "Active",
  PASSED: "Passed",
  REJECTED: "Rejected",
  EXECUTED: "Executed",
};

app.get("/api/health", (_req, res) => {
  res.json({ status: "ok", service: "buildersdao-server", at: new Date().toISOString() });
});

app.post("/api/dao/create", (req, res) => {
  const { name = "BuildersDAO", members = ["Alice", "Bob", "Charlie"], totalTokens = 1000 } = req.body || {};
  state.dao = {
    name,
    members,
    totalTokens,
    treasuryBalance: 1200,
    createdAt: new Date().toISOString(),
  };
  res.status(201).json({ dao: state.dao });
});

app.post("/api/proposals", (req, res) => {
  if (!state.dao) return res.status(400).json({ error: "Create DAO first." });
  const { title = "Fund dev team with 500 POT", amount = 500, recipient = "DevTeamWallet" } = req.body || {};
  const proposer = req.body?.proposer || "Alice";
  if (!state.dao.members.includes(proposer)) {
    return res.status(403).json({ error: "Only DAO members can create proposals." });
  }

  const proposal = {
    id: state.proposals.length + 1,
    title,
    recipient,
    proposer,
    amount,
    yesVotes: 0,
    noVotes: 0,
    status: PROPOSAL_STATUS.ACTIVE,
    votes: [],
    createdAt: new Date().toISOString(),
  };
  state.proposals.push(proposal);
  res.status(201).json({ proposal });
});

app.post("/api/proposals/:id/vote", (req, res) => {
  const proposal = state.proposals.find((p) => p.id === Number(req.params.id));
  if (!proposal) return res.status(404).json({ error: "Proposal not found." });
  if (!state.dao) return res.status(400).json({ error: "Create DAO first." });
  if (proposal.status !== PROPOSAL_STATUS.ACTIVE) {
    return res.status(409).json({ error: "Proposal is no longer active." });
  }

  const { member = "Alice", approve = true } = req.body || {};
  if (!state.dao.members.includes(member)) {
    return res.status(403).json({ error: "Only DAO members can vote." });
  }
  if (proposal.votes.some((vote) => vote.member === member)) {
    return res.status(409).json({ error: "Member already voted." });
  }

  proposal.votes.push({ member, approve, at: new Date().toISOString() });
  if (approve) proposal.yesVotes += 1;
  else proposal.noVotes += 1;

  const threshold = Math.floor(state.dao.members.length / 2) + 1;
  if (proposal.yesVotes >= threshold) {
    proposal.status = PROPOSAL_STATUS.PASSED;
  } else if (proposal.noVotes >= threshold) {
    proposal.status = PROPOSAL_STATUS.REJECTED;
  }

  res.json({ proposal });
});

app.post("/api/proposals/:id/execute", (req, res) => {
  const proposal = state.proposals.find((p) => p.id === Number(req.params.id));
  if (!proposal) return res.status(404).json({ error: "Proposal not found." });
  if (!state.dao) return res.status(400).json({ error: "Create DAO first." });
  if (proposal.status !== PROPOSAL_STATUS.PASSED) {
    return res.status(400).json({ error: "Only passed proposals can be executed." });
  }
  if (state.dao.treasuryBalance < proposal.amount) {
    return res.status(400).json({ error: "Insufficient treasury balance." });
  }

  state.dao.treasuryBalance -= proposal.amount;
  proposal.status = PROPOSAL_STATUS.EXECUTED;
  proposal.executedAt = new Date().toISOString();
  proposal.txHash = `0x${Math.random().toString(16).slice(2).padEnd(64, "0").slice(0, 64)}`;

  res.json({ proposal, dao: state.dao });
});

app.get("/api/chain-state", (_req, res) => {
  res.json({
    dao: state.dao,
    proposals: state.proposals,
    immutableNote: "All proposal actions are append-only events in this MVP simulation.",
  });
});

app.listen(port, () => {
  console.log(`BuildersDAO server listening on http://localhost:${port}`);
});
