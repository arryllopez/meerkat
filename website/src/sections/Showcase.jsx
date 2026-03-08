import SectionReveal from '../components/SectionReveal'
import ShowcaseCard, { WEEKS } from '../components/ShowcaseCard'

export default function Showcase() {
  return (
    <section className="py-20 md:py-28 px-6">
      <div className="max-w-4xl mx-auto">
        <SectionReveal>
          <span className="text-accent text-xs font-semibold tracking-[0.2em] uppercase font-body block mb-3">
            Build Log
          </span>
          <h2
            className="font-display font-bold text-text-primary mb-4"
            style={{ fontSize: 'clamp(1.75rem, 4vw, 2.75rem)' }}
          >
            Development Showcase
          </h2>
          <p className="text-text-muted max-w-lg mb-16 text-sm md:text-base">
            Eight weeks from first cargo new to live deployment. Each phase built on the last — server foundation, plugin skeleton, object sync, transforms, presence, and observability.
          </p>
        </SectionReveal>

        <div className="flex flex-col gap-14 md:gap-20">
          {WEEKS.map((_, i) => (
            <ShowcaseCard key={i} index={i} />
          ))}
        </div>
      </div>
    </section>
  )
}
