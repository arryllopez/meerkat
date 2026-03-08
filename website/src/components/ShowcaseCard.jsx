import SectionReveal from './SectionReveal'

const WEEKS = [
  {
    week: 1,
    title: 'Server Foundation',
    description: 'Built the authoritative Rust WebSocket server with tokio and axum. Implemented session management, connection lifecycle, and the core message routing infrastructure.',
    metrics: ['WebSocket routing', 'Session state', 'Event logging'],
  },
  {
    week: 2,
    title: 'Blender Plugin Skeleton',
    description: 'Created the Blender addon with background WebSocket thread, main-thread dispatcher via bpy.app.timers, and the connection UI panel in the 3D viewport sidebar.',
    metrics: ['Thread-safe bridge', 'UI panel', 'State sync'],
  },
  {
    week: 3,
    title: 'Object Lifecycle',
    description: 'Implemented create, delete, and sync for all object types — primitives, cameras, lights, and asset references from shared .blend libraries.',
    metrics: ['UUID tagging', '7 object types', 'Asset linking'],
  },
  {
    week: 4,
    title: 'Transform & Property Sync',
    description: 'Added 30Hz change-detection polling with echo suppression. Smooth real-time transform sync and property sync for cameras and lights across all connected clients.',
    metrics: ['30Hz polling', 'Echo suppression', 'Property sync'],
  },
  {
    week: 5,
    title: 'Presence & Persistence',
    description: 'User presence panel with color-coded cursors, selection highlighting with GPU-drawn overlays, auto-reconnect with exponential backoff, and scene export.',
    metrics: ['Selection overlay', 'Auto-reconnect', 'Save scene'],
  },
  {
    week: 6,
    title: 'Observability & Polish',
    description: 'File-backed event log with crash recovery, metrics endpoint, property-based testing with proptest, and latency benchmarking across simulated clients.',
    metrics: ['Crash recovery', 'p50/p95/p99', 'Prop testing'],
  },
]

export default function ShowcaseCard({ index }) {
  const data = WEEKS[index]
  const isReversed = index % 2 !== 0

  return (
    <SectionReveal delay={index * 0.08}>
      <div className={`flex flex-col md:flex-row gap-6 md:gap-10 items-stretch ${isReversed ? 'md:flex-row-reverse' : ''}`}>
        {/* Placeholder image */}
        <div className="flex-1 min-h-[200px] md:min-h-[240px] rounded-xl bg-surface border border-surface-border
          flex items-center justify-center overflow-hidden relative group">
          <div className="absolute inset-0 bg-gradient-to-br from-accent-glow/30 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
          <span className="text-text-muted text-sm font-body tracking-wide">Week {data.week} Screenshot</span>
        </div>

        {/* Content */}
        <div className="flex-1 flex flex-col justify-center py-2">
          <span className="text-accent text-xs font-semibold tracking-[0.2em] uppercase mb-2 font-body">
            Week {data.week}
          </span>
          <h3 className="text-xl md:text-2xl font-bold mb-3 font-display text-text-primary">
            {data.title}
          </h3>
          <p className="text-text-muted text-sm leading-relaxed mb-4">
            {data.description}
          </p>
          <div className="flex flex-wrap gap-2">
            {data.metrics.map((m) => (
              <span key={m} className="text-xs px-3 py-1 rounded-full bg-surface border border-surface-border text-text-muted">
                {m}
              </span>
            ))}
          </div>
        </div>
      </div>
    </SectionReveal>
  )
}

export { WEEKS }
