import React from "react";
import { useState } from "react";

const API = import.meta.env.VITE_API_BASE_URL || "http://localhost:4010/api";

export default function App() {
  const [dao, setDao] = useState(null);
  const [proposal, setProposal] = useState(null);
  const [chainState, setChainState] = useState(null);
  const [message, setMessage] = useState("");

  async function createDao() {
    const response = await fetch(`${API}/dao/create`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: "BuildersDAO",
        members: ["Alice", "Bob", "Charlie"],
        totalTokens: 1000,
      }),
    });
    const data = await response.json();
    if (data.error) {
      setMessage(data.error);
      return;
    }
    setDao(data.dao);
    setMessage("DAO created: BuildersDAO (1000 governance tokens, 3 members).");
  }

  async function submitProposal() {
    const response = await fetch(`${API}/proposals`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        title: "Fund dev team with 500 POT",
        recipient: "DevTeamWallet",
        amount: 500,
        proposer: "Alice",
      }),
    });
    const data = await response.json();
    if (data.error) {
      setMessage(data.error);
      return;
    }
    setProposal(data.proposal);
    setMessage("Proposal submitted.");
  }

  async function voteYes(member) {
    if (!proposal) return;
    const response = await fetch(`${API}/proposals/${proposal.id}/vote`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ member, approve: true }),
    });
    const data = await response.json();
    if (data.error) {
      setMessage(data.error);
      return;
    }
    setProposal(data.proposal);
    setMessage(`${member} voted YES.`);
  }

  async function executeProposal() {
    if (!proposal) return;
    const response = await fetch(`${API}/proposals/${proposal.id}/execute`, {
      method: "POST",
    });
    const data = await response.json();
    if (data.error) {
      setMessage(data.error);
      return;
    }
    setProposal(data.proposal);
    setDao(data.dao);
    setMessage("Treasury payment executed on-chain (demo tx hash generated).");
  }

  async function loadChainState() {
    const response = await fetch(`${API}/chain-state`);
    const data = await response.json();
    setChainState(data);
    setMessage("Chain state loaded.");
  }

  const events = chainState?.events || [];
  const statusTone =
    proposal?.status === "Executed"
      ? "#0f766e"
      : proposal?.status === "Passed"
        ? "#1d4ed8"
        : proposal?.status === "Rejected"
          ? "#b91c1c"
          : "#475569";

  return (
    <main
      style={{
        fontFamily: "system-ui, sans-serif",
        padding: 24,
        maxWidth: 900,
        margin: "0 auto",
        minHeight: "100vh",
        background: "#ffffff",
        color: "#111111",
      }}
    >
      <h1>BuildersDAO Demo</h1>
      <p>{message || "Run the 4-step flow below."}</p>

      <section style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 20 }}>
        <button onClick={createDao}>1. Create BuildersDAO</button>
        <button onClick={submitProposal} disabled={!dao}>2. Submit Proposal</button>
        <button onClick={() => voteYes("Alice")} disabled={!proposal}>Vote YES (Alice)</button>
        <button onClick={() => voteYes("Bob")} disabled={!proposal}>Vote YES (Bob)</button>
        <button onClick={executeProposal} disabled={!proposal}>3. Execute Treasury Payment</button>
        <button onClick={loadChainState}>4. Check Chain State</button>
      </section>

      {proposal ? (
        <section
          style={{
            border: `1px solid ${statusTone}`,
            borderRadius: 10,
            padding: 12,
            marginBottom: 12,
            background: "#fafafa",
          }}
        >
          <p style={{ margin: 0 }}>
            Proposal status: <strong style={{ color: statusTone }}>{proposal.status}</strong>
          </p>
          <p style={{ margin: "6px 0 0" }}>
            {proposal.title} → {proposal.recipient} ({proposal.amount} POT)
          </p>
        </section>
      ) : null}

      <section style={{ marginBottom: 12 }}>
        <h3 style={{ marginBottom: 8 }}>Onchain Event Timeline (MVP Simulation)</h3>
        <div style={{ background: "#f8fafc", borderRadius: 8, padding: 12 }}>
          {events.length === 0 ? (
            <p style={{ margin: 0, color: "#64748b" }}>No events yet. Run the flow to generate immutable history.</p>
          ) : (
            events
              .slice()
              .reverse()
              .map((event) => (
                <div
                  key={event.id}
                  style={{
                    borderBottom: "1px solid #e2e8f0",
                    padding: "8px 0",
                    fontSize: 14,
                  }}
                >
                  <strong>{event.type}</strong> · <span>{event.at}</span>
                </div>
              ))
          )}
        </div>
      </section>

      <pre style={{ background: "#f6f6f6", padding: 16, borderRadius: 8, overflow: "auto" }}>
{JSON.stringify({ dao, proposal, chainState }, null, 2)}
      </pre>
    </main>
  );
}
