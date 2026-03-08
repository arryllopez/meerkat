export default function Footer() {
  return (
    <footer className="border-t border-surface-border mt-10">
      <div className="max-w-4xl mx-auto px-6 py-12 md:py-16 flex flex-col md:flex-row items-center justify-between gap-6">
        <div className="font-heading text-lg font-semibold text-text-primary tracking-tight">
          Meerkat
        </div>

        <div className="flex gap-8 text-text-muted text-sm">
          <a
            href="https://github.com/arryllopez/meerkat"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-accent transition-colors duration-200"
          >
            GitHub
          </a>
          <a href="#" className="hover:text-accent transition-colors duration-200">
            Docs
          </a>
          <a href="#" className="hover:text-accent transition-colors duration-200">
            Contact
          </a>
        </div>

        <span className="text-text-muted text-xs">
          &copy; {new Date().getFullYear()} Meerkat
        </span>
      </div>
    </footer>
  )
}
