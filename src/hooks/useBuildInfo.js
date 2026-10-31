import { useState, useEffect } from "react";
import { invokeTauri } from "../services/runtime";

export function useBuildInfo({ activeTab, isTauri }) {
  const [buildInfo, setBuildInfo] = useState(null);
  const [buildInfoError, setBuildInfoError] = useState(null);
  const [buildInfoLoading, setBuildInfoLoading] = useState(false);

  useEffect(() => {
    if (!isTauri || activeTab !== "about" || buildInfo || buildInfoLoading) return;

    const loadBuildInfo = async () => {
      setBuildInfoLoading(true);
      setBuildInfoError(null);
      try {
        const info = await invokeTauri("get_dataset_stats");
        setBuildInfo(info);
      } catch (err) {
        setBuildInfoError(`Failed to load dataset stats: ${err}`);
      } finally {
        setBuildInfoLoading(false);
      }
    };

    loadBuildInfo();
  }, [activeTab, buildInfo, buildInfoLoading, isTauri]);

  return { buildInfo, buildInfoError, buildInfoLoading };
}
