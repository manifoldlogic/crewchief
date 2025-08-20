import React from "react";
import { useQuery, useMutation } from "@apollo/client";
import { GET_AGENTS, SPAWN_AGENT } from "../queries/agentQueries";

const AgentOrchestration = () => {
  const { data } = useQuery(GET_AGENTS);
  const [spawnAgent] = useMutation(SPAWN_AGENT);

  // Spawn dialog, grid, message, monitoring

  return <div>Agent Orchestration</div>;
};

export default AgentOrchestration;
