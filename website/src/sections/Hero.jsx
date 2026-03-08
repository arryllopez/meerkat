import { motion } from 'framer-motion'

export default function Hero() {
  return (
    <section className="relative h-screen flex flex-col items-center justify-start pt-[8vh] px-6 overflow-hidden">
      {/* Background glow */}
      <div
        className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[400px] md:w-[900px] md:h-[600px] rounded-full pointer-events-none"
        style={{
          background: 'radial-gradient(ellipse at center, rgba(212, 162, 83, 0.12) 0%, rgba(212, 162, 83, 0.04) 40%, transparent 70%)',
          animation: 'glow-pulse 5s ease-in-out infinite',
        }}
      />

      {/* Subtle grid lines */}
      <div
        className="absolute inset-0 pointer-events-none opacity-[0.03]"
        style={{
          backgroundImage: `linear-gradient(rgba(0,0,0,0.06) 1px, transparent 1px),
            linear-gradient(90deg, rgba(0,0,0,0.06) 1px, transparent 1px)`,
          backgroundSize: '60px 60px',
        }}
      />

      <motion.div
        initial={{ opacity: 0, y: 30 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.9, ease: [0.22, 1, 0.36, 1] }}
        className="relative z-10 text-center flex flex-col items-center flex-1 min-h-0 w-full pb-[10vh]"
      >
        <h1
          className="font-display font-bold tracking-tight leading-[0.9] mb-3 text-black"
          style={{ fontSize: 'clamp(4rem, 12vw, 9rem)', letterSpacing: '-0.04em' }}
        >
          Meerkat
        </h1>

        <motion.h2
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.2, duration: 0.8 }}
          className="font-display font-normal text-black mb-10"
          style={{ fontSize: 'clamp(1.1rem, 2.5vw, 1.5rem)' }}
        >
          Real-Time Collaborative Scene Layout for Blender
        </motion.h2>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4, duration: 0.8 }}
          className="w-[90vw] max-w-[1200px] mx-auto flex-1 min-h-0"
        >
          <div className="relative h-full rounded-xl bg-surface border border-surface-border group cursor-pointer
            hover:border-accent/30 transition-all duration-500 hover:shadow-[0_0_40px_rgba(184,137,63,0.08)]">
            <div className="absolute inset-0 rounded-xl overflow-hidden">
              <div className="absolute inset-0 bg-gradient-to-b from-transparent via-transparent to-bg/50" />
            </div>
            <div className="absolute inset-0 flex flex-col items-center justify-center gap-4">
              <div className="w-14 h-14 md:w-16 md:h-16 rounded-full border-2 border-surface-border
                flex items-center justify-center
                group-hover:border-accent/50 group-hover:scale-110 transition-all duration-300
                group-hover:shadow-[0_0_30px_rgba(184,137,63,0.12)]">
                <svg
                  viewBox="0 0 24 24"
                  fill="currentColor"
                  className="w-5 h-5 md:w-6 md:h-6 text-text-muted group-hover:text-accent transition-colors duration-300 ml-0.5"
                >
                  <path d="M8 5v14l11-7z" />
                </svg>
              </div>
              <span className="font-display italic text-text-muted text-sm group-hover:text-text-primary transition-colors duration-300">
                Demo Coming Soon
              </span>
            </div>
            {/* Meerkat peeking from the right */}
            <img
              src="/hero-preview.png"
              alt="Meerkat peeking"
              className="absolute -top-14 -right-30 h-[45%] w-auto pointer-events-none select-none drop-shadow-lg z-10"
            />
          </div>
        </motion.div>
      </motion.div>
    </section>
  )
}
