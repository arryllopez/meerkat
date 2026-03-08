import Navbar from './components/Navbar'
import Hero from './sections/Hero'
import ZoomParallax from './sections/ZoomParallax'
import Installation from './sections/Installation'
import Footer from './sections/Footer'

export default function App() {
  return (
    <main className="bg-bg min-h-screen">
      <Navbar />
      <Hero />
      <ZoomParallax />
      <Installation />
      <Footer />
    </main>
  )
}
