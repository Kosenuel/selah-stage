import { useState } from "react";

import { Field, NumberField, SelectField, TextField } from "@/components/fields";
import type {
  CueChangeInput_Deserialize as CueChange,
  CueView_Serialize as CueView,
  CurveKind,
  FollowMode,
  GroupType,
} from "@/lib/bindings";

const FOLLOW: { value: FollowMode; label: string }[] = [
  { value: "none", label: "Wait for GO" },
  { value: "continue", label: "Start the next cue at the same time" },
  { value: "follow", label: "Start the next cue when this one ends" },
];

const CURVES: { value: CurveKind; label: string }[] = [
  { value: "equalPower", label: "Smooth (equal power)" },
  { value: "linear", label: "Straight" },
  { value: "sCurve", label: "Slow at both ends" },
];

const GROUP_MODES: { value: GroupType; label: string }[] = [
  { value: "together", label: "All at once" },
  { value: "sequence", label: "One after another" },
  { value: "shuffle", label: "One at random each time" },
];

const CONTROL_TITLE = {
  fade: "Fade another cue",
  stop: "Stop another cue",
  pause: "Pause another cue",
  goTo: "Move the playhead",
  arm: "Arm or disarm another cue",
} as const;

/** A cue that a control cue can be pointed at. */
export interface TargetOption {
  id: string;
  label: string;
  /** In the main list, not inside a group. */
  topLevel: boolean;
}

type PlayCount = "once" | "forever" | "times";

function playCount(loops: number | null): PlayCount {
  if (loops === null || loops === 1) {
    return "once";
  }
  return loops === 0 ? "forever" : "times";
}

/** The settings of the selected cue. Every change goes to the core. */
export function CueInspector({
  cue,
  targets,
  onChange,
}: {
  cue: CueView;
  targets: TargetOption[];
  onChange: (change: CueChange) => void;
}) {
  const audio = cue.audio;
  const control = cue.control;
  const group = cue.group;
  return (
    <aside
      aria-label={`Settings for ${cue.name}`}
      className="flex w-80 shrink-0 flex-col gap-4 overflow-auto rounded-xl border border-border bg-surface p-4"
    >
      <h2 className="text-sm font-semibold">Cue {cue.number}</h2>

      <TextField
        label="Name"
        value={cue.name}
        onCommit={(value) => {
          onChange({ type: "name", value });
        }}
      />
      <TextField
        label="Number"
        value={cue.number}
        onCommit={(value) => {
          onChange({ type: "number", value });
        }}
      />
      <TextField
        label="Notes"
        value={cue.notes}
        multiline
        onCommit={(value) => {
          onChange({ type: "notes", value });
        }}
      />

      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={cue.armed}
          onChange={(e) => {
            onChange({ type: "armed", value: e.target.checked });
          }}
        />
        Armed (a disarmed cue is skipped)
      </label>

      <fieldset className="flex flex-col gap-3">
        <legend className="mb-1 text-xs font-semibold tracking-wide text-text-muted uppercase">
          Timing
        </legend>
        <NumberField
          label="Wait before starting"
          unit="seconds"
          value={cue.preWait}
          onCommit={(value) => {
            if (value !== null) {
              onChange({ type: "preWait", value });
            }
          }}
        />
        <SelectField
          label="After this cue starts"
          value={cue.follow}
          options={FOLLOW}
          onChange={(value) => {
            onChange({ type: "follow", value });
          }}
        />
        {cue.follow !== "none" && (
          <NumberField
            label="Wait before the next cue"
            unit="seconds"
            value={cue.postWait}
            onCommit={(value) => {
              if (value !== null) {
                onChange({ type: "postWait", value });
              }
            }}
          />
        )}
      </fieldset>

      {control && (
        <fieldset className="flex flex-col gap-3">
          <legend className="mb-1 text-xs font-semibold tracking-wide text-text-muted uppercase">
            {CONTROL_TITLE[control.kind]}
          </legend>
          <SelectField
            label="Acts on"
            value={control.target}
            options={[
              ...targets
                .filter((t) => t.id !== cue.id && (control.kind !== "goTo" || t.topLevel))
                .map((t) => ({ value: t.id, label: t.label })),
              ...(control.targetName === null
                ? [{ value: control.target, label: "(a deleted cue)" }]
                : []),
            ]}
            onChange={(value) => {
              onChange({ type: "target", value });
            }}
          />
          {control.targetName === null && (
            <p role="status" className="text-xs text-standby">
              <span aria-hidden="true">⚠ </span>
              The cue this acts on was deleted, so it does nothing. Choose another.
            </p>
          )}
          {control.kind === "fade" && (
            <>
              <NumberField
                label="Fade to"
                unit="dB, −120 is silence"
                value={control.to}
                onCommit={(value) => {
                  if (value !== null) {
                    onChange({ type: "fadeTo", value });
                  }
                }}
              />
              <NumberField
                label="Fade time"
                unit="seconds"
                value={control.duration}
                onCommit={(value) => {
                  if (value !== null) {
                    onChange({ type: "fadeDuration", value });
                  }
                }}
              />
              <SelectField
                label="Fade shape"
                value={control.curve}
                options={CURVES}
                onChange={(value) => {
                  onChange({ type: "curve", value });
                }}
              />
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={control.stopWhenDone}
                  onChange={(e) => {
                    onChange({ type: "stopWhenDone", value: e.target.checked });
                  }}
                />
                Stop it when the fade ends
              </label>
            </>
          )}
          {control.kind === "arm" && (
            <SelectField
              label="Action"
              value={control.arms ? "arm" : "disarm"}
              options={[
                { value: "arm", label: "Arm it (it will play when reached)" },
                { value: "disarm", label: "Disarm it (it will be skipped)" },
              ]}
              onChange={(value) => {
                onChange({ type: "armAction", value: value === "arm" });
              }}
            />
          )}
        </fieldset>
      )}

      {group && (
        <fieldset className="flex flex-col gap-3">
          <legend className="mb-1 text-xs font-semibold tracking-wide text-text-muted uppercase">
            Group
          </legend>
          <SelectField
            label="Start the cues inside"
            value={group.mode}
            options={GROUP_MODES}
            onChange={(value) => {
              onChange({ type: "groupMode", value });
            }}
          />
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={group.looped}
              onChange={(e) => {
                onChange({ type: "looped", value: e.target.checked });
              }}
            />
            Start again when finished
          </label>
        </fieldset>
      )}

      {audio && (
        <fieldset className="flex flex-col gap-3">
          <legend className="mb-1 text-xs font-semibold tracking-wide text-text-muted uppercase">
            Audio
          </legend>
          <p className="text-xs break-all text-text-faint select-text">{audio.file}</p>

          <LevelControl
            value={audio.level ?? 0}
            onChange={(value) => {
              onChange({ type: "level", value });
            }}
          />
          <NumberField
            label="Start at"
            unit="seconds"
            value={audio.start}
            onCommit={(value) => {
              if (value !== null) {
                onChange({ type: "start", value });
              }
            }}
          />
          <NumberField
            label="Stop at"
            unit="seconds"
            value={audio.end}
            optional
            onCommit={(value) => {
              onChange({ type: "end", value });
            }}
          />
          <NumberField
            label="Fade in"
            unit="seconds"
            value={audio.fadeIn}
            onCommit={(value) => {
              if (value !== null) {
                onChange({ type: "fadeIn", value });
              }
            }}
          />
          <NumberField
            label="Fade out at the end"
            unit="seconds"
            value={audio.fadeOut}
            onCommit={(value) => {
              if (value !== null) {
                onChange({ type: "fadeOut", value });
              }
            }}
          />
          <SelectField
            label="Fade shape"
            value={audio.curve}
            options={CURVES}
            onChange={(value) => {
              onChange({ type: "curve", value });
            }}
          />
          <Repeat
            loops={audio.loops}
            onChange={(value) => {
              onChange({ type: "loops", value });
            }}
          />
        </fieldset>
      )}
    </aside>
  );
}

