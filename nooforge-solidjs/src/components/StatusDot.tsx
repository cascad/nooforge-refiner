// src/components/StatusDot.tsx
import { Component, Show } from 'solid-js';
import type { Status } from '../types/ui';

interface StatusDotProps {
  status: Status;
  title?: string;
}

const StatusDot: Component<StatusDotProps> = (props) => {
  const colorClass = () => {
    switch (props.status) {
      case 'loading':
        return 'bg-yellow-400 animate-pulse';
      case 'ok':
        return 'bg-emerald-500';
      case 'error':
        return 'bg-rose-500';
      default:
        return 'bg-neutral-500';
    }
  };

  return (
    <span
      class="inline-flex items-center gap-2"
      title={props.title || props.status}
    >
      <span class={`inline-block h-2.5 w-2.5 rounded-full ${colorClass()}`} />
      <Show when={props.title}>
        <span class="text-xs text-neutral-400">{props.title}</span>
      </Show>
    </span>
  );
};

export default StatusDot;
