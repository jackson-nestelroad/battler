export interface InfoIconProps {
  size?: number;
  className?: string;
}

export default function InfoIcon({ size = 12, className }: InfoIconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="currentColor"
      aria-hidden="true"
      className={className}
    >
      <path d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1Zm0 2.5a1.1 1.1 0 1 1 0 2.2 1.1 1.1 0 0 1 0-2.2ZM6.75 7.5h1.75v4.25h1.25v1H6.25v-1h1.25V8.5H6.75v-1Z" />
    </svg>
  );
}
