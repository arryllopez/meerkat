import { useRef } from 'react'
import { motion, useScroll, useTransform } from 'framer-motion'

const pictures = [
  // Center hero image — scales slowest
  { scaleRange: [1, 4], width: '25vw', height: '25vh', top: 0, left: 0 },
  // Surrounding images — scale faster so they fly outward
  { scaleRange: [1, 5], width: '25vw', height: '20vh', top: -25, left: 20 },
  { scaleRange: [1, 6], width: '22vw', height: '25vh', top: -8, left: -30 },
  { scaleRange: [1, 5], width: '23vw', height: '18vh', top: 23, left: 10 },
  { scaleRange: [1, 6], width: '20vw', height: '20vh', top: 25, left: -22 },
  { scaleRange: [1, 8], width: '18vw', height: '16vh', top: 18, left: 32 },
  { scaleRange: [1, 9], width: '15vw', height: '14vh', top: -30, left: -20 },
]

const colors = ['#e8e0d4', '#d4cfc7', '#ddd8cf', '#e2dbd1', '#d9d3c9', '#ded7cc', '#e5ddd2']

function ZoomImage({ scrollYProgress, pic, index }) {
  const scale = useTransform(scrollYProgress, [0, 1], pic.scaleRange)

  return (
    <motion.div
      style={{ scale }}
      className="absolute w-full h-full flex items-center justify-center"
    >
      <div
        className="absolute rounded-lg overflow-hidden"
        style={{
          width: pic.width,
          height: pic.height,
          top: `calc(50% + ${pic.top}vh)`,
          left: `calc(50% + ${pic.left}vw)`,
          transform: 'translate(-50%, -50%)',
          backgroundColor: colors[index],
        }}
      >
        <div className="absolute inset-0 flex items-center justify-center text-text-muted/40 font-display text-sm italic">
          {index === 0 ? 'Hero Image' : `Image ${index + 1}`}
        </div>
      </div>
    </motion.div>
  )
}

export default function ZoomParallax() {
  const container = useRef(null)
  const { scrollYProgress } = useScroll({
    target: container,
    offset: ['start start', 'end end'],
  })

  return (
    <section id="showcase" ref={container} className="relative" style={{ height: '300vh' }}>
      <div className="sticky top-0 h-screen overflow-hidden flex items-center justify-center">
        <h2 className="absolute top-12 left-1/2 -translate-x-1/2 z-10 font-heading text-3xl md:text-5xl font-semibold text-black text-center whitespace-nowrap px-6">
          Collaborate Inside of Blender in Realtime
        </h2>
        {pictures.map((pic, i) => (
          <ZoomImage key={i} scrollYProgress={scrollYProgress} pic={pic} index={i} />
        ))}
      </div>
    </section>
  )
}
