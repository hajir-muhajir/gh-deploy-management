import { Badge } from "../ui/Badge";
import { SegmentedControl, type SegmentOption } from "../ui/SegmentedControl";
import type { TabId, ViewId } from "../../types";

interface TabBarProps {
  /** Accepts ViewId so "settings" simply renders no active tab. */
  value: ViewId;
  onChange: (tab: TabId) => void;
  failingCount: number;
  reviewCount: number;
}

export function TabBar({ value, onChange, failingCount, reviewCount }: TabBarProps) {
  const options: SegmentOption<TabId>[] = [
    { value: "releases", label: "Releases" },
    { value: "run", label: "Run" },
    { value: "actions", label: "Actions", trailing: <Badge count={failingCount} /> },
    { value: "prs", label: "PRs", trailing: <Badge count={reviewCount} /> },
  ];

  return (
    <div className="px-3 pb-2">
      <SegmentedControl options={options} value={value} onChange={onChange} />
    </div>
  );
}
