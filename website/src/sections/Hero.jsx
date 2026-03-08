import { useState, useEffect, useRef, lazy, Suspense } from 'react'
import { motion } from 'framer-motion'

const SuzanneScene = lazy(() => import('../components/SuzanneScene'))

const BLENDER = 'Blender'
const BLENDER_COLORS = ['#E87D0D', '#4A90D9', '#6C5CE7', '#2ECC71', '#D94A8C', '#E74C3C', '#F39C12']
const LETTER_DURATION = 300

// Cursor SVG component
function CursorIcon({ color }) {
  return (
    <svg width="14" height="17" viewBox="0 0 14 17" fill="none">
      <path
        d="M1 1L1 12.5L3.8 9.5L7 15L9 14L5.8 8.5L10 8.5L1 1Z"
        fill={color}
        stroke="white"
        strokeWidth="1"
        strokeLinejoin="round"
      />
    </svg>
  )
}

// Scattered shapes with cursors actively manipulating them
const floatingShapes = [
  {
    // Top-right: sphere being scaled
    id: 'sphere',
    cursor: { name: 'Jamie', color: '#6C5CE7' },
    position: { top: '15%', right: '8%' },
    shape: (
      <svg width="85" height="85" viewBox="0 0 50 50" fill="none">
        <circle cx="25" cy="25" r="22" stroke="#c0c0c0" strokeWidth="1.5" fill="rgba(108,92,231,0.06)" />
        <ellipse cx="25" cy="25" rx="22" ry="8" stroke="#c0c0c0" strokeWidth="0.8" />
        <ellipse cx="25" cy="25" rx="8" ry="22" stroke="#c0c0c0" strokeWidth="0.8" />
      </svg>
    ),
    animation: {
      scale: [1, 1.3, 0.9, 1.15, 1],
      transition: { duration: 6, repeat: Infinity, ease: 'easeInOut' },
    },
    cursorOffset: { x: 65, y: 55 },
  },
  {
    // Bottom-left: Suzanne 3D model
    id: 'suzanne',
    is3D: true,
    position: { bottom: '8%', left: '5%' },
  },
  {
    // Bottom-right: cylinder being moved
    id: 'cylinder',
    cursor: { name: 'Sam', color: '#2ECC71' },
    position: { bottom: '18%', right: '6%' },
    shape: (
      <svg width="75" height="95" viewBox="0 0 44 56" fill="none">
        <ellipse cx="22" cy="10" rx="20" ry="8" stroke="#c0c0c0" strokeWidth="1.5" fill="rgba(46,204,113,0.06)" />
        <line x1="2" y1="10" x2="2" y2="46" stroke="#c0c0c0" strokeWidth="1.5" />
        <line x1="42" y1="10" x2="42" y2="46" stroke="#c0c0c0" strokeWidth="1.5" />
        <ellipse cx="22" cy="46" rx="20" ry="8" stroke="#c0c0c0" strokeWidth="1.5" fill="rgba(46,204,113,0.06)" />
      </svg>
    ),
    animation: {
      x: [0, -20, 10, -15, 0],
      y: [0, 15, -10, 20, 0],
      transition: { duration: 9, repeat: Infinity, ease: 'easeInOut' },
    },
    cursorOffset: { x: 55, y: -8 },
  },
  {
    // Mid-right: torus/donut being rotated
    id: 'torus',
    cursor: { name: 'Mika', color: '#D94A8C' },
    position: { top: '48%', right: '5%' },
    shape: (
      <svg width="85" height="58" viewBox="0 0 50 34" fill="none">
        <ellipse cx="25" cy="17" rx="23" ry="15" stroke="#c0c0c0" strokeWidth="1.5" fill="none" />
        <ellipse cx="25" cy="17" rx="10" ry="6" stroke="#c0c0c0" strokeWidth="1.5" fill="rgba(217,74,140,0.06)" />
      </svg>
    ),
    animation: {
      rotate: [0, -12, 8, -15, 12, 0],
      scaleX: [1, 1.1, 0.95, 1.05, 1],
      transition: { duration: 10, repeat: Infinity, ease: 'easeInOut' },
    },
    cursorOffset: { x: 70, y: 35 },
  },
]

