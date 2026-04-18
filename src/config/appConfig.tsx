import React from "react";
import type { AppId } from "@/lib/api/types";
import {
  ClaudeIcon,
  CodexIcon,
  GeminiIcon,
  OpenClawIcon,
} from "@/components/BrandIcons";
import { ProviderIcon } from "@/components/ProviderIcon";

export interface AppConfig {
  label: string;
  icon: React.ReactNode;
  activeClass: string;
  badgeClass: string;
}

export const APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "openclaw",
];

/** App IDs shown in MCP & Skills panels (excludes OpenClaw) */
export const MCP_SKILLS_APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "opencode",
];

export const APP_ICON_MAP: Record<AppId, AppConfig> = {
  claude: {
    label: "Claude",
    icon: <ClaudeIcon size={14} />,
    activeClass:
      "bg-black/5 ring-1 ring-black/15 hover:bg-black/10 text-foreground dark:bg-white/10 dark:ring-white/15 dark:hover:bg-white/15 dark:text-foreground",
    badgeClass:
      "bg-black/5 text-foreground hover:bg-black/10 border-0 gap-1.5 dark:bg-white/10 dark:hover:bg-white/15",
  },
  codex: {
    label: "Codex",
    icon: <CodexIcon size={14} />,
    activeClass:
      "bg-black/5 ring-1 ring-black/15 hover:bg-black/10 text-foreground dark:bg-white/10 dark:ring-white/15 dark:hover:bg-white/15 dark:text-foreground",
    badgeClass:
      "bg-black/5 text-foreground hover:bg-black/10 border-0 gap-1.5 dark:bg-white/10 dark:hover:bg-white/15",
  },
  gemini: {
    label: "Gemini",
    icon: <GeminiIcon size={14} />,
    activeClass:
      "bg-black/5 ring-1 ring-black/15 hover:bg-black/10 text-foreground dark:bg-white/10 dark:ring-white/15 dark:hover:bg-white/15 dark:text-foreground",
    badgeClass:
      "bg-black/5 text-foreground hover:bg-black/10 border-0 gap-1.5 dark:bg-white/10 dark:hover:bg-white/15",
  },
  opencode: {
    label: "OpenCode",
    icon: (
      <ProviderIcon
        icon="opencode"
        name="OpenCode"
        size={14}
        showFallback={false}
      />
    ),
    activeClass:
      "bg-black/5 ring-1 ring-black/15 hover:bg-black/10 text-foreground dark:bg-white/10 dark:ring-white/15 dark:hover:bg-white/15 dark:text-foreground",
    badgeClass:
      "bg-black/5 text-foreground hover:bg-black/10 border-0 gap-1.5 dark:bg-white/10 dark:hover:bg-white/15",
  },
  openclaw: {
    label: "OpenClaw",
    icon: <OpenClawIcon size={14} />,
    activeClass:
      "bg-black/5 ring-1 ring-black/15 hover:bg-black/10 text-foreground dark:bg-white/10 dark:ring-white/15 dark:hover:bg-white/15 dark:text-foreground",
    badgeClass:
      "bg-black/5 text-foreground hover:bg-black/10 border-0 gap-1.5 dark:bg-white/10 dark:hover:bg-white/15",
  },
};
