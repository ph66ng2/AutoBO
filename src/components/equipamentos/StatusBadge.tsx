import {
  corStatusEquipamento,
  labelStatusEquipamento,
} from "../../lib/status-equipamento";

export function StatusBadge({ status }: { status?: string | null }) {
  const label = labelStatusEquipamento(status);
  const color = corStatusEquipamento(status);
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${color}`}
    >
      {label}
    </span>
  );
}
