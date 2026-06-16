import type { ComponentType, SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const base: IconProps = {
  width: 18,
  height: 18,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 2,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  "aria-hidden": true,
};

export function NavIconHome({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={["nav-icon--nudge-1", className].filter(Boolean).join(" ")}>
      <path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
      <polyline points="9 22 9 12 15 12 15 22" />
    </svg>
  );
}

export function NavIconBoard({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={["nav-icon--nudge-1", className].filter(Boolean).join(" ")}>
      <rect x="4" y="4" width="16" height="16" rx="3" />
      <circle cx="12" cy="12" r="2.25" />
    </svg>
  );
}

/** 汇总 / 全部资产 */
export function NavIconLayers({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={["nav-icon--nudge-1", className].filter(Boolean).join(" ")}>
      <polygon points="12 2 2 7 12 12 22 7 12 2" />
      <polyline points="2 17 12 22 22 17" />
      <polyline points="2 12 12 17 22 12" />
    </svg>
  );
}

export function NavIconPrompt({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={["nav-icon--nudge-1", className].filter(Boolean).join(" ")}>
      <path d="M4 5h16v12H7l-3 3V5z" />
      <path d="M8 9h8M8 13h5" />
    </svg>
  );
}

export function NavIconBot(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M12 8V4H8" />
      <rect width="16" height="12" x="4" y="8" rx="2" />
      <path d="M2 14h2" />
      <path d="M20 14h2" />
      <path d="M15 13v2" />
      <path d="M9 13v2" />
    </svg>
  );
}

export function NavIconCursor({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={className}>
      <path d="M12 3 20.5 8v8L12 21l-8.5-5V8L12 3z" />
      <path d="M4.75 7.9h14.5L12 20v-8.25L4.75 7.9z" />
    </svg>
  );
}

export function NavIconClaude({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={className}>
      <path d="M12 3.25v5.5" />
      <path d="M12 15.25v5.5" />
      <path d="M3.25 12h5.5" />
      <path d="M15.25 12h5.5" />
      <path d="m5.8 5.8 3.9 3.9" />
      <path d="m14.3 14.3 3.9 3.9" />
      <path d="m18.2 5.8-3.9 3.9" />
      <path d="m9.7 14.3-3.9 3.9" />
      <circle cx="12" cy="12" r="1.25" />
    </svg>
  );
}

export function NavIconCodex({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={className}>
      <rect x="4" y="4" width="16" height="16" rx="4" />
      <path d="m10 9-3 3 3 3" />
      <path d="M14 15h3" />
      <path d="M12.75 7.5c2.45.35 4.25 2.2 4.25 4.5" />
      <path d="M11.25 16.5C8.8 16.15 7 14.3 7 12" />
    </svg>
  );
}

export function NavIconHermes({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={className}>
      <path d="M8 5v14" />
      <path d="M16 5v14" />
      <path d="M8 12h8" />
      <path d="M6.5 8.25 3.5 6.5" />
      <path d="M6.5 10.75 3 10" />
      <path d="m17.5 8.25 3-1.75" />
      <path d="m17.5 10.75 3.5-.75" />
    </svg>
  );
}

export function NavIconOpenClaw({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={className}>
      <path d="M6.25 11.5C6.25 7.9 8.65 5 12 5s5.75 2.9 5.75 6.5c0 4.1-2.85 7.5-5.75 7.5s-5.75-3.4-5.75-7.5z" />
      <path d="M7.75 5.75 5.5 3.5" />
      <path d="m16.25 5.75 2.25-2.25" />
      <path d="M6.2 12.25c-2.35-.55-3.65.65-2.7 2.55.6 1.2 1.75 1.5 3.05.55" />
      <path d="M17.8 12.25c2.35-.55 3.65.65 2.7 2.55-.6 1.2-1.75 1.5-3.05.55" />
      <circle cx="9.5" cy="10" r="0.65" fill="currentColor" stroke="none" />
      <circle cx="14.5" cy="10" r="0.65" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function NavIconTrae({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={className}>
      <path d="M4 5h17v15H7v-3H4V5z" />
      <path d="M7 17h11V8H7v9z" />
      <path d="m10 11 2 2-2 2-2-2z" />
      <path d="m16 11 2 2-2 2-2-2z" />
    </svg>
  );
}

export function NavIconQoder({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={className}>
      <path d="M11.6 3.25 6.9 5.9c-2.45 1.4-3.9 3.9-3.9 6.9v2.7c0 2.05 1 3.65 2.65 4.45l4.1 2" />
      <path d="m12.4 20.75 4.7-2.65c2.45-1.4 3.9-3.9 3.9-6.9V8.5c0-2.05-1-3.65-2.65-4.45l-4.1-2" />
      <path d="M14.25 8.35a4.25 4.25 0 1 0 1.9 5.7" />
      <path d="m15 15 2.75 2.75" />
    </svg>
  );
}

export function NavIconKiro({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={className}>
      <path d="M8.2 17.4c-1.4 2.55.8 4.15 3.2 2.35.85 2.3 3.3.9 4.45-.75 2.6-3.75 1.9-9.35.3-12.05-2.1-3.55-7.35-3.5-9.2.15-.8 1.6-.55 3.65-1.05 5.25-.35 1.2-1.15 2.15-2.05 3.2-.95 1.1-.15 3.1 2.15 2.45.7-.2 1.4-.4 2.2-.6z" />
      <ellipse cx="10.5" cy="10.5" rx="0.85" ry="1.35" fill="currentColor" stroke="none" />
      <ellipse cx="14.25" cy="10.5" rx="0.85" ry="1.35" fill="currentColor" stroke="none" />
    </svg>
  );
}

export function NavIconOpencode({ className, ...props }: IconProps) {
  return (
    <svg {...base} {...props} className={className}>
      <circle cx="12" cy="12" r="9" />
      <path d="M8 12 11 15 16 9" />
    </svg>
  );
}

export function NavIconChat(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
    </svg>
  );
}

export function NavIconWorkflow(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <rect width="8" height="8" x="3" y="3" rx="2" />
      <path d="M7 11v4a2 2 0 0 0 2 2h4" />
      <rect width="8" height="8" x="13" y="13" rx="2" />
    </svg>
  );
}

export function NavIconBrackets(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M16 18 22 12 16 6" />
      <path d="m8 6-6 6 6 6" />
    </svg>
  );
}

export function NavIconZap(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
    </svg>
  );
}

export function NavIconFolder(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
    </svg>
  );
}

export function NavIconFolderPlus(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M12 10v6" />
      <path d="M9 13h6" />
      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
    </svg>
  );
}

export function NavIconSettings(props: IconProps) {
  return (
    <svg {...base} {...props}>
      <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

const AGENT_ICONS: Record<string, ComponentType<IconProps>> = {
  cursor: NavIconCursor,
  claude: NavIconClaude,
  codex: NavIconCodex,
  hermes: NavIconHermes,
  openclaw: NavIconOpenClaw,
  trae: NavIconTrae,
  qoder: NavIconQoder,
  kiro: NavIconKiro,
  opencode: NavIconOpencode,
};

export function NavIconForAgent(agentId: string, props?: IconProps) {
  const Ic = AGENT_ICONS[agentId] ?? NavIconBot;
  return <Ic {...props} />;
}