/**
 * A level slider. While it is being dragged it shows its own position, so the
 * thumb does not jump back to the last value the core reported.
 */
function LevelControl({ value, onChange }: { value: number; onChange: (value: number) => void }) {
  const [drag, setDrag] = useState<number | null>(null);
  const shown = drag ?? value;
  const stop = () => {
    setDrag(null);
  };
  return (
    <Field label="Level">
      {(id) => (
        <div className="flex items-center gap-3">
          <input
            id={id}
            type="range"
            min={-60}
            max={12}
            step={0.5}
            value={Math.max(-60, shown)}
            className="flex-1"
            onChange={(e) => {
              const next = Number(e.target.value);
              setDrag(next);
              onChange(next);
            }}
            onPointerUp={stop}
            onKeyUp={stop}
            onBlur={stop}
          />
          <output className="w-16 text-right text-sm text-text-muted tabular-nums">
            {shown <= -60 ? "−∞ dB" : `${shown.toFixed(1)} dB`}
          </output>
        </div>
      )}
    </Field>
  );
}

function Repeat({
  loops,
  onChange,
}: {
  loops: number | null;
  onChange: (loops: number | null) => void;
}) {
  const mode = playCount(loops);
  return (
    <>
      <SelectField
        label="Play"
        value={mode}
        options={[
          { value: "once", label: "Once" },
          { value: "forever", label: "Over and over until stopped" },
          { value: "times", label: "A set number of times" },
        ]}
        onChange={(next) => {
          onChange(next === "once" ? null : next === "forever" ? 0 : 2);
        }}
      />
      {mode === "times" && (
        <NumberField
          label="Times"
          value={loops}
          step={1}
          onCommit={(value) => {
            if (value !== null && Number.isInteger(value) && value >= 1) {
              onChange(value);
            }
          }}
        />
      )}
    </>
  );
}
