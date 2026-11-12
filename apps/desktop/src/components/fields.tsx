import { useId, type ReactNode } from "react";

/** A label above a control, wired together for screen readers. */
export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: (id: string) => ReactNode;
}) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="text-xs font-medium text-text-muted">
        {label}
      </label>
      {children(id)}
      {hint && <span className="text-xs text-text-faint">{hint}</span>}
    </div>
  );
}

const INPUT =
  "border-border bg-surface text-text w-full rounded-lg border px-3 py-2 text-sm select-text disabled:opacity-50";

/**
 * A text box that sends its value when the operator leaves it or presses
 * Enter. It shows what the core holds: when that changes, the box resets to it.
 */
export function TextField({
  label,
  value,
  onCommit,
  disabled,
  multiline,
}: {
  label: string;
  value: string;
  onCommit: (value: string) => void;
  disabled?: boolean;
  multiline?: boolean;
}) {
  return (
    <Field label={label}>
      {(id) =>
        multiline ? (
          <textarea
            key={value}
            id={id}
            defaultValue={value}
            disabled={disabled}
            rows={3}
            className={INPUT}
            onBlur={(e) => {
              if (e.target.value !== value) {
                onCommit(e.target.value);
              }
            }}
          />
        ) : (
          <input
            key={value}
            id={id}
            type="text"
            defaultValue={value}
            disabled={disabled}
            className={INPUT}
            onBlur={(e) => {
              if (e.target.value !== value) {
                onCommit(e.target.value);
              }
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.currentTarget.blur();
              }
            }}
          />
        )
      }
    </Field>
  );
}

/**
 * A number box, with an optional unit. Leaving it empty means "none" when
 * `optional` is set. Invalid text is not sent; the core's own limits are
 * checked by the core.
 */
export function NumberField({
  label,
  value,
  onCommit,
  unit,
  step = 0.1,
  optional = false,
  disabled,
}: {
  label: string;
  value: number | null;
  onCommit: (value: number | null) => void;
  unit?: string;
  step?: number;
  optional?: boolean;
  disabled?: boolean;
}) {
  const shown = value === null ? "" : String(Math.round(value * 1000) / 1000);
  const commit = (text: string) => {
    const trimmed = text.trim();
    if (trimmed === "") {
      if (optional && value !== null) {
        onCommit(null);
      }
      return;
    }
    const parsed = Number(trimmed);
    if (Number.isFinite(parsed) && parsed !== value) {
      onCommit(parsed);
    }
  };
  return (
    <Field label={unit ? `${label} (${unit})` : label}>
      {(id) => (
        <input
          key={shown}
          id={id}
          type="text"
          inputMode="decimal"
          defaultValue={shown}
          placeholder={optional ? "None" : undefined}
          disabled={disabled}
          data-step={step}
          className={INPUT}
          onBlur={(e) => {
            commit(e.target.value);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.currentTarget.blur();
            }
          }}
        />
      )}
    </Field>
  );
}

export function SelectField<T extends string>({
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  disabled?: boolean;
}) {
  return (
    <Field label={label}>
      {(id) => (
        <select
          id={id}
          value={value}
          disabled={disabled}
          className={INPUT}
          onChange={(e) => {
            onChange(e.target.value as T);
          }}
        >
          {options.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      )}
    </Field>
  );
}
