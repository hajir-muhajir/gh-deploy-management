import clsx from "clsx";
import { CheckIcon, PlayIcon } from "../icons";
import { SegmentedControl } from "../ui/SegmentedControl";
import { Toggle } from "../ui/Toggle";
import { BRANCHES, WORKFLOWS } from "../../data/dummy";
import type { InputValue } from "../../types";

interface RunTabProps {
  workflowIndex: number;
  branch: string;
  values: Record<string, InputValue>;
  onSelectWorkflow: (index: number) => void;
  onBranchChange: (branch: string) => void;
  onValueChange: (key: string, value: InputValue) => void;
  onSubmit: () => void;
}

export function RunTab({
  workflowIndex,
  branch,
  values,
  onSelectWorkflow,
  onBranchChange,
  onValueChange,
  onSubmit,
}: RunTabProps) {
  const workflow = WORKFLOWS[workflowIndex];

  return (
    <div className="flex flex-col gap-3.5 px-2 pb-2 pt-1">
      <section className="flex flex-col gap-1.5">
        <span className="pl-0.5 text-[11px] font-medium text-fg2">Workflow</span>
        <div className="overflow-hidden rounded-[10px] bg-card shadow-[inset_0_0_0_0.5px_var(--sep)]">
          {WORKFLOWS.map((wf, index) => (
            <button
              key={wf.file}
              type="button"
              onClick={() => onSelectWorkflow(index)}
              className="flex w-full cursor-pointer items-center gap-2.5 border-0 border-t-[0.5px] border-sep bg-transparent px-3 py-[9px] text-left text-fg first:border-t-0 hover:bg-hover"
            >
              <span className="flex flex-1 flex-col gap-0.5">
                <span className="text-[13px] font-medium">{wf.name}</span>
                <span className="font-mono text-[11px] text-fg2">{`.github/workflows/${wf.file}`}</span>
              </span>
              <CheckIcon
                width="14"
                height="14"
                className={clsx("text-accent", index === workflowIndex ? "opacity-100" : "opacity-0")}
              />
            </button>
          ))}
        </div>
      </section>

      <div className="flex items-center justify-between gap-3">
        <span className="text-[13px]">Branch</span>
        <select
          value={branch}
          onChange={(e) => onBranchChange(e.target.value)}
          className="rounded-md border-0 bg-fill px-2 py-[5px] font-mono text-xs text-fg"
        >
          {BRANCHES.map((b) => (
            <option key={b} value={b}>
              {b}
            </option>
          ))}
        </select>
      </div>

      {workflow.inputs.map((input) => {
        if (input.type === "choice") {
          return (
            <div key={input.key} className="flex flex-col gap-1.5">
              <span className="text-[13px]">{input.label}</span>
              <SegmentedControl
                options={input.options.map((o) => ({ value: o, label: o }))}
                value={String(values[input.key] ?? input.def)}
                onChange={(v) => onValueChange(input.key, v)}
              />
            </div>
          );
        }

        if (input.type === "text") {
          return (
            <div key={input.key} className="flex items-center justify-between gap-3">
              <span className="text-[13px]">{input.label}</span>
              <input
                value={String(values[input.key] ?? "")}
                onChange={(e) => onValueChange(input.key, e.target.value)}
                placeholder={input.ph}
                className="w-[150px] rounded-md border-0 bg-fill px-2 py-1.5 text-right font-mono text-xs text-fg outline-none"
              />
            </div>
          );
        }

        return (
          <div key={input.key} className="flex items-center justify-between gap-3">
            <span className="flex flex-col gap-px">
              <span className="text-[13px]">{input.label}</span>
              {input.hint && <span className="text-[11px] text-fg2">{input.hint}</span>}
            </span>
            <Toggle
              label={input.label}
              checked={Boolean(values[input.key])}
              onChange={() => onValueChange(input.key, !values[input.key])}
            />
          </div>
        );
      })}

      {workflow.inputs.length === 0 && (
        <span className="text-xs text-fg2">This workflow has no inputs.</span>
      )}

      <button
        type="button"
        onClick={onSubmit}
        className="mt-0.5 flex cursor-pointer items-center justify-center gap-[7px] rounded-lg border-0 bg-accent py-[9px] text-[13px] font-semibold text-white hover:brightness-110"
      >
        <PlayIcon />
        Run workflow
      </button>
    </div>
  );
}
