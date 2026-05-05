import cors from "cors";
import express from "express";
import { InMemoryDaoService } from "./dao-service.js";

const app = express();
const port = Number(process.env.PORT || 4010);
const daoService = new InMemoryDaoService();

app.use(cors());
app.use(express.json());

app.get("/api/health", (_req, res) => {
  res.json(daoService.getHealth());
});

app.post("/api/dao/create", (req, res) => {
  const result = daoService.createDao(req.body || {});
  res.status(201).json(result);
});

app.post("/api/proposals", (req, res) => {
  const result = daoService.submitProposal(req.body || {});
  if (result.error) {
    return res.status(result.status || 400).json({ error: result.error });
  }
  res.status(result.status || 201).json({ proposal: result.proposal });
});

app.post("/api/proposals/:id/vote", (req, res) => {
  const result = daoService.vote({
    proposalId: req.params.id,
    member: req.body?.member,
    approve: req.body?.approve,
  });
  if (result.error) {
    return res.status(result.status || 400).json({ error: result.error });
  }
  res.json({ proposal: result.proposal });
});

app.post("/api/proposals/:id/execute", (req, res) => {
  const result = daoService.executeProposal({ proposalId: req.params.id });
  if (result.error) {
    return res.status(result.status || 400).json({ error: result.error });
  }
  res.json({ proposal: result.proposal, dao: result.dao });
});

app.get("/api/chain-state", (_req, res) => {
  res.json(daoService.getChainState());
});

app.listen(port, () => {
  console.log(`BuildersDAO server listening on http://localhost:${port}`);
});
