import Hero from './sections/Hero'
import Showcase from './sections/Showcase'
import Demo from './sections/Demo'
import Features from './sections/Features'
import Footer from './sections/Footer'

export default function App() {
  return (
    <main className="bg-bg min-h-screen">
      <Hero />
      <Showcase />
      <Demo />
      <Features />
      <Footer />
    </main>
  )
}
