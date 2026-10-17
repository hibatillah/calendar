import {
  ArrowDown,
  ArrowRight,
  ArrowUpRight,
  CalendarDays,
  Check,
  Download,
  Laptop,
  Menu,
  Plus,
  Smartphone,
  X,
} from "lucide-react";
import { useState } from "react";
import "../landing.css";
import { PhonePreview, ProductPreview, useInteractiveReady } from "./product-previews";
import { Button } from "./ui/button";

const repository = "https://github.com/hibatillah/calendar";

export function MarketingNav() {
  const interactiveReady = useInteractiveReady();
  const [menuOpen, setMenuOpen] = useState(false);

  return (
    <header className="marketing-nav">
      <a href="/" className="calendar-wordmark" aria-label="Calendar home">
        <CalendarDays size={25} strokeWidth={1.8} />
        Calendar
      </a>
      <nav
        aria-label="Main navigation"
        className={menuOpen ? "marketing-links is-open" : "marketing-links"}
      >
        <a href="/#features" onClick={() => setMenuOpen(false)}>
          Features
        </a>
        <a href="/download" onClick={() => setMenuOpen(false)}>
          Download
        </a>
        <a href={`${repository}/releases`}>
          Changelog <ArrowUpRight size={12} />
        </a>
      </nav>
      <div className="marketing-nav-actions">
        <a href="/app" className="open-app-link">
          Open calendar <ArrowUpRight size={14} />
        </a>
        <Button asChild className="marketing-button">
          <a href="/download">Get Calendar</a>
        </Button>
        <button
          className="mobile-menu-button"
          disabled={!interactiveReady}
          aria-label={menuOpen ? "Close navigation" : "Open navigation"}
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen(!menuOpen)}
        >
          {menuOpen ? <X /> : <Menu />}
        </button>
      </div>
    </header>
  );
}

export function DownloadSection() {
  return (
    <section className="download-section" id="download">
      <span className="section-eyebrow">A PLACE FOR EVERY DAY</span>
      <h2>Make yourself some room.</h2>
      <p>Calendar is in preview. Pick your platform and follow along.</p>
      <div className="platform-grid">
        <article>
          <Laptop />
          <h3>Windows</h3>
          <span className="platform-status available">Developer preview</span>
          <p>
            Native desktop app. Current builds require a local backend and Google setup.
          </p>
          <Button asChild className="marketing-button">
            <a href={`${repository}/actions/workflows/ci.yml`}>
              View preview builds <ArrowUpRight />
            </a>
          </Button>
        </article>
        <article>
          <Laptop />
          <h3>macOS</h3>
          <span className="platform-status">In development</span>
          <p>A native desktop experience with your upcoming day in the menu bar.</p>
          <span className="unavailable-download">No public download yet</span>
        </article>
        <article>
          <Smartphone />
          <h3>Android</h3>
          <span className="platform-status">In development</span>
          <p>A month at a glance, with a simple agenda for the day you choose.</p>
          <a className="platform-text-link" href={`${repository}/pull/3`}>
            Follow mobile progress <ArrowUpRight size={14} />
          </a>
        </article>
        <article>
          <Smartphone />
          <h3>iOS</h3>
          <span className="platform-status">In development</span>
          <p>
            Native date and time controls. TestFlight access will come after device
            testing.
          </p>
          <span className="unavailable-download">TestFlight not available yet</span>
        </article>
      </div>
      <p className="download-note">
        Preview artifacts may require a GitHub account and expire. Signed public
        installers will appear here when ready.
      </p>
    </section>
  );
}

export function MarketingFooter() {
  return (
    <footer className="marketing-footer">
      <a className="calendar-wordmark" href="/">
        <CalendarDays size={21} />
        Calendar
      </a>
      <span>A little room for your time.</span>
      <nav aria-label="Footer navigation">
        <a href="/privacy">Preview privacy</a>
        <a href={`${repository}/issues`}>Feedback</a>
        <a href={repository}>
          GitHub <ArrowUpRight size={13} />
        </a>
      </nav>
      <small>© 2026 Calendar · Open source under MIT</small>
    </footer>
  );
}

