import React, { useState, useEffect, useRef } from "react";

const SplitPane = ({ leftChildren, rightChildren, storageKey = "splitRatio" }) => {
  const [leftWidth, setLeftWidth] = useState(50);
  const [isDragging, setIsDragging] = useState(false);
  const containerRef = useRef(null);

  useEffect(() => {
    const saved = localStorage.getItem(storageKey);
    if (saved) setLeftWidth(parseFloat(saved));
  }, [storageKey]);

  const handleMouseDown = () => setIsDragging(true);

  const handleMouseUp = () => {
    setIsDragging(false);
    localStorage.setItem(storageKey, leftWidth.toString());
  };

  const handleMouseMove = (e) => {
    if (!isDragging || !containerRef.current) return;
    const containerRect = containerRef.current.getBoundingClientRect();
    const newWidth = ((e.clientX - containerRect.left) / containerRect.width) * 100;

    if (newWidth > 20 && newWidth < 80) {
      setLeftWidth(newWidth);
    }
  };

  useEffect(() => {
    if (isDragging) {
      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
    } else {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    }
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isDragging]);

  return (
    <div className="app-container" ref={containerRef}>
      <div className="sidebar" style={{ width: `${leftWidth}%` }}>
        {leftChildren}
      </div>

      <div
        className={`resizer ${isDragging ? "active" : ""}`}
        onMouseDown={handleMouseDown}
        role="separator"
        aria-valuenow={Math.round(leftWidth)}
        aria-label="Resize panel"
        tabIndex={0}
      />

      <div className="map-container" style={{ width: `${100 - leftWidth}%` }}>
        {rightChildren}
      </div>
    </div>
  );
};

export default SplitPane;
