import { useState, useCallback, useRef, useEffect } from "react";

const TAB_STORAGE_KEY = "hslActiveTab";
const TAB_ORDER = ["routes", "settings", "about"];

const parseStoredTab = () => {
  try {
    const raw = localStorage.getItem(TAB_STORAGE_KEY);
    if (!raw) return "routes";
    const parsed = JSON.parse(raw);
    return TAB_ORDER.includes(parsed) ? parsed : "routes";
  } catch {
    return "routes";
  }
};

export function useTabNavigation() {
  const [activeTab, setActiveTab] = useState(parseStoredTab);
  const [focusedTab, setFocusedTab] = useState(activeTab);
  const tabRefs = useRef({});

  useEffect(() => {
    localStorage.setItem(TAB_STORAGE_KEY, JSON.stringify(activeTab));
    setFocusedTab(activeTab);
  }, [activeTab]);

  const focusTab = useCallback((tabId) => {
    setFocusedTab(tabId);
    const tabNode = tabRefs.current[tabId];
    if (tabNode) tabNode.focus();
  }, []);

  const handleTabKeyDown = useCallback(
    (event, tabId) => {
      const currentIndex = TAB_ORDER.indexOf(tabId);
      if (currentIndex === -1) return;

      if (event.key === "ArrowRight") {
        event.preventDefault();
        focusTab(TAB_ORDER[(currentIndex + 1) % TAB_ORDER.length]);
        return;
      }
      if (event.key === "ArrowLeft") {
        event.preventDefault();
        focusTab(TAB_ORDER[(currentIndex - 1 + TAB_ORDER.length) % TAB_ORDER.length]);
        return;
      }
      if (event.key === "Home") {
        event.preventDefault();
        focusTab(TAB_ORDER[0]);
        return;
      }
      if (event.key === "End") {
        event.preventDefault();
        focusTab(TAB_ORDER[TAB_ORDER.length - 1]);
        return;
      }
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        setActiveTab(tabId);
      }
    },
    [focusTab],
  );

  return { activeTab, setActiveTab, focusedTab, setFocusedTab, tabRefs, handleTabKeyDown };
}
