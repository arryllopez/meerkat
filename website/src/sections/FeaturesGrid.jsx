import SectionReveal from '../components/SectionReveal'

const features = [
  {
    title: 'Real-time Sync',
    description: 'Transforms, properties, and names propagate to all clients within 250ms via WebSocket.',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-6 h-6">
        <path d="M4 12c0-4.418 3.582-8 8-8 2.03 0 3.885.756 5.3 2L20 3v5h-5l2.293-2.293A5.96 5.96 0 0012 4c-3.314 0-6 2.686-6 6" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M20 12c0 4.418-3.582 8-8 8a7.96 7.96 0 01-5.3-2L4 21v-5h5l-2.293 2.293A5.96 5.96 0 0012 20c3.314 0 6-2.686 6-6" strokeLinecap="round" strokeLinejoin="round"/>
      </svg>
    ),
  },
  {
    title: 'Shared Asset Library',
    description: 'Place complex 3D models from a shared .blend file. Only lightweight references travel the wire.',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-6 h-6">
        <path d="M4 19.5A2.5 2.5 0 016.5 17H20" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M6.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15A2.5 2.5 0 016.5 2z" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M9 7h6M9 11h4" strokeLinecap="round"/>
      </svg>
    ),
  },
  {
    title: 'User Presence',
    description: 'Color-coded selection highlights show what each collaborator is working on in real time.',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-6 h-6">
        <circle cx="9" cy="7" r="3"/>
        <path d="M3 21v-2a4 4 0 014-4h4a4 4 0 014 4v2" strokeLinecap="round"/>
        <circle cx="17" cy="9" r="2.5"/>
        <path d="M21 21v-1.5a3 3 0 00-2-2.83" strokeLinecap="round"/>
      </svg>
    ),
  },
  {
    title: 'Session Persistence',
    description: 'Sessions survive disconnection and server restarts via event-sourced state reconstruction.',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="w-6 h-6">
        <path d="M12 2L2 7l10 5 10-5-10-5z" strokeLinejoin="round"/>
        <path d="M2 17l10 5 10-5" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M2 12l10 5 10-5" strokeLinecap="round" strokeLinejoin="round"/>
      </svg>
    ),
  },
]

export default function FeaturesGrid() {
  return (
    <section className="py-16 md:py-24 px-6">
      <div className="max-w-[1200px] mx-auto">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 md:gap-6">
          {features.map((f, i) => (
            <SectionReveal key={i} delay={i * 0.08}>
              <div className="p-5 md:p-6 rounded-xl border border-surface-border bg-surface
                hover:border-accent/40 hover:-translate-y-0.5 transition-all duration-300 group">
                <div className="text-text-muted group-hover:text-accent transition-colors duration-300 mb-3">
                  {f.icon}
                </div>
                <h3 className="font-display text-sm md:text-base font-bold text-black mb-1.5">
                  {f.title}
                </h3>
                <p className="text-text-muted text-xs md:text-sm leading-relaxed font-body">
                  {f.description}
                </p>
              </div>
            </SectionReveal>
          ))}
        </div>
      </div>
    </section>
  )
}
