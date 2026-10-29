import { useEffect, useRef, useState } from 'react';
import type { AgentConfig, AgentThread, MessageAttachment } from '../../../shared/types';
import { AgentThreadList } from './AgentThreadList';
import { AgentConversation } from './AgentConversation';

interface AgentPanelProps {
  onImageClick: (attachment: MessageAttachment) => void;
}

export function AgentPanel({ onImageClick }: AgentPanelProps) {
  const [agents, setAgents] = useState<AgentConfig[]>([]);
  const [threads, setThreads] = useState<AgentThread[]>([]);
  const [selectedThreadId, setSelectedThreadId] = useState<string | null>(null);
  // Set right before CREATE_AGENT_THREAD is sent - the next AGENT_THREADS
  // broadcast that lands is guaranteed to have the new thread at index 0
  // (getThreads() sorts by updatedAt desc, and nothing else touches
  // updatedAt concurrently), so this is how the freshly created thread
  // gets auto-selected without main having to echo back its id directly.
  const awaitingNewThread = useRef(false);

  useEffect(() => {
    const unsubs = [
      window.charlotteChat.onAgentsUpdated(setAgents),
      window.charlotteChat.onAgentThreads((list) => {
        setThreads(list);
        if (awaitingNewThread.current && list.length > 0) {
          awaitingNewThread.current = false;
          setSelectedThreadId(list[0].id);
        }
      }),
    ];
    window.charlotteChat.requestAgents();
    window.charlotteChat.requestAgentThreads();
    return () => unsubs.forEach((unsub) => unsub());
  }, []);

  const selectedThread = threads.find((t) => t.id === selectedThreadId) ?? null;

  if (selectedThread) {
    const agent = agents.find((a) => a.id === selectedThread.agentId) ?? null;
    return (
      <AgentConversation
        key={selectedThread.id}
        thread={selectedThread}
        agent={agent}
        onBack={() => setSelectedThreadId(null)}
        onDelete={() => {
          window.charlotteChat.deleteAgentThread(selectedThread.id);
          setSelectedThreadId(null);
        }}
        onImageClick={onImageClick}
      />
    );
  }

  return (
    <AgentThreadList
      agents={agents}
      threads={threads}
      onSelectThread={setSelectedThreadId}
      onCreateThread={(agentId) => {
        awaitingNewThread.current = true;
        window.charlotteChat.createAgentThread(agentId);
      }}
      onDeleteThread={(id) => window.charlotteChat.deleteAgentThread(id)}
    />
  );
}
