export default function BlenderViewport() {
  return (
    <div className="absolute inset-0 overflow-hidden pointer-events-none" style={{ opacity: 0.35 }}>
      {/* Viewport background */}
      <div className="absolute inset-0" style={{ background: 'linear-gradient(180deg, #e8e8e8 0%, #d4d4d4 100%)' }} />

      {/* Perspective grid */}
      <svg className="absolute inset-0 w-full h-full" preserveAspectRatio="none" viewBox="0 0 1200 800">
        {/* Horizon line */}
        <line x1="0" y1="380" x2="1200" y2="380" stroke="#b0b0b0" strokeWidth="0.5" />

        {/* Vanishing point grid lines - horizontal */}
        {Array.from({ length: 20 }, (_, i) => {
          const y = 380 + (i + 1) * 22
          const spread = (i + 1) * 0.08
          return (
            <line
              key={`h-${i}`}
              x1={600 - 600 * (1 + spread)}
              y1={y}
              x2={600 + 600 * (1 + spread)}
              y2={y}
              stroke="#b8b8b8"
              strokeWidth={i % 5 === 4 ? '0.6' : '0.3'}
            />
          )
        })}

        {/* Vanishing point grid lines - vertical converging */}
        {Array.from({ length: 30 }, (_, i) => {
          const x = (i - 15) * 80
          return (
            <line
              key={`v-${i}`}
              x1={600}
              y1={380}
              x2={600 + x}
              y2={800}
              stroke="#b8b8b8"
              strokeWidth={i % 5 === 0 ? '0.6' : '0.3'}
            />
          )
        })}

        {/* Center axis lines */}
        <line x1="600" y1="380" x2="600" y2="800" stroke="#c45454" strokeWidth="0.8" opacity="0.4" />
        <line x1="0" y1="380" x2="1200" y2="380" stroke="#54c454" strokeWidth="0.8" opacity="0.4" />
      </svg>

      {/* 3D cursor crosshair */}
      <div className="absolute" style={{ top: '47%', left: '50%', transform: 'translate(-50%, -50%)' }}>
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" opacity="0.3">
          <circle cx="12" cy="12" r="5" stroke="#333" strokeWidth="1" fill="none" strokeDasharray="2 2" />
          <line x1="12" y1="0" x2="12" y2="8" stroke="#333" strokeWidth="0.8" />
          <line x1="12" y1="16" x2="12" y2="24" stroke="#333" strokeWidth="0.8" />
          <line x1="0" y1="12" x2="8" y2="12" stroke="#333" strokeWidth="0.8" />
          <line x1="16" y1="12" x2="24" y2="12" stroke="#333" strokeWidth="0.8" />
        </svg>
      </div>

      {/* Axis gizmo - top right corner */}
      <div className="absolute" style={{ top: '20px', right: '30px' }}>
        <svg width="60" height="60" viewBox="0 0 60 60" fill="none" opacity="0.5">
          {/* Z axis - up (blue) */}
          <line x1="30" y1="30" x2="30" y2="8" stroke="#4A7ADB" strokeWidth="1.5" />
          <circle cx="30" cy="6" r="4" fill="#4A7ADB" />
          <text x="30" y="8" fill="white" fontSize="6" textAnchor="middle" dominantBaseline="middle" fontFamily="Inter, sans-serif" fontWeight="600">Z</text>

          {/* X axis - right (red) */}
          <line x1="30" y1="30" x2="52" y2="38" stroke="#DB4A4A" strokeWidth="1.5" />
          <circle cx="54" cy="39" r="4" fill="#DB4A4A" />
          <text x="54" y="41" fill="white" fontSize="6" textAnchor="middle" dominantBaseline="middle" fontFamily="Inter, sans-serif" fontWeight="600">X</text>

          {/* Y axis - left-down (green) */}
          <line x1="30" y1="30" x2="12" y2="42" stroke="#4ADB5C" strokeWidth="1.5" />
          <circle cx="10" cy="43" r="4" fill="#4ADB5C" />
          <text x="10" y="45" fill="white" fontSize="6" textAnchor="middle" dominantBaseline="middle" fontFamily="Inter, sans-serif" fontWeight="600">Y</text>

          {/* Center dot */}
          <circle cx="30" cy="30" r="2" fill="#888" />
        </svg>
      </div>

      {/* Top header bar */}
      <div className="absolute top-0 left-0 right-0 h-6" style={{ background: 'rgba(180,180,180,0.4)' }}>
        <div className="flex items-center h-full px-3 gap-4">
          <span style={{ fontSize: '9px', color: '#666', fontFamily: 'Inter, sans-serif' }}>Object Mode</span>
          <span style={{ fontSize: '9px', color: '#888', fontFamily: 'Inter, sans-serif' }}>|</span>
          <span style={{ fontSize: '9px', color: '#666', fontFamily: 'Inter, sans-serif' }}>View</span>
          <span style={{ fontSize: '9px', color: '#666', fontFamily: 'Inter, sans-serif' }}>Select</span>
          <span style={{ fontSize: '9px', color: '#666', fontFamily: 'Inter, sans-serif' }}>Add</span>
          <span style={{ fontSize: '9px', color: '#666', fontFamily: 'Inter, sans-serif' }}>Object</span>
        </div>
      </div>

      {/* Bottom info bar */}
      <div className="absolute bottom-0 left-0 right-0 h-5" style={{ background: 'rgba(180,180,180,0.3)' }}>
        <div className="flex items-center justify-between h-full px-3">
          <span style={{ fontSize: '8px', color: '#777', fontFamily: 'Inter, sans-serif' }}>Verts: 2,418 | Faces: 2,304 | Tris: 4,608</span>
          <span style={{ fontSize: '8px', color: '#777', fontFamily: 'Inter, sans-serif' }}>Collection | Scene</span>
        </div>
      </div>

      {/* Left toolbar hint */}
      <div className="absolute left-0 top-8 bottom-6 w-8" style={{ background: 'rgba(180,180,180,0.2)' }}>
        <div className="flex flex-col items-center pt-2 gap-2">
          {['⊞', '↔', '⟳', '⤢', '◎'].map((icon, i) => (
            <div key={i} className="w-5 h-5 flex items-center justify-center rounded" style={{ fontSize: '10px', color: '#888' }}>
              {icon}
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
