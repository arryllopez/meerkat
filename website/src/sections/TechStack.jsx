import SectionReveal from '../components/SectionReveal'
import Pill from '../components/Pill'

const TECH = ['Rust', 'Tokio', 'Axum', 'WebSocket', 'Python', 'Blender API']

export default function TechStack() {
  return (
    <section className="py-12 md:py-16 px-6">
      <SectionReveal>
        <div className="max-w-3xl mx-auto flex flex-wrap gap-3 justify-center">
          {TECH.map((t) => (
            <Pill key={t} label={t} />
          ))}
        </div>
      </SectionReveal>
    </section>
  )
}
