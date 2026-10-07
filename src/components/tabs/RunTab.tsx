import clsx from "clsx";
import { useEffect, useMemo, useState } from "react";
import { CheckIcon, PlayIcon } from "../icons";
import { ConfirmButton } from "../ui/ConfirmButton";
import { SegmentedControl } from "../ui/SegmentedControl";
import { Toggle } from "../ui/Toggle";
import type { ApiError, DispatchInput, DispatchWorkflow } from "../../lib/github";

interface RunTabProps {
  /** Only workflows with a `workflow_dispatch` trigger reach this component. */
  workflows: DispatchWorkflow[];
  loading: boolean;
  error: ApiError | null;
  /** The ref every dispatch uses; there is no branch picker. */
  defaultBranch: string;
  onDispatch: (workflow: DispatchWorkflow, inputs: Record<string, string>) => void;
}

/** Input values start at the workflow's own defaults. */
function defaultValues(inputs: DispatchInput[]): Record<string, string> {
  return Object.fromEntries(inputs.map((input) => [input.key, input.default]));
}

export function RunTab({ workflows, loading, error, defaultBranch, onDispatch }: RunTabProps) {
  // Keyed by path, not by index: the list is refetched, and an index would
  // silently come to mean a different workflow.
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [values, setValues] = useState<Record<string, string>>({});

  const selected = useMemo(
    () => workflows.find((workflow) => workflow.path === selectedPath) ?? null,
    [workflows, selectedPath],
  );

  // Fall back to the first workflow whenever the selection no longer exists —
  // on first load, and after switching repository.
  useEffect(() => {
    if (selected || workflows.length === 0) return;
    const first = workflows[0];
    setSelectedPath(first.path);
    setValues(defaultValues(first.inputs));
  }, [workflows, selected]);

  function select(workflow: DispatchWorkflow) {
    setSelectedPath(workflow.path);
    setValues(defaultValues(workflow.inputs));
  }

  if (loading && workflows.length === 0) {
    return <p className="px-3 py-4 text-xs text-fg2">Reading workflow files…</p>;
  }

  if (error) {
    return <p className="px-3 py-4 text-xs text-red-dot">{error.message}</p>;
  }

  if (workflows.length === 0) {
    return (
      <p className="px-3 py-4 text-xs text-fg2">
        No workflow in this repository can be started by hand. Only workflows with a
        <span className="font-mono"> workflow_dispatch </span>
        trigger can.
      </p>
    );
  }

  // GitHub would answer 422 anyway; refusing here says which field is missing.
  const missing = selected
    ? selected.inputs.filter((input) => input.required && !values[input.key]?.trim())
    : [];

  return (
    <div className="flex flex-col gap-3.5 px-2 pb-2 pt-1">
      <section className="flex flex-col gap-1.5">
        <span className="pl-0.5 text-[11px] font-medium text-fg2">Workflow</span>
        <div className="overflow-hidden rounded-[10px] bg-card shadow-[inset_0_0_0_0.5px_var(--sep)]">
          {workflows.map((workflow) => (
            <button
              key={workflow.path}
              type="button"
              onClick={() => select(workflow)}
              className="flex w-full cursor-pointer items-center gap-2.5 border-0 border-t-[0.5px] border-sep bg-transparent px-3 py-[9px] text-left text-fg first:border-t-0 hover:bg-hover"
            >
              <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="truncate text-[13px] font-medium">{workflow.name}</span>
                <span className="truncate font-mono text-[11px] text-fg2">{workflow.path}</span>
              </span>
              <CheckIcon
                width="14"
                height="14"
                className={clsx(
                  "flex-none text-accent",
                  workflow.path === selectedPath ? "opacity-100" : "opacity-0",
                )}
              />
            </button>
          ))}
        </div>
      </section>

      {/* Read-only: this repository has over 200 branches, and a deploy runs
          from the default one in practice. */}
      <div className="flex items-center justify-between gap-3">
        <span className="text-[13px]">Branch</span>
        <span className="truncate rounded-md bg-fill px-2 py-[5px] font-mono text-xs text-fg2">
          {defaultBranch}
        </span>
      </div>

      {selected?.inputs.map((input) => {
        const value = values[input.key] ?? "";
        const set = (next: string) => setValues((prev) => ({ ...prev, [input.key]: next }));

        if (input.kind === "choice") {
          return (
            <div key={input.key} className="flex flex-col gap-1.5">
              <span className="text-[13px]">{input.label}</span>
              <SegmentedControl
                options={input.options.map((option) => ({ value: option, label: option }))}
                value={value}
                onChange={set}
              />
            </div>
          );
        }

        if (input.kind === "bool") {
          return (
            <div key={input.key} className="flex items-center justify-between gap-3">
              <span className="min-w-0 flex-1 text-[13px]">{input.label}</span>
              <Toggle
                label={input.label}
                checked={value === "true"}
                onChange={() => set(value === "true" ? "false" : "true")}
              />
            </div>
          );
        }

        return (
          <div key={input.key} className="flex items-center justify-between gap-3">
            <span className="min-w-0 flex-1 text-[13px]">
              {input.label}
              {input.required && <span className="text-fg3"> *</span>}
            </span>
            <input
              value={value}
              onChange={(event) => set(event.target.value)}
              spellCheck={false}
              className="w-[150px] flex-none rounded-md border-0 bg-fill px-2 py-1.5 text-right font-mono text-xs text-fg outline-none"
            />
          </div>
        );
      })}

      {selected?.inputs.length === 0 && (
        <span className="text-xs text-fg2">This workflow has no inputs.</span>
      )}

      {missing.length > 0 && (
        <span className="text-[11px] text-fg2">
          {`${missing.map((input) => input.key).join(", ")} ${missing.length === 1 ? "is" : "are"} required.`}
        </span>
      )}

      <ConfirmButton
        variant="primary"
        title="Run workflow"
        disabled={!selected || missing.length > 0}
        onConfirm={() => selected && onDispatch(selected, values)}
      >
        <PlayIcon />
        Run workflow
      </ConfirmButton>
    </div>
  );
}
