import SectionReveal from '../components/SectionReveal'
import FeatureCard, { FEATURES } from '../components/FeatureCard'

export default function Features() {
  return (
    <section id="features" className="py-20 md:py-28 px-6">
      <div className="max-w-4xl mx-auto">
        <SectionReveal>
          <span className="text-accent text-xs font-semibold tracking-[0.2em] uppercase font-body block mb-3">
            Capabilities
          </span>
          <h2
            className="font-display font-bold text-text-primary mb-14"
            style={{ fontSize: 'clamp(1.75rem, 4vw, 2.75rem)' }}
          >
            Feature Highlights
          </h2>
        </SectionReveal>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          {FEATURES.map((_, i) => (
            <FeatureCard key={i} index={i} />
          ))}
        </div>
      </div>
    </section>
  )
}
