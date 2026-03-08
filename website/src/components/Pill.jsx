export default function Pill({ label }) {
  return (
    <span className="px-4 py-1.5 text-sm font-medium rounded-full border border-surface-border bg-surface text-text-muted
      hover:border-accent hover:text-accent transition-all duration-200 hover:scale-105 cursor-default select-none">
      {label}
    </span>
  )
}
