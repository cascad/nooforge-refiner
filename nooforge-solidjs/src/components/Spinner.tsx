// src/components/Spinner.tsx
import { Component } from 'solid-js';

interface SpinnerProps {
  size?: number;
}

const Spinner: Component<SpinnerProps> = (props) => {
  const size = () => props.size || 16;
  
  return (
    <svg
      width={size()}
      height={size()}
      viewBox="0 0 24 24"
      class="animate-spin inline-block"
      style={{ "vertical-align": "-2px" }}
    >
      <circle
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        stroke-width="3"
        fill="none"
        opacity="0.2"
      />
      <path
        d="M22 12a10 10 0 0 0-10-10"
        stroke="currentColor"
        stroke-width="3"
        fill="none"
      />
    </svg>
  );
};

export default Spinner;
