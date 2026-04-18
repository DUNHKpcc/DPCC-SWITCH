import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { useTranslation } from "react-i18next";
import type {
  InstallerDependencyState,
  InstallerDependencyStatus,
} from "@/types/installer";

const stateLabelKey: Record<InstallerDependencyState, string> = {
  installed: "settings.installerDependencyState.installed",
  missing: "settings.installerDependencyState.missing",
  outdated: "settings.installerDependencyState.outdated",
  broken: "settings.installerDependencyState.broken",
  manual: "settings.installerDependencyState.manual",
};

const kindLabelKey: Record<InstallerDependencyStatus["kind"], string> = {
  core: "settings.installerDependencyKind.core",
  tool: "settings.installerDependencyKind.tool",
};

const stateClassName: Record<InstallerDependencyState, string> = {
  installed:
    "border-emerald-500/20 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
  missing:
    "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  outdated:
    "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  broken:
    "border-red-500/20 bg-red-500/10 text-red-700 dark:text-red-300",
  manual: "border-sky-500/20 bg-sky-500/10 text-sky-700 dark:text-sky-300",
};

interface InstallerDependencyCardProps {
  dependency: InstallerDependencyStatus;
}

export function InstallerDependencyCard({
  dependency,
}: InstallerDependencyCardProps) {
  const { t } = useTranslation();

  return (
    <Card className="h-full border-border-default/80">
      <CardHeader className="gap-3 space-y-0 pb-4">
        <div className="flex items-start justify-between gap-3">
          <div className="space-y-1">
            <CardTitle className="text-base font-semibold lowercase">
              {dependency.name}
            </CardTitle>
            <p className="text-sm text-muted-foreground capitalize">
              {t(kindLabelKey[dependency.kind], {
                defaultValue: dependency.kind,
              })}
            </p>
          </div>
          <Badge className={cn("capitalize", stateClassName[dependency.state])}>
            {t(stateLabelKey[dependency.state], {
              defaultValue: dependency.state,
            })}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-2 text-sm">
        <div className="flex flex-wrap gap-x-3 gap-y-1 text-muted-foreground">
          <span>
            {dependency.version ??
              t("settings.installerVersionUnavailable", {
                defaultValue: "Version unavailable",
              })}
          </span>
          {dependency.path ? (
            <span className="break-all">{dependency.path}</span>
          ) : null}
        </div>
        {dependency.message ? (
          <p className="text-sm text-muted-foreground">{dependency.message}</p>
        ) : null}
      </CardContent>
    </Card>
  );
}
