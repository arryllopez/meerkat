import { useRef, useMemo } from 'react'
import { Canvas, useFrame } from '@react-three/fiber'
import { useGLTF, Html } from '@react-three/drei'

function Suzanne(props) {
  const { nodes, materials } = useGLTF('/suzanne_skin_material_test.glb')
  return (
    <group {...props} dispose={null}>
      <group position={[0, -0.011, -0.005]} rotation={[-2.049, 0, 0]} scale={0.319}>
        <mesh
          geometry={nodes.Suzanne_0.geometry}
          material={materials.Skin}
        />
      </group>
    </group>
  )
}

function BouncingCursor({ name, color, position, seed = 1 }) {
  const groupRef = useRef()
  const base = useMemo(() => position, [position])

  useFrame(({ clock }) => {
    const t = clock.getElapsedTime()
    groupRef.current.position.x = base[0] + Math.sin(t * 0.4 + seed * 2) * 0.15
    groupRef.current.position.y = base[1] + Math.sin(t * 0.5 + seed * 1.3) * 0.1
    groupRef.current.position.z = base[2] + Math.cos(t * 0.35 + seed * 0.8) * 0.12
  })

  return (
    <group ref={groupRef} position={position}>
      <Html center style={{ pointerEvents: 'none' }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '2px' }}>
          <svg width="14" height="17" viewBox="0 0 14 17" fill="none">
            <path
              d="M1 1L1 12.5L3.8 9.5L7 15L9 14L5.8 8.5L10 8.5L1 1Z"
              fill={color}
              stroke="white"
              strokeWidth="1"
              strokeLinejoin="round"
            />
          </svg>
          <span
            style={{
              background: color,
              color: 'white',
              fontSize: '9px',
              fontFamily: 'Inter, sans-serif',
              fontWeight: 600,
              padding: '1px 5px',
              borderRadius: '3px',
              whiteSpace: 'nowrap',
              marginLeft: '8px',
              lineHeight: '14px',
            }}
          >
            {name}
          </span>
        </div>
      </Html>
    </group>
  )
}

useGLTF.preload('/suzanne_skin_material_test.glb')

export default function SuzanneScene() {
  return (
    <Canvas
      camera={{ position: [0, 0, 2.8], fov: 30 }}
      style={{ background: 'transparent', width: '100%', height: '100%' }}
      dpr={1}
      resize={{ scroll: false, debounce: { scroll: 0, resize: 0 } }}
      gl={{ alpha: true, antialias: true }}
    >
      <ambientLight intensity={0.9} />
      <directionalLight position={[5, 8, 5]} intensity={0.5} />

      <Suzanne position={[0, 0, 0]} rotation={[0.3, 0.3, 0]} scale={1.8} />

      <BouncingCursor name="Michael" color="#E74C3C" position={[0.5, 0.4, 0.4]} seed={3} />
      <BouncingCursor name="Lincoln" color="#F39C12" position={[-0.3, -0.2, 0.5]} seed={4} />
    </Canvas>
  )
}
