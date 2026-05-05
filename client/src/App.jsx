import React from "react";
import { useEffect, useState } from "react";

const API = import.meta.env.VITE_API_BASE_URL || "http://localhost:4010/api";
const EXPLORER_BASE_URL = import.meta.env.VITE_EXPLORER_BASE_URL || "";

export default function App() {
  const [daoForm, setDaoForm] = useState({
    name: "BuildersDAO",
    member1: "Alice",
    member2: "Bob",
    member3: "Charlie",
    totalTokens: 1000,
  });
  const [proposalForm, setProposalForm] = useState({
    title: "Fund dev team with 500 POT",
    recipient: "DevTeamWallet",
    amount: 500,
    proposer: "Alice",
  });
  const [activeMember, setActiveMember] = useState("Alice");
  const [dao, setDao] = useState(null);
  const [proposals, setProposals] = useState([]);
  const [chainState, setChainState] = useState(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadChainState();
    const timer = setInterval(() => {
      loadChainState(true);
    }, 4000);
    return () => clearInterval(timer);
  }, []);

  async function callApi(url, options = {}, silent = false) {
    if (!silent) {
      setLoading(true);
      setError("");
    }
    try {
      const response = await fetch(url, options);
      const data = await response.json();
      if (!response.ok || data.error) {
        throw new Error(data.error || "Request failed.");
      }
      return data;
    } catch (apiError) {
      if (!silent) {
        setError(apiError.message || "Something went wrong.");
      }
      return null;
    } finally {
      if (!silent) setLoading(false);
    }
  }

  async function createDao(event) {
    event.preventDefault();
    const members = [daoForm.member1, daoForm.member2, daoForm.member3].map((entry) =>
      String(entry || "").trim(),
    );
    if (members.some((entry) => !entry)) {
      setError("Please provide all 3 members.");
      return;
    }

    const data = await callApi(`${API}/dao/create`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: daoForm.name,
        members,
        totalTokens: Number(daoForm.totalTokens),
      }),
    });
    if (!data) return;
    setDao(data.dao);
    setProposals([]);
    setChainState((prev) => ({ ...prev, dao: data.dao, proposals: [] }));
    setMessage(`DAO created: ${data.dao.name} with 3 members.`);
    setProposalForm((prev) => ({ ...prev, proposer: members[0] }));
    setActiveMember(members[0]);
  }

  async function submitProposal(event) {
    event.preventDefault();
    const data = await callApi(`${API}/proposals`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        title: proposalForm.title,
        recipient: proposalForm.recipient,
        amount: Number(proposalForm.amount),
        proposer: proposalForm.proposer,
      }),
    });
    if (!data) return;
    setMessage(`Proposal #${data.proposal.id} created.`);
    await loadChainState(true);
  }

  async function vote(proposalId, support) {
    const data = await callApi(`${API}/proposals/${proposalId}/vote`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ member: activeMember, approve: support }),
    });
    if (!data) return;
    setMessage(`${activeMember} voted ${support ? "YES" : "NO"} on proposal #${proposalId}.`);
    await loadChainState(true);
  }

  async function executeProposal(proposalId, amount) {
    const data = await callApi(`${API}/proposals/${proposalId}/execute`, {
      method: "POST",
    });
    if (!data) return;
    setMessage(`Proposal #${proposalId} executed. Treasury decreased by ${amount} POT.`);
    await loadChainState(true);
  }

  async function loadChainState(silent = false) {
    const data = await callApi(`${API}/chain-state`, {}, silent);
    if (!data) return;
    setChainState(data);
    setDao(data.dao || null);
    setProposals(data.proposals || []);
  }

  const events = chainState?.events || [];
  const memberList = dao?.members || [];
  const tokenPerMember = dao ? Math.floor(Number(dao.totalTokens || 0) / 3) : 0;

  function hasMemberVoted(item) {
    return item.votes?.some((entry) => entry.member === activeMember);
  }

  function getTxLink(txHash) {
    if (!txHash || !EXPLORER_BASE_URL) return null;
    return `${EXPLORER_BASE_URL}${txHash}`;
  }

  return (
    <main className="app">
      <section className="hero">
        <h1>BuildersDAO Launcher</h1>
        <p>{message || "Create DAO, submit proposal, vote, execute, and verify state."}</p>
      </section>

      {error ? (
        <p className="alert error">{error}</p>
      ) : null}
      {loading ? (
        <p className="alert info">Processing transaction...</p>
      ) : null}

      <section className="grid-2">
        <form
          onSubmit={createDao}
          className="card"
        >
          <h3>1. DAO Creation</h3>
          <label>DAO Name</label>
          <input
            value={daoForm.name}
            onChange={(event) => setDaoForm((prev) => ({ ...prev, name: event.target.value }))}
            style={{ width: "100%", marginBottom: 10 }}
          />
          <label>Member 1</label>
          <input
            value={daoForm.member1}
            onChange={(event) => setDaoForm((prev) => ({ ...prev, member1: event.target.value }))}
            style={{ width: "100%", marginBottom: 8 }}
          />
          <label>Member 2</label>
          <input
            value={daoForm.member2}
            onChange={(event) => setDaoForm((prev) => ({ ...prev, member2: event.target.value }))}
            style={{ width: "100%", marginBottom: 8 }}
          />
          <label>Member 3</label>
          <input
            value={daoForm.member3}
            onChange={(event) => setDaoForm((prev) => ({ ...prev, member3: event.target.value }))}
            style={{ width: "100%", marginBottom: 8 }}
          />
          <label>Total Supply (POT)</label>
          <input
            type="number"
            value={daoForm.totalTokens}
            onChange={(event) => setDaoForm((prev) => ({ ...prev, totalTokens: event.target.value }))}
            style={{ width: "100%", marginBottom: 12 }}
          />
          <button type="submit">Create DAO</button>
        </form>

        <form
          onSubmit={submitProposal}
          className="card"
        >
          <h3>2. Proposal Creation</h3>
          <label>Proposer</label>
          <select
            value={proposalForm.proposer}
            onChange={(event) => setProposalForm((prev) => ({ ...prev, proposer: event.target.value }))}
            style={{ width: "100%", marginBottom: 10 }}
            disabled={!dao}
          >
            {memberList.map((member) => (
              <option key={member} value={member}>
                {member}
              </option>
            ))}
          </select>
          <label>Proposal Title</label>
          <input
            value={proposalForm.title}
            onChange={(event) => setProposalForm((prev) => ({ ...prev, title: event.target.value }))}
            style={{ width: "100%", marginBottom: 8 }}
            disabled={!dao}
          />
          <label>Recipient</label>
          <input
            value={proposalForm.recipient}
            onChange={(event) => setProposalForm((prev) => ({ ...prev, recipient: event.target.value }))}
            style={{ width: "100%", marginBottom: 8 }}
            disabled={!dao}
          />
          <label>Amount (POT)</label>
          <input
            type="number"
            value={proposalForm.amount}
            onChange={(event) => setProposalForm((prev) => ({ ...prev, amount: event.target.value }))}
            style={{ width: "100%", marginBottom: 12 }}
            disabled={!dao}
          />
          <button type="submit" disabled={!dao}>
            Submit Proposal
          </button>
        </form>
      </section>

      <section className="card">
        <h3>Dashboard</h3>
        <p>
          <strong>DAO:</strong> {dao?.name || "-"} | <strong>Treasury:</strong> {dao?.treasuryBalance ?? "-"} POT |{" "}
          <strong>Total Supply:</strong> {dao?.totalTokens ?? "-"}
        </p>
        <p>
          <strong>Members:</strong> {memberList.length ? memberList.join(", ") : "-"}
        </p>
        <p>
          <strong>Token Distribution:</strong>{" "}
          {memberList.length ? `${tokenPerMember} POT per member (rounded)` : "-"}
        </p>
      </section>

      <section className="card">
        <h3>3. Proposal List + Voting + Execution</h3>
        <div style={{ marginBottom: 10 }}>
          <label>Active voter:&nbsp;</label>
          <select value={activeMember} onChange={(event) => setActiveMember(event.target.value)} disabled={!dao}>
            {memberList.map((member) => (
              <option key={member} value={member}>
                {member}
              </option>
            ))}
          </select>
        </div>
        {proposals.length === 0 ? (
          <p>No proposals yet.</p>
        ) : (
          proposals.map((item) => {
            const yesPct = Math.min(100, Math.round(((item.yesVotes || 0) / 3) * 100));
            const voted = hasMemberVoted(item);
            const canExecute = item.status === "Passed";
            const statusClass =
              item.status === "Executed"
                ? "executed"
                : item.status === "Passed"
                  ? "passed"
                  : item.status === "Rejected"
                    ? "rejected"
                    : "active";
            return (
              <article
                key={item.id}
                className="card proposal"
                style={{ marginBottom: 10 }}
              >
                <p style={{ margin: "0 0 4px" }}>
                  <strong>#{item.id}</strong> {item.title}
                </p>
                <p style={{ margin: "0 0 4px" }}>
                  Recipient: {item.recipient} | Amount: {item.amount} POT | Status: <span className={`pill ${statusClass}`}>{item.status}</span>
                </p>
                <p style={{ margin: "0 0 8px" }}>
                  Votes: YES {item.yesVotes} / NO {item.noVotes}
                </p>
                <div className="progress">
                  <div style={{ width: `${yesPct}%` }} />
                </div>
                <div className="btn-row">
                  <button
                    type="button"
                    onClick={() => vote(item.id, true)}
                    disabled={item.status !== "Active" || voted}
                  >
                    Vote YES
                  </button>
                  <button
                    type="button"
                    onClick={() => vote(item.id, false)}
                    disabled={item.status !== "Active" || voted}
                  >
                    Vote NO
                  </button>
                  <button
                    type="button"
                    onClick={() => executeProposal(item.id, item.amount)}
                    disabled={!canExecute}
                  >
                    Execute Proposal
                  </button>
                  <button type="button" onClick={() => loadChainState()}>
                    Refresh
                  </button>
                </div>
                {item.txHash ? (
                  <p style={{ marginTop: 8 }}>
                    Tx Hash: {item.txHash}
                    {getTxLink(item.txHash) ? (
                      <>
                        {" "}
                        |{" "}
                        <a href={getTxLink(item.txHash)} target="_blank" rel="noreferrer">
                          View on explorer
                        </a>
                      </>
                    ) : null}
                  </p>
                ) : null}
              </article>
            );
          })
        )}
      </section>

      <section className="card">
        <h3 style={{ marginBottom: 8 }}>4. Event Timeline</h3>
        <div className="events">
          {events.length === 0 ? (
            <p className="muted" style={{ margin: 0 }}>No events yet. Run actions to populate logs.</p>
          ) : (
            events
              .slice()
              .reverse()
              .map((event) => (
                <div key={event.id} className="event-item">
                  <strong>{event.type}</strong> · {event.at}
                </div>
              ))
          )}
        </div>
      </section>

      <pre style={{ background: "#f6f6f6", padding: 16, borderRadius: 8, overflow: "auto" }}>
{JSON.stringify({ dao, proposals, chainState }, null, 2)}
      </pre>
    </main>
  );
}
