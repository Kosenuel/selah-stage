interface LogoProps {
  className?: string;
}

/** The Selah Stage mark: a fermata, the musical sign for a held pause. */
export function Logo({ className }: LogoProps) {
  return (
    <svg className={className} viewBox="0 0 1024 1024" role="img" aria-label="Selah Stage">
      <rect className="fill-accent-strong" x="64" y="64" width="896" height="896" rx="208" />
      <path
        className="stroke-on-accent"
        d="M 232 640 A 280 280 0 0 1 792 640"
        fill="none"
        strokeWidth="84"
        strokeLinecap="round"
      />
      <circle className="fill-on-accent" cx="512" cy="620" r="70" />
    </svg>
  );
}