export function LandingPage() {
  return (
    <main className="marketing-page">
      <a href="#features" className="marketing-skip-link">
        Skip to features
      </a>
      <section className="landing-hero">
        <MarketingNav />
        <div className="hero-copy">
          <span className="preview-badge">
            <i /> A new space for your Google calendars <ArrowRight size={12} />
          </span>
          <h1>
            Your week,
            <br />
            <span>clearly.</span>
          </h1>
          <p>
            Work, life, and the time in between.
            <br />
            All your Google calendars in one calm space.
          </p>
          <div className="hero-actions">
            <Button asChild className="marketing-button hero-download">
              <a href="/download">
                <Download size={17} />
                Get the Windows preview
              </a>
            </Button>
            <a href="/app" className="hero-secondary">
              Open calendar <ArrowUpRight size={15} />
            </a>
          </div>
          <span className="hero-note">Built for desktop. Growing with you.</span>
        </div>
        <div className="hero-product">
          <ProductPreview />
        </div>
        <div className="hero-preview-caption">
          <span>Example calendar · click an event to explore</span>
          <span>WEEK / MONTH</span>
        </div>
      </section>
      <section className="landing-intro" id="features">
        <span className="section-eyebrow">LESS BETWEEN YOU AND YOUR DAY</span>
        <h2>
          See what matters.
          <br />
          <span>Leave room for the rest.</span>
        </h2>
        <p>
          Some days need a plan. Others need a little space.
          <br />
          Calendar gives you a clear view of both.
        </p>
      </section>
      <section className="feature-grid" aria-label="Calendar features">
        <article className="feature-card">
          <span className="feature-number">01 / A CLEARER VIEW</span>
          <div className="feature-week-art" aria-hidden="true">
            {["M", "T", "W", "T", "F"].map((day, index) => (
              <div key={index}>
                <span>{day}</span>
                <i
                  style={{
                    top: 57 + (index % 3) * 27,
                    height: 45 + (index % 2) * 30,
                    background: ["#8faebc", "#d7a294", "#a6b59c"][index % 3],
                  }}
                />
              </div>
            ))}
          </div>
          <h3>
            Your whole week.
            <br />
            Nothing in the way.
          </h3>
          <p>Switch between a spacious week and the bigger picture of a month.</p>
        </article>
        <article className="feature-card">
          <span className="feature-number">02 / PLANS CAN CHANGE</span>
          <div className="feature-move-art" aria-hidden="true">
            <span className="ghost-event">Design time</span>
            <ArrowDown size={24} />
            <span className="moved-event">
              <strong>Design time</strong>
              <small>14:00 — 15:30</small>
              <i />
            </span>
          </div>
          <h3>
            Make time.
            <br />
            Then move it.
          </h3>
          <p>Drag an event into place. Pull its edge when you need a little longer.</p>
        </article>
        <article className="feature-card">
          <span className="feature-number">03 / ALL TOGETHER</span>
          <div className="feature-accounts-art" aria-hidden="true">
            <div>
              <i className="account-avatar">a</i>
              <span>
                alex@example.com<small>Personal</small>
              </span>
              <Check size={15} />
            </div>
            <div>
              <i className="account-avatar work-avatar">a</i>
              <span>
                alex@studio.example<small>Work · Team</small>
              </span>
              <Check size={15} />
            </div>
          </div>
          <h3>
            Different calendars.
            <br />
            One place.
          </h3>
          <p>
            Keep work and personal calendars together, with colors that feel familiar.
          </p>
        </article>
      </section>
      <section className="mobile-feature">
        <div className="mobile-feature-copy">
          <span className="section-eyebrow">THE BIG PICTURE. THE LITTLE PLANS.</span>
          <h2>
            A month in view.
            <br />A day in hand.
          </h2>
          <p>
            See the shape of your month. Tap a date for a simple list of what’s ahead.
            No squeezing an entire desktop onto your phone.
          </p>
          <ul>
            <li>
              <Check size={16} />
              Color-coded event markers
            </li>
            <li>
              <Check size={16} />A daily agenda, in time order
            </li>
            <li>
              <Check size={16} />
              Native date and time controls
            </li>
          </ul>
          <a className="marketing-inline-link" href="/download">
            See mobile availability <ArrowRight size={16} />
          </a>
          <span className="feature-availability">Mobile is in development.</span>
        </div>
        <div className="phone-stage">
          <span className="phone-orbit" />
          <PhonePreview />
        </div>
      </section>
      <section className="tray-feature">
        <div
          className="tray-preview"
          data-theme="light"
          aria-label="Example upcoming schedule"
        >
          <div className="tray-system-bar">
            <span>Wednesday, October 14</span>
            <CalendarDays size={16} />
            <span>09:22</span>
          </div>
          <div className="tray-popover">
            <span className="tray-popover-notch" />
            <div className="tray-popover-title">
              <strong>Coming up</strong>
              <span>Today</span>
            </div>
            <p>Your next little part of the day.</p>
            <div className="tray-event">
              <i />
              <div>
                <strong>Coffee with Maya</strong>
                <span>09:30 – 10:30</span>
              </div>
              <span className="tray-countdown">in 8 min</span>
            </div>
            <div className="tray-event">
              <i />
              <div>
                <strong>Lunch, unhurried</strong>
                <span>12:00 – 13:00</span>
              </div>
            </div>
            <div className="tray-open-label">
              Open calendar <ArrowUpRight size={14} />
            </div>
          </div>
        </div>
        <div className="tray-copy">
          <span className="section-eyebrow">RIGHT THERE WHEN YOU NEED IT</span>
          <h2>
            Your next plan.
            <br />
            Just a glance away.
          </h2>
          <p>
            Keep your upcoming schedule close in the Windows tray. macOS menu-bar
            support is being prepared, too.
          </p>
          <p className="small-feature-note">
            Optional desktop reminders are in development. They require Calendar to stay
            running.
          </p>
        </div>
      </section>
      <section className="privacy-strip">
        <div>
          <span className="section-eyebrow">YOUR CALENDAR, YOUR CALL</span>
          <h2>
            Connected to Google.
            <br />
            Considerate of your space.
          </h2>
        </div>
        <div>
          <p>
            Choose which calendars you see. View previously loaded events offline.
            Disconnect an account when you’re done.
          </p>
          <a href="/privacy" className="marketing-inline-link">
            How the preview handles your data <ArrowUpRight size={16} />
          </a>
        </div>
      </section>
      <DownloadSection />
      <section className="faq-section">
        <h2>A few things to know.</h2>
        {[
          [
            "Which calendars can I connect?",
            "Calendar currently supports Google Calendar, including multiple Google accounts. Outlook and other providers are not supported yet.",
          ],
          [
            "Can I use Calendar offline?",
            "You can view calendar ranges already loaded on your device. Creating and editing events currently requires a connection.",
          ],
          [
            "Is this ready for everyone?",
            "Not yet. This is a developer preview with a local Rust backend. Hosted sign-in, signed releases, and broader device testing are still being prepared.",
          ],
          [
            "Where can I share feedback?",
            "Open an issue in the GitHub repository linked below. Please keep calendar details, account tokens, and other private information out of public reports.",
          ],
        ].map(([question, answer]) => (
          <details key={question}>
            <summary>
              {question}
              <Plus size={18} />
            </summary>
            <p>{answer}</p>
          </details>
        ))}
      </section>
      <MarketingFooter />
    </main>
  );
}
