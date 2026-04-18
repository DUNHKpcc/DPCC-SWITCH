import type { ReactNode } from "react";
import { motion } from "framer-motion";

interface LocalEnvironmentCardProps {
  testId: string;
  delay: number;
  header: ReactNode;
  footer?: ReactNode;
}

export function LocalEnvironmentCard({
  testId,
  delay,
  header,
  footer,
}: LocalEnvironmentCardProps) {
  return (
    <motion.div
      data-testid={testId}
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, delay }}
      whileHover={{ scale: 1.02 }}
      className="flex h-[8rem] min-h-[8rem] flex-col gap-1.5 overflow-hidden rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-3.5 shadow-sm transition-colors hover:border-primary/30"
    >
      {header}
      {footer ? <div className="min-h-0 flex-1 overflow-hidden space-y-1.5">{footer}</div> : null}
    </motion.div>
  );
}
