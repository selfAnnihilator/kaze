import { memo, type ReactNode } from "react";
import type { DownloadTask } from "../types";
import { useDownloadProgress } from "../services/downloadProgress";

export const LiveDownloadTask = memo(function LiveDownloadTask({
  task,
  children,
}: {
  task: DownloadTask;
  children: (task: DownloadTask) => ReactNode;
}) {
  return children(useDownloadProgress(task));
});
