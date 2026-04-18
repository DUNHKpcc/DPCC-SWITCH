import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { useTranslation } from "react-i18next";
import type { InstallExecutionStep, InstallProgressStage } from "@/types/installer";

const stageLabelKey: Record<InstallProgressStage, string> = {
  queued: "settings.installerProgressStage.queued",
  downloading: "settings.installerProgressStage.downloading",
  installing: "settings.installerProgressStage.installing",
  verifying: "settings.installerProgressStage.verifying",
  completed: "settings.installerProgressStage.completed",
  failed: "settings.installerProgressStage.failed",
  manual: "settings.installerProgressStage.manual",
};

const stageClassName: Record<InstallProgressStage, string> = {
  queued: "border-border-default bg-muted text-muted-foreground",
  downloading: "border-sky-500/20 bg-sky-500/10 text-sky-700 dark:text-sky-300",
  installing: "border-blue-500/20 bg-blue-500/10 text-blue-700 dark:text-blue-300",
  verifying:
    "border-violet-500/20 bg-violet-500/10 text-violet-700 dark:text-violet-300",
  completed:
    "border-emerald-500/20 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
  failed: "border-red-500/20 bg-red-500/10 text-red-700 dark:text-red-300",
  manual: "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-300",
};

interface InstallerProgressPanelProps {
  steps: InstallExecutionStep[];
}

export function InstallerProgressPanel({
  steps,
}: InstallerProgressPanelProps) {
  const { t } = useTranslation();

  return (
    <Card className="border-border-default/80">
      <CardHeader>
        <CardTitle className="text-base">
          {t("settings.installerProgressTitle", {
            defaultValue: "Install Progress",
          })}
        </CardTitle>
      </CardHeader>
      <CardContent>
        {steps.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {t("settings.installerProgressEmpty", {
              defaultValue: "No install activity yet.",
            })}
          </p>
        ) : (
          <ScrollArea className="max-h-64 pr-4">
            <div className="space-y-3">
              {steps.map((step, index) => (
                <div
                  key={`${step.name}-${step.stage}-${index}`}
                  className="rounded-lg border border-border-default/70 p-3"
                >
                  <div className="mb-2 flex items-center justify-between gap-3">
                    <span className="text-sm font-medium lowercase">
                      {step.name}
                    </span>
                    <Badge
                      className={cn("capitalize", stageClassName[step.stage])}
                    >
                      {t(stageLabelKey[step.stage], {
                        defaultValue: step.stage,
                      })}
                    </Badge>
                  </div>
                  <p className="text-sm text-muted-foreground">{step.message}</p>
                </div>
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
