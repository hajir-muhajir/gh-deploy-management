import { useCallback, useState } from "react";
import { PreviewBackdrop } from "./components/PreviewBackdrop";
import { FlyoutPanel } from "./components/flyout/FlyoutPanel";
import { useIsTauri } from "./hooks/useIsTauri";
import { useTheme } from "./hooks/useTheme";

function App() {
  const theme = useTheme();
  const isTauri = useIsTauri();

  // Inside Tauri the OS shows/hides the window, so the panel is always "open".
  const [open, setOpen] = useState(true);
  const [badgeCount, setBadgeCount] = useState(0);

  const onBadgeChange = useCallback((count: number) => setBadgeCount(count), []);

  const panel = (
    <FlyoutPanel
      theme={theme}
      open={isTauri || open}
      anchored={!isTauri}
      onBadgeChange={onBadgeChange}
    />
  );

  return (
    <div
      data-theme={theme.theme}
      data-accent={theme.accent}
      data-glass={isTauri ? "off" : "on"}
      className="h-full w-full bg-solid font-sans text-[13px]"
    >
      {isTauri ? (
        panel
      ) : (
        <PreviewBackdrop
          open={open}
          onToggle={() => setOpen((v) => !v)}
          badgeCount={badgeCount}
        >
          {panel}
        </PreviewBackdrop>
      )}
    </div>
  );
}

export default App;