export default function Hero() {
  const [revealedLetters, setRevealedLetters] = useState(0)
  const [activeCursor, setActiveCursor] = useState(-1)
  const [blenderDone, setBlenderDone] = useState(false)
  const [showSubtitle, setShowSubtitle] = useState(false)
  const hasStarted = useRef(false)

  useEffect(() => {
    if (hasStarted.current) return
    hasStarted.current = true

    // Show subtitle immediately, then start typing "Blender" after a short delay
    setTimeout(() => setShowSubtitle(true), 300)

    BLENDER.split('').forEach((_, i) => {
      const delay = 600 + i * LETTER_DURATION
      setTimeout(() => setActiveCursor(i), delay)
      setTimeout(() => setRevealedLetters((prev) => Math.max(prev, i + 1)), delay + 150)
      setTimeout(() => setActiveCursor(-1), delay + LETTER_DURATION - 50)
    })

    const totalTime = 600 + BLENDER.length * LETTER_DURATION
    setTimeout(() => setBlenderDone(true), totalTime)
  }, [])

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

      {/* Ornn with cursors */}
      <motion.div
        initial={{ opacity: 0, scale: 0.8 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 1, delay: 0.5, ease: 'easeOut' }}
        className="absolute hidden md:block pointer-events-none"
        style={{ top: '8%', left: '0%', width: 'clamp(250px, 26vw, 400px)', height: 'clamp(250px, 26vw, 400px)' }}
      >
        <motion.div
          animate={{ y: [0, -8, 0] }}
          transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut' }}
          className="relative w-full h-full flex items-center justify-center"
        >
          <img src="/Adobe Express - file.png" alt="Ornn" style={{ width: 'clamp(180px, 18vw, 280px)' }} className="h-auto drop-shadow-lg" />

          {/* Alex cursor */}
          <motion.div
            className="absolute"
            style={{ top: '25%', right: '10%' }}
            animate={{ x: [0, 3, -2, 4, 0], y: [0, -3, 2, -1, 0] }}
            transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut' }}
          >
            <CursorIcon color="#4A90D9" />
            <span
              className="block mt-0.5 text-white text-[9px] font-semibold font-heading rounded px-1 py-px whitespace-nowrap"
              style={{ background: '#4A90D9', marginLeft: 8 }}
            >
              Alex
            </span>
          </motion.div>

          {/* Jamie cursor */}
          <motion.div
            className="absolute"
            style={{ bottom: '30%', left: '15%' }}
            animate={{ x: [0, -3, 2, -4, 0], y: [0, 2, -3, 1, 0] }}
            transition={{ duration: 3.5, repeat: Infinity, ease: 'easeInOut' }}
          >
            <CursorIcon color="#6C5CE7" />
            <span
              className="block mt-0.5 text-white text-[9px] font-semibold font-heading rounded px-1 py-px whitespace-nowrap"
              style={{ background: '#6C5CE7', marginLeft: 8 }}
            >
              Jamie
            </span>
          </motion.div>
        </motion.div>
      </motion.div>

      {/* Scattered floating shapes with cursors */}
      {floatingShapes.map((item, i) =>
        item.is3D ? (
          <motion.div
            key={item.id}
            initial={{ opacity: 0, scale: 0.5 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: 0.5 + i * 0.15, duration: 0.6, ease: 'easeOut' }}
            className="absolute pointer-events-none hidden md:block"
            style={{ ...item.position, width: 'clamp(260px, 28vw, 420px)', height: 'clamp(240px, 25vw, 380px)' }}
          >
            <Suspense fallback={null}>
              <SuzanneScene />
            </Suspense>
          </motion.div>
        ) : (
          <motion.div
            key={item.id}
            initial={{ opacity: 0, scale: 0.5 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ delay: 0.5 + i * 0.15, duration: 0.6, ease: 'easeOut' }}
            className="absolute pointer-events-none hidden md:block"
            style={item.position}
          >
            {/* The shape itself, animated */}
            <motion.div animate={item.animation}>
              {item.shape}

              {/* Cursor + label following the shape */}
              <motion.div
                className="absolute"
                style={{ left: item.cursorOffset.x, top: item.cursorOffset.y }}
                animate={{
                  x: [0, 3, -2, 4, 0],
                  y: [0, -3, 2, -1, 0],
                }}
                transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut' }}
              >
                <CursorIcon color={item.cursor.color} />
                <span
                  className="block mt-0.5 text-white text-[9px] font-semibold font-heading rounded px-1 py-px whitespace-nowrap"
                  style={{ background: item.cursor.color, marginLeft: 8 }}
                >
                  {item.cursor.name}
                </span>
              </motion.div>
            </motion.div>
          </motion.div>
        )
      )}

      <div className="relative z-10 text-center flex flex-col items-center flex-1 min-h-0 w-full pb-[10vh]">
        {/* Static title */}
        <h1
          className="font-heading font-semibold tracking-tight leading-[0.9] mb-3 text-black"
          style={{ fontSize: 'clamp(4rem, 12vw, 9rem)', letterSpacing: '-0.04em' }}
        >
          Meerkat
        </h1>

        {/* Subtitle with cursor typing "Blender" */}
        <motion.h2
          initial={{ opacity: 0 }}
          animate={{ opacity: showSubtitle ? 1 : 0 }}
          transition={{ duration: 0.8 }}
          className="font-heading font-normal text-black mb-4"
          style={{ fontSize: 'clamp(1.1rem, 2.5vw, 1.5rem)' }}
        >
          Real-Time Collaborative Editing for{' '}
          <span className="relative inline-block">
            {BLENDER.split('').map((letter, i) => (
              <span key={i} className="relative inline-block">
                <span
                  className="transition-opacity duration-150 font-semibold"
                  style={{
                    opacity: i < revealedLetters ? 1 : 0,
                    color: BLENDER_COLORS[i % BLENDER_COLORS.length],
                  }}
                >
                  {letter}
                </span>

                {!blenderDone && activeCursor === i && (
                  <motion.span
                    initial={{ opacity: 0, scaleY: 0 }}
                    animate={{ opacity: 1, scaleY: 1 }}
                    className="absolute -right-px top-[0.1em] bottom-[0.1em] w-0.5 rounded-full"
                    style={{
                      backgroundColor: BLENDER_COLORS[i % BLENDER_COLORS.length],
                      animation: 'cursor-blink 0.6s ease-in-out infinite',
                    }}
                  />
                )}
              </span>
            ))}
          </span>
        </motion.h2>

        {/* Video placeholder */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: showSubtitle ? 1 : 0, y: showSubtitle ? 0 : 20 }}
          transition={{ delay: 0.2, duration: 0.8 }}
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
              <span className="font-heading italic text-text-muted text-sm group-hover:text-text-primary transition-colors duration-300">
                Demo Coming Soon
              </span>
            </div>
            <img
              src="/hero-preview.png"
              alt="Meerkat peeking"
              className="absolute -top-14 -right-32 h-[45%] w-auto pointer-events-none select-none drop-shadow-lg z-10"
            />
          </div>
        </motion.div>
      </div>
    </section>
  )
}
