import SectionReveal from '../components/SectionReveal'

export default function Demo() {
  return (
    <section className="py-20 md:py-28 px-6">
      <div className="max-w-4xl mx-auto">
        <SectionReveal>
          <span className="text-accent text-xs font-semibold tracking-[0.2em] uppercase font-body block mb-3">
            See It In Action
          </span>
          <h2
            className="font-display font-bold text-text-primary mb-12"
            style={{ fontSize: 'clamp(1.75rem, 4vw, 2.75rem)' }}
          >
            Live Demo
          </h2>
        </SectionReveal>

        <SectionReveal delay={0.1}>
          <div className="relative aspect-video rounded-xl bg-surface border border-surface-border overflow-hidden group cursor-pointer
            hover:border-accent/30 transition-all duration-500 hover:shadow-[0_0_40px_rgba(212,162,83,0.08)]">
            {/* Subtle gradient overlay */}
            <div className="absolute inset-0 bg-gradient-to-b from-transparent via-transparent to-bg/50" />

            {/* Play button */}
            <div className="absolute inset-0 flex flex-col items-center justify-center gap-5">
              <div className="w-16 h-16 md:w-20 md:h-20 rounded-full border-2 border-surface-border
                flex items-center justify-center
                group-hover:border-accent/50 group-hover:scale-110 transition-all duration-300
                group-hover:shadow-[0_0_30px_rgba(212,162,83,0.12)]">
                <svg
                  viewBox="0 0 24 24"
                  fill="currentColor"
                  className="w-6 h-6 md:w-7 md:h-7 text-text-muted group-hover:text-accent transition-colors duration-300 ml-1"
                >
                  <path d="M8 5v14l11-7z" />
                </svg>
              </div>
              <span className="font-display italic text-text-muted text-sm md:text-base group-hover:text-text-primary transition-colors duration-300">
                Live Demo Coming Soon
              </span>
            </div>
          </div>

          <p className="text-text-muted text-sm text-center mt-6 max-w-lg mx-auto">
            Two Blender instances connected to the same session — placing assets, moving objects,
            and syncing cameras in real time.
          </p>
        </SectionReveal>
      </div>
    </section>
  )
}
