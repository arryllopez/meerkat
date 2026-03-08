import SectionReveal from '../components/SectionReveal'

const steps = [
  {
    number: '01',
    title: 'Download the Addon',
    description: 'Grab the latest meerkat-blender-addon.zip from the GitHub releases page.',
  },
  {
    number: '02',
    title: 'Install in Blender',
    description: 'Edit → Preferences → Add-ons → Install → select the .zip file. Enable the Meerkat checkbox.',
  },
  {
    number: '03',
    title: 'Connect & Collaborate',
    description: 'Open the Meerkat panel in the 3D viewport sidebar (N), enter a room name, and click Connect.',
  },
]

export default function Installation() {
  return (
    <section id="installation" className="py-20 md:py-28 px-6">
      <div className="max-w-4xl mx-auto">
        <SectionReveal>
          <span className="text-accent text-xs font-semibold tracking-[0.2em] uppercase font-body block mb-3">
            Get Started
          </span>
          <h2 className="font-heading text-3xl md:text-4xl font-semibold text-black mb-14">
            Installation
          </h2>
        </SectionReveal>

        <div className="grid gap-8 md:grid-cols-3">
          {steps.map((step) => (
            <SectionReveal key={step.number}>
              <div className="p-6 rounded-xl border border-surface-border bg-surface hover:border-accent/30 transition-all duration-300">
                <span className="font-heading text-4xl font-semibold text-accent/30 block mb-3">
                  {step.number}
                </span>
                <h3 className="font-heading text-lg font-normal text-black mb-2">
                  {step.title}
                </h3>
                <p className="text-text-muted text-sm leading-relaxed font-body">
                  {step.description}
                </p>
              </div>
            </SectionReveal>
          ))}
        </div>
      </div>
    </section>
  )
}
