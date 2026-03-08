import { useRef, useEffect, useMemo } from 'react'
import { Canvas, useFrame } from '@react-three/fiber'
import { useGLTF, useAnimations, Html } from '@react-three/drei'

function Bird(props) {
  const group = useRef()
  const { nodes, materials, animations } = useGLTF('/bird_orange.glb')
  const { actions } = useAnimations(animations, group)

  useEffect(() => {
    const firstAction = Object.values(actions)[0]
    if (firstAction) firstAction.reset().fadeIn(0.5).play()
  }, [actions])

  return (
    <group ref={group} {...props} dispose={null}>
      <group name="Sketchfab_Scene">
        <group name="Sketchfab_model" rotation={[-Math.PI / 2, 0, 0]} scale={46.683}>
          <group
            name="bfb1ea86655f4c4ab4c6cbbb449cedf4fbx"
            rotation={[Math.PI / 2, 0, 0]}
            scale={0.01}>
            <group name="Object_2">
              <group name="RootNode">
                <group name="BirdOrange_all">
                  <group name="Main" position={[-0.083, 0, 0.451]} rotation={[0, -0.074, 0]}>
                    <group name="Object_6">
                      <primitive object={nodes._rootJoint} />
                      <skinnedMesh
                        name="Object_51"
                        geometry={nodes.Object_51.geometry}
                        material={materials.BirdOrange_LMB}
                        skeleton={nodes.Object_51.skeleton}
                      />
                      <group name="Object_50" />
                    </group>
                  </group>
                  <group name="Geometry">
                    <group name="BirdOrange" />
                  </group>
                </group>
              </group>
            </group>
          </group>
        </group>
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

useGLTF.preload('/bird_orange.glb')

export default function CottageScene() {
  return (
    <Canvas
      camera={{ position: [4, 3, 5], fov: 30 }}
      style={{ background: 'transparent', width: '100%', height: '100%' }}
      dpr={1}
      resize={{ scroll: false, debounce: { scroll: 0, resize: 0 } }}
      gl={{ alpha: true, antialias: true }}
    >
      <ambientLight intensity={0.9} />
      <directionalLight position={[5, 8, 5]} intensity={0.5} />

      <Bird position={[0, -0.8, 0]} rotation={[0, -0.4, 0]} scale={1.4} />

      <BouncingCursor name="Alex" color="#4A90D9" position={[1.2, 1.2, 0.8]} seed={1} />
      <BouncingCursor name="Jamie" color="#6C5CE7" position={[0.3, 0.5, 1.2]} seed={2} />
    </Canvas>
  )
}
