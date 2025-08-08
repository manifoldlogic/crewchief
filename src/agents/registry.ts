import { AgentType } from './types';

const builtin: Record<string, AgentType> = {
  'project-manager': {
    id: 'project-manager',
    name: 'Project Manager',
    platform: 'claude',
    capabilities: ['planning', 'delegation', 'review'],
    agentDefinitionPath: '.claude/agents/project-manager.md',
    executionCommand: 'claude-cli'
  },
  'backend-developer': {
    id: 'backend-developer',
    name: 'Backend Developer',
    platform: 'claude',
    capabilities: ['api', 'database', 'testing'],
    agentDefinitionPath: '.claude/agents/backend-developer.md',
    executionCommand: 'claude-cli'
  },
  'frontend-developer': {
    id: 'frontend-developer',
    name: 'Frontend Developer',
    platform: 'gemini',
    capabilities: ['ui', 'components', 'styling'],
    agentDefinitionPath: '.gemini/agents/frontend-developer.txt',
    executionCommand: 'gemini-cli'
  }
};

export function getAgentType(id: string): AgentType | undefined {
  return builtin[id];
}


