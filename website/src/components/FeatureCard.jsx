import SectionReveal from './SectionReveal'

const ICONS = {
  sync: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-8 h-8">
      <path d="M4 12c0-4.418 3.582-8 8-8 2.03 0 3.885.756 5.3 2L20 3v5h-5l2.293-2.293A5.96 5.96 0 0012 4c-3.314 0-6 2.686-6 6" strokeLinecap="round" strokeLinejoin="round"/>
      <path d="M20 12c0 4.418-3.582 8-8 8a7.96 7.96 0 01-5.3-2L4 21v-5h5l-2.293 2.293A5.96 5.96 0 0012 20c3.314 0 6-2.686 6-6" strokeLinecap="round" strokeLinejoin="round"/>
    </svg>
  ),
  library: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-8 h-8">
      <path d="M4 19.5A2.5 2.5 0 016.5 17H20" strokeLinecap="round" strokeLinejoin="round"/>
      <path d="M6.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15A2.5 2.5 0 016.5 2z" strokeLinecap="round" strokeLinejoin="round"/>
      <path d="M9 7h6M9 11h4" strokeLinecap="round"/>
    </svg>
  ),
  presence: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-8 h-8">
      <circle cx="9" cy="7" r="3"/>
      <path d="M3 21v-2a4 4 0 014-4h4a4 4 0 014 4v2" strokeLinecap="round"/>
      <circle cx="17" cy="9" r="2.5"/>
      <path d="M21 21v-1.5a3 3 0 00-2-2.83" strokeLinecap="round"/>
    </svg>
  ),
  persist: (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-8 h-8">
      <path d="M12 2L2 7l10 5 10-5-10-5z" strokeLinejoin="round"/>
      <path d="M2 17l10 5 10-5" strokeLinecap="round" strokeLinejoin="round"/>
      <path d="M2 12l10 5 10-5" strokeLinecap="round" strokeLinejoin="round"/>
    </svg>
  ),
}

const FEATURES = [
  {
    icon: 'sync',
    title: 'Real-time Sync',
    description: 'Transform, property, and name changes propagate to all connected clients within 250ms via WebSocket.',
  },
  {
    icon: 'library',
    title: 'Shared Asset Library',
    description: 'Place complex 3D models from a shared .blend file. Only lightweight references travel the wire — geometry stays local.',
  },
  {
    icon: 'presence',
    title: 'User Presence',
    description: 'See who is connected with color-coded selection highlights. Know exactly what each collaborator is working on.',
  },
  {
    icon: 'persist',
    title: 'Session Persistence',
    description: 'Sessions survive disconnection and server restarts. Event-sourced state reconstruction from a write-ahead log.',
  },
]

export default function FeatureCard({ index }) {
  const feature = FEATURES[index]

  return (
    <SectionReveal delay={index * 0.1}>
      <div className="bg-surface border border-surface-border rounded-xl p-7 md:p-8
        hover:border-accent/50 hover:-translate-y-1 transition-all duration-300
        hover:shadow-[0_8px_30px_rgba(212,162,83,0.06)] group">
        <div className="text-text-muted group-hover:text-accent transition-colors duration-300 mb-5">
          {ICONS[feature.icon]}
        </div>
        <h3 className="font-display text-lg font-bold text-text-primary mb-2">
          {feature.title}
        </h3>
        <p className="text-text-muted text-sm leading-relaxed">
          {feature.description}
        </p>
      </div>
    </SectionReveal>
  )
}

export { FEATURES }
