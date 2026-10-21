import { marketingButtonClasses, marketingPageClasses } from "@/lib/marketing-styles";
import type { CSSProperties } from "react";
import {
  ArrowDown,
  ArrowRight,
  ArrowUpRight,
  CalendarDays,
  Check,
  Download,
  Menu,
  Plus,
  X,
} from "lucide-react";
import { useEffect, useState } from "react";
import { PhonePreview, ProductPreview, useInteractiveReady } from "./product-previews";
import { Button } from "./ui/button";

export { DownloadSection } from "./download-section";
import { DownloadSection } from "./download-section";

const repository = "https://github.com/hibatillah/calendar";

export function MarketingNav() {
  const interactiveReady = useInteractiveReady();
  const [menuOpen, setMenuOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const updateScroll = () => setScrolled(window.scrollY > 24);

    updateScroll();
    window.addEventListener("scroll", updateScroll, { passive: true });

    return () => window.removeEventListener("scroll", updateScroll);
  }, []);

  return (
    <div className="marketing-nav-space h-19.5 max-[700px]:h-17">
      <header
        className={`marketing-nav min-h-14.5 p-[10px_10px_10px_22px] flex items-center gap-[35px] justify-between fixed top-6.5 left-[50%] [transform:translateX(-50%)] w-[calc(100%_-_92px)] [border:1px_solid_transparent] rounded-[28px] [transition:width_320ms_ease,_top_320ms_ease,_min-height_320ms_ease,_border-radius_320ms_ease,_background-color_320ms_ease,_border-color_320ms_ease,_box-shadow_320ms_ease,_backdrop-filter_320ms_ease] z-[30] max-[700px]:p-[10px_10px_10px_22px] max-[700px]:gap-3 max-[700px]:min-h-12 max-[700px]:w-[calc(100%_-_46px)] max-[700px]:top-4.5${scrolled ? " is-floating [.marketing-nav&]:fixed [.marketing-nav&]:top-3.5 [.marketing-nav&]:left-[50%] [.marketing-nav&]:[transform:translateX(-50%)] [.marketing-nav&]:w-[min(1120px,_calc(100%_-_40px))] [.marketing-nav&]:min-h-16 [.marketing-nav&]:rounded-full [.marketing-nav&]:[background:rgb(255_255_255_/_78%)] [.marketing-nav&]:[border:1px_solid_rgb(255_255_255_/_80%)] [.marketing-nav&]:[backdrop-filter:blur(20px)] [.marketing-nav&]:shadow-[0_8px_32px_#23352a12] max-[700px]:[.marketing-nav&]:w-[calc(100%_-_24px)] max-[700px]:[.marketing-nav&]:top-2.5 max-[700px]:[.marketing-nav&]:min-h-15 max-[700px]:[.marketing-nav&]:p-[10px_10px_10px_22px] z-100" : ""}`}
      >
        <a
          href="/"
          className="calendar-wordmark text-[23px] font-[550] tracking-[-1px] inline-flex items-center gap-2 max-[700px]:text-[21px]"
          aria-label="Calendar home"
        >
          Calendar
        </a>
        <nav
          aria-label="Main navigation"
          className={
            menuOpen
              ? "marketing-links flex items-center gap-8 text-[13px] ml-25 [&_a]:inline-flex [&_a]:items-center [&_a]:gap-[5px] [&_a:hover]:opacity-[0.6] max-[1000px]:ml-0 max-[1000px]:gap-5 max-[700px]:hidden max-[700px]:absolute max-[700px]:top-16.5 max-[700px]:right-3.5 max-[700px]:left-3.5 max-[700px]:bg-[#f5fbf5] max-[700px]:[border:1px_solid_#bfcfc1] max-[700px]:p-5 max-[700px]:rounded-[12px] max-[700px]:shadow-[0_12px_30px_#314c3020] is-open max-[700px]:[.marketing-links&]:flex max-[700px]:[.marketing-links&]:flex-col max-[700px]:[.marketing-links&]:items-start max-[700px]:[.marketing-links&]:gap-4.5"
              : "marketing-links flex items-center gap-8 text-[13px] ml-25 [&_a]:inline-flex [&_a]:items-center [&_a]:gap-[5px] [&_a:hover]:opacity-[0.6] max-[1000px]:ml-0 max-[1000px]:gap-5 max-[700px]:hidden max-[700px]:absolute max-[700px]:top-16.5 max-[700px]:right-3.5 max-[700px]:left-3.5 max-[700px]:bg-[#f5fbf5] max-[700px]:[border:1px_solid_#bfcfc1] max-[700px]:p-5 max-[700px]:rounded-[12px] max-[700px]:shadow-[0_12px_30px_#314c3020]"
          }
        >
          <a href="/#features" onClick={() => setMenuOpen(false)}>
            Features
          </a>
          <a href="/download" onClick={() => setMenuOpen(false)}>
            Download
          </a>
          <a href={`${repository}/releases`} target="_blank" rel="noopener noreferrer">
            Changelog <ArrowUpRight size={12} />
          </a>
        </nav>
        <div className="marketing-nav-actions flex items-center gap-5.5 text-[13px] max-[700px]:gap-2">
          <Button asChild className={marketingButtonClasses}>
            <a href="/download">Get Calendar</a>
          </Button>
          <button
            className="mobile-menu-button hidden p-2 border-0 bg-none text-[#222a25] max-[700px]:inline-flex"
            disabled={!interactiveReady}
            aria-label={menuOpen ? "Close navigation" : "Open navigation"}
            aria-expanded={menuOpen}
            onClick={() => setMenuOpen(!menuOpen)}
          >
            {menuOpen ? <X /> : <Menu />}
          </button>
        </div>
      </header>
    </div>
  );
}

export function MarketingFooter() {
  return (
    <footer
      className={`
        marketing-footer max-w-277.5 m-[auto] p-[40px_0_30px] [border-top:1px_solid_#e0e4da] flex
        items-center gap-[25px] [flex-wrap:wrap] [&_>_span]:text-[#969f8e] [&_>_span]:text-[11px]
        [&_>_span]:flex-1 [&_nav]:flex [&_nav]:gap-[25px] [&_nav]:text-[11px] [&_nav_a]:flex
        [&_nav_a]:items-center [&_nav_a]:gap-1 [&_small]:w-full [&_small]:text-[#abb1a2] [&_small]:text-[9px]
        [&_small]:mt-[25px] max-[1000px]:ml-5 max-[1000px]:mr-5 max-[700px]:m-[0_18px] max-[700px]:gap-[15px]
        max-[700px]:[&_>_span]:text-[10px] max-[700px]:[&_nav]:w-full max-[700px]:[&_nav]:mt-3
        max-[700px]:[&_small]:mt-2.5
      `}
    >
      <a
        className="calendar-wordmark text-[23px] font-[550] tracking-[-1px] inline-flex items-center gap-2 max-[700px]:text-[21px]"
        href="/"
      >
        Calendar
      </a>
      <span>A little room for your time.</span>
      <nav aria-label="Footer navigation">
        <a href="/privacy">Preview privacy</a>
        <a href={`${repository}/issues`}>Feedback</a>
        <a href={repository} target="_blank" rel="noopener noreferrer">
          GitHub <ArrowUpRight size={13} />
        </a>
      </nav>
      <small>© 2026 Calendar · Open source under MIT</small>
    </footer>
  );
}

export function LandingPage() {
  return (
    <main className={marketingPageClasses}>
      <a
        href="#features"
        className="marketing-skip-link absolute left-[25px] top-[-100px] z-[99] bg-[#fff] p-2.5 [&:focus]:top-5"
      >
        Skip to features
      </a>
      <section
        className={`
          landing-hero relative isolate rounded-[28px] overflow-hidden [&::before]:content-['']
          [&::before]:absolute [&::before]:inset-0 [&::before]:z-[-1] [&::before]:rounded-[inherit]
          [&::before]:[background:radial-gradient(ellipse_at_6%_49%,_#ffffffc9_0%,_transparent_24%),_radial-gradient(ellipse_at_98%_30%,_#f3ffffd9,_transparent_28%),_linear-gradient(135deg,_#c5eff0,_#a9e2e9_46%,_#dbeee1)]
          max-[700px]:rounded-[19px]
        `}
      >
        <MarketingNav />
        <div
          className={`
            hero-copy text-center p-[60px_20px_55px] [&_h1_span]:text-[#3d675e] [&_h1]:whitespace-nowrap
            [&_h1]:text-[clamp(30px,_6.2vw,_88px)] [&_h1]:tracking-[-0.055em] [&_p]:text-[17px]
            [&_p]:leading-[1.65] [&_p]:text-[#47605a] max-[700px]:p-[38px_15px_45px]
            max-[700px]:[&_h1]:text-[clamp(28px,_7.8vw,_54px)] max-[700px]:[&_h1]:tracking-[-0.055em]
            max-[700px]:[&_p]:text-[13px]
          `}
        >
          <span
            className={`
              preview-badge [background:rgb(255_255_255_/_45%)] inline-flex items-center gap-2 text-[11px]
              [border:1px_solid_#344d4029] rounded-[50px] p-[5px_11px] text-[#41594c] [&_i]:w-[5px] [&_i]:h-[5px]
              [&_i]:bg-[#587767] [&_i]:rounded-full max-[700px]:text-[9px]
            `}
          >
            <i /> A new space for your Google calendars <ArrowRight size={12} />
          </span>
          <h1>
            Your week, <span>clearly.</span>
          </h1>
          <p>
            Work, life, and the time in between.
            <br />
            All your Google calendars in one calm space.
          </p>
          <div className="hero-actions flex justify-center items-center gap-3 mt-7 max-[700px]:gap-2.5 max-[700px]:flex-col">
            <Button
              asChild
              className={`${marketingButtonClasses} hero-download [.marketing-page_&]:h-[47px] [.marketing-page_&]:px-[23px] max-[700px]:[.marketing-page_&]:h-11 max-[700px]:[.marketing-page_&]:text-[12px]`}
            >
              <a href="/download">
                <Download size={17} />
                Get the Windows preview
              </a>
            </Button>
            <a
              href="/app"
              className="hero-secondary rounded-full [background:rgb(255_255_255_/_50%)] p-[13px_21px] text-[13px] inline-flex items-center gap-1 max-[700px]:text-[11px]"
            >
              Open calendar <ArrowUpRight size={15} />
            </a>
          </div>
        </div>
        <div className="hero-product m-[0_auto] max-w-277.5 p-[0_30px] h-130 overflow-hidden max-[700px]:p-[0_14px] max-[700px]:overflow-hidden max-[700px]:h-[343px]">
          <ProductPreview />
        </div>
      </section>
      <div
        className={`
          hero-preview-caption max-w-262.5 m-[13px_auto_0] p-[0_4px_27px] flex justify-between text-[#536f64]
          text-[10px] [&_>_span:last-child]:text-[8px] [&_>_span:last-child]:tracking-[1.5px]
          max-[1000px]:px-7.5 max-[700px]:p-[0_16px_20px] max-[700px]:text-[8px] max-[700px]:mt-2.5
          max-[700px]:[&_>_span:last-child]:hidden
        `}
      >
        <span>Example calendar · click an event to explore</span>
        <span>WEEK / MONTH</span>
      </div>
      <section
        className={`
          landing-intro text-center p-[110px_20px_65px] scroll-mt-22.5 [&_h2_span]:text-[#90998f] [&_p]:mt-6
          [&_p]:text-[#7b837b] [&_p]:text-[15px] max-[700px]:p-[75px_12px_40px] max-[700px]:[&_h2]:text-[38px]
          max-[700px]:[&_p]:text-[12px]
        `}
        id="features"
      >
        <span
          className={`
            section-eyebrow text-[9px] font-[550] tracking-[1.7px] block text-[#718078] mb-[23px]
            [.privacy-strip_&]:mb-[15px] max-[700px]:text-[8px] max-[700px]:tracking-[1.3px] max-[700px]:mb-4.5
          `}
        >
          LESS BETWEEN YOU AND YOUR DAY
        </span>
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
      <section
        className="feature-grid grid grid-cols-3 gap-4.5 max-w-277.5 m-[0_auto_105px] max-[1000px]:ml-5 max-[1000px]:mr-5 max-[700px]:grid-cols-1 max-[700px]:m-[0_12px_60px]"
        aria-label="Calendar features"
      >
        <article
          className={`
            feature-card bg-[#edf5fb] p-[27px] rounded-[17px] overflow-hidden [&:nth-child(2)]:bg-[#edf7ee]
            [&:nth-child(3)]:bg-[#fff1e8] [&_h3]:text-[26px] [&_h3]:leading-[1.2] [&_h3]:tracking-[-0.9px]
            [&_h3]:m-[20px_0_14px] [&_p]:text-[13px] [&_p]:leading-[1.7] [&_p]:text-[#7a8075] [&_p]:max-w-62.5
            max-[1000px]:p-5 max-[1000px]:[&_h3]:text-[23px] max-[700px]:p-[25px] max-[700px]:[&_h3]:text-[27px]
            max-[700px]:[&_p]:max-w-none
          `}
        >
          <span className="feature-number text-[8px] text-[#7d8479] tracking-[1px]">
            01 / A CLEARER VIEW
          </span>
          <div
            className={`
              feature-week-art h-43 mt-[25px] flex
              [background:repeating-linear-gradient(_transparent_0,_transparent_44px,_#dce0d6_44px,_#dce0d6_45px_)]
              [&_>_div]:w-[20%] [&_>_div]:[border-right:1px_solid_#dce0d6] [&_>_div]:relative
              [&_>_div:last-child]:[border-right:none] [&_span]:text-[9px] [&_span]:text-[#8e9489] [&_span]:block
              [&_span]:text-center [&_i]:absolute [&_i]:left-[5px] [&_i]:right-[5px] [&_i]:rounded-[5px]
              [&_i]:opacity-[1] max-[700px]:max-w-67.5 max-[700px]:ml-auto max-[700px]:mr-auto
            `}
            aria-hidden="true"
          >
            {["M", "T", "W", "T", "F"].map((day, index) => (
              <div key={index}>
                <span>{day}</span>
                <i
                  className="top-[var(--calendar-top)] h-[var(--calendar-height)] bg-[var(--calendar-background)]"
                  style={
                    {
                      "--calendar-top": `${57 + (index % 3) * 27}px`,
                      "--calendar-height": `${45 + (index % 2) * 30}px`,
                      "--calendar-background": ["#62b5ee", "#f49983", "#80cba0"][
                        index % 3
                      ],
                    } as CSSProperties
                  }
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
        <article
          className={`
            feature-card bg-[#edf5fb] p-[27px] rounded-[17px] overflow-hidden [&:nth-child(2)]:bg-[#edf7ee]
            [&:nth-child(3)]:bg-[#fff1e8] [&_h3]:text-[26px] [&_h3]:leading-[1.2] [&_h3]:tracking-[-0.9px]
            [&_h3]:m-[20px_0_14px] [&_p]:text-[13px] [&_p]:leading-[1.7] [&_p]:text-[#7a8075] [&_p]:max-w-62.5
            max-[1000px]:p-5 max-[1000px]:[&_h3]:text-[23px] max-[700px]:p-[25px] max-[700px]:[&_h3]:text-[27px]
            max-[700px]:[&_p]:max-w-none
          `}
        >
          <span className="feature-number text-[8px] text-[#7d8479] tracking-[1px]">
            02 / PLANS CAN CHANGE
          </span>
          <div
            className={`
              feature-move-art h-43 mt-[25px] flex flex-col items-center gap-[7px] [&_>_svg]:text-[#8a9b88]
              [&_>_svg]:[transform:rotate(-25deg)] max-[700px]:max-w-67.5 max-[700px]:ml-auto max-[700px]:mr-auto
            `}
            aria-hidden="true"
          >
            <span className="ghost-event p-[10px_22px] rounded-[5px] [border:1px_dashed_#a5b2a5] text-[#a3afa1] text-[11px] self-start ml-5">
              Design time
            </span>
            <ArrowDown size={24} />
            <span
              className={`
                moved-event relative bg-[#a5dfb9] [border-left:3px_solid_#42a96c] rounded-[5px] p-[14px_22px_18px]
                self-end mr-4 min-w-[165px] [&_strong]:block [&_strong]:text-[12px] [&_strong]:font-medium
                [&_small]:text-[#7e9077] [&_small]:text-[10px] [&_i]:w-[21px] [&_i]:h-0.5 [&_i]:rounded-[2px]
                [&_i]:bg-[#8a9d85] [&_i]:absolute [&_i]:bottom-[5px] [&_i]:left-[46%]
              `}
            >
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
        <article
          className={`
            feature-card bg-[#edf5fb] p-[27px] rounded-[17px] overflow-hidden [&:nth-child(2)]:bg-[#edf7ee]
            [&:nth-child(3)]:bg-[#fff1e8] [&_h3]:text-[26px] [&_h3]:leading-[1.2] [&_h3]:tracking-[-0.9px]
            [&_h3]:m-[20px_0_14px] [&_p]:text-[13px] [&_p]:leading-[1.7] [&_p]:text-[#7a8075] [&_p]:max-w-62.5
            max-[1000px]:p-5 max-[1000px]:[&_h3]:text-[23px] max-[700px]:p-[25px] max-[700px]:[&_h3]:text-[27px]
            max-[700px]:[&_p]:max-w-none
          `}
        >
          <span className="feature-number text-[8px] text-[#7d8479] tracking-[1px]">
            03 / ALL TOGETHER
          </span>
          <div
            className={`
              feature-accounts-art h-43 mt-[25px] flex flex-col justify-center gap-3 [&_>_div]:flex
              [&_>_div]:items-center [&_>_div]:gap-[11px] [&_>_div]:p-[13px] [&_>_div]:[border:1px_solid_#e2e4dc]
              [&_>_div]:bg-[#f9faf6] [&_>_div]:rounded-[10px] [&_>_div]:shadow-[0_3px_5px_#00000003]
              [&_span]:text-[10px] [&_span]:flex-1 [&_small]:block [&_small]:text-[9px] [&_small]:text-[#9ba296]
              [&_svg]:text-[#81937c] max-[700px]:max-w-67.5 max-[700px]:ml-auto max-[700px]:mr-auto
            `}
            aria-hidden="true"
          >
            <div>
              <i className="account-avatar w-7 h-7 grid place-items-center not-italic text-[#6a847a] bg-[#b4e7cb] rounded-full">
                a
              </i>
              <span>
                alex@example.com<small>Personal</small>
              </span>
              <Check size={15} />
            </div>
            <div>
              <i className="account-avatar w-7 h-7 grid place-items-center not-italic text-[#6a847a] bg-[#b4e7cb] rounded-full work-avatar text-[#8d6d62] bg-[#ffc9ad]">
                a
              </i>
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
      <section
        className={`
          mobile-feature max-w-277.5 m-[0_auto_80px] rounded-[24px] bg-[#edf0e6] grid grid-cols-2 min-h-115
          overflow-hidden max-[1000px]:ml-5 max-[1000px]:mr-5 max-[700px]:grid-cols-1
          max-[700px]:m-[0_12px_65px]
        `}
      >
        <div
          className={`
            mobile-feature-copy p-[46px_0_34px_55px] [&_h2]:text-[46px] [&_>_p]:text-[#7a8272] [&_>_p]:max-w-82.5
            [&_>_p]:mt-[23px] [&_>_p]:text-[14px] [&_ul]:p-0 [&_ul]:m-[28px_0] [&_ul]:list-none [&_ul]:grid
            [&_ul]:gap-3 [&_ul]:text-[12px] [&_ul]:text-[#626e59] [&_li]:flex [&_li]:gap-2.5 [&_li]:items-center
            max-[1000px]:pl-10 max-[1000px]:[&_h2]:text-[38px] max-[700px]:p-[38px_28px_10px]
            max-[700px]:[&_h2]:text-[38px] max-[700px]:[&_>_p]:text-[13px]
          `}
        >
          <span
            className={`
              section-eyebrow text-[9px] font-[550] tracking-[1.7px] block text-[#718078] mb-[23px]
              [.privacy-strip_&]:mb-[15px] max-[700px]:text-[8px] max-[700px]:tracking-[1.3px] max-[700px]:mb-4.5
            `}
          >
            THE BIG PICTURE. THE LITTLE PLANS.
          </span>
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
        </div>
        <div className="phone-stage relative flex items-start justify-center h-[387px] self-end overflow-hidden max-[700px]:h-[387px] max-[700px]:p-0">
          <span className="phone-orbit w-125 h-125 bg-[#e0e7d5] rounded-full absolute top-22.5 max-[700px]:top-[25px]" />
          <PhonePreview />
        </div>
      </section>
      <section
        className={`
          tray-feature max-w-262.5 m-[0_auto_110px] grid grid-cols-2 gap-[85px] items-center max-[1000px]:ml-5
          max-[1000px]:mr-5 max-[1000px]:gap-10 max-[700px]:grid-cols-1 max-[700px]:m-[0_12px_65px]
          max-[700px]:gap-7.5
        `}
      >
        <div
          className="tray-preview bg-[#e7eef0] h-87.5 rounded-[18px] p-[26px_30px] max-[700px]:p-[25px_22px] max-[700px]:h-82.5"
          data-theme="light"
          aria-label="Example upcoming schedule"
        >
          <div className="tray-system-bar flex items-center gap-[13px] justify-end text-[10px] text-[#78858a] [&_span:first-child]:mr-auto [&_span:first-child]:text-[9px]">
            <span>Wednesday, October 14</span>
            <CalendarDays size={16} />
            <span>09:22</span>
          </div>
          <div className="tray-popover relative mt-4.5 p-5 rounded-[13px] bg-muted text-foreground shadow-[0_15px_28px_#78898a30] [&_>_p]:mt-1 [&_>_p]:text-muted-foreground [&_>_p]:text-[9px]">
            <span className="tray-popover-notch w-3 h-3 absolute top-[-5px] right-[67px] [transform:rotate(45deg)] bg-muted" />
            <div className="tray-popover-title flex justify-between items-center [&_strong]:text-[15px] [&_strong]:font-medium [&_span]:text-muted-foreground [&_span]:text-[10px]">
              <strong>Coming up</strong>
              <span>Today</span>
            </div>
            <p>Your next little part of the day.</p>
            <div
              className={`
                tray-event flex items-center gap-2.5 mt-4.5 [&_>_i]:w-[3px] [&_>_i]:h-8 [&_>_i]:bg-[#c9b281]
                [&_>_i]:rounded-[3px] [&:nth-of-type(3)_>_i]:bg-[#b88776] [&_strong]:block [&_strong]:text-[11px]
                [&_strong]:font-medium [&_div_>_span]:block [&_div_>_span]:text-[9px]
                [&_div_>_span]:text-muted-foreground [&_div_>_span]:mt-[3px]
              `}
            >
              <i />
              <div>
                <strong>Coffee with Maya</strong>
                <span>09:30 – 10:30</span>
              </div>
              <span className="tray-countdown text-[#45613e] bg-[#e8efdf] p-[4px_6px] rounded-[5px] text-[8px] ml-auto">
                in 8 min
              </span>
            </div>
            <div
              className={`
                tray-event flex items-center gap-2.5 mt-4.5 [&_>_i]:w-[3px] [&_>_i]:h-8 [&_>_i]:bg-[#c9b281]
                [&_>_i]:rounded-[3px] [&:nth-of-type(3)_>_i]:bg-[#b88776] [&_strong]:block [&_strong]:text-[11px]
                [&_strong]:font-medium [&_div_>_span]:block [&_div_>_span]:text-[9px]
                [&_div_>_span]:text-muted-foreground [&_div_>_span]:mt-[3px]
              `}
            >
              <i />
              <div>
                <strong>Lunch, unhurried</strong>
                <span>12:00 – 13:00</span>
              </div>
            </div>
            <div className="tray-open-label text-[10px] text-muted-foreground [border-top:1px_solid_var(--calendar-grid)] pt-3.5 mt-5 flex justify-between">
              Open calendar <ArrowUpRight size={14} />
            </div>
          </div>
        </div>
        <div className="tray-copy [&_h2]:text-[43px] [&_>_p]:mt-5.5 [&_>_p]:text-[#7b847b] [&_>_p]:text-[14px] max-[700px]:p-[0_12px] max-[700px]:[&_h2]:text-[38px]">
          <span
            className={`
              section-eyebrow text-[9px] font-[550] tracking-[1.7px] block text-[#718078] mb-[23px]
              [.privacy-strip_&]:mb-[15px] max-[700px]:text-[8px] max-[700px]:tracking-[1.3px] max-[700px]:mb-4.5
            `}
          >
            RIGHT THERE WHEN YOU NEED IT
          </span>
          <h2>
            Your next plan.
            <br />
            Just a glance away.
          </h2>
          <p>
            Keep your upcoming schedule close in the Windows tray. macOS menu-bar
            support is being prepared, too.
          </p>
          <p className="small-feature-note [.tray-copy_&]:text-[11px] [.tray-copy_&]:text-[#929a90] [.tray-copy_&]:mt-[15px]">
            Optional desktop reminders are in development. They require Calendar to stay
            running.
          </p>
        </div>
      </section>
      <section
        className={`
          privacy-strip max-w-262.5 m-[0_auto_110px] [border-top:1px_solid_#dfe3da]
          [border-bottom:1px_solid_#dfe3da] p-[48px_0] grid grid-cols-2 gap-[85px] items-center
          [&_h2]:text-[35px] [&_h2]:tracking-[-1.3px] [&_p]:text-[#82897b] [&_p]:text-[14px] [&_p]:mb-4.5
          max-[1000px]:ml-5 max-[1000px]:mr-5 max-[1000px]:gap-10 max-[700px]:grid-cols-1
          max-[700px]:m-[0_24px_70px] max-[700px]:gap-[25px] max-[700px]:p-[35px_0]
          max-[700px]:[&_h2]:text-[31px]
        `}
      >
        <div>
          <span
            className={`
              section-eyebrow text-[9px] font-[550] tracking-[1.7px] block text-[#718078] mb-[23px]
              [.privacy-strip_&]:mb-[15px] max-[700px]:text-[8px] max-[700px]:tracking-[1.3px] max-[700px]:mb-4.5
            `}
          >
            YOUR CALENDAR, YOUR CALL
          </span>
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
          <a
            href="/privacy"
            className="marketing-inline-link inline-flex gap-2 items-center text-[12px] [border-bottom:1px_solid_#a4afa0] pb-1"
          >
            How the preview handles your data <ArrowUpRight size={16} />
          </a>
        </div>
      </section>
      <DownloadSection />
      <section
        className={`
          faq-section max-w-170 m-[0_auto_110px] [&_h2]:text-center [&_h2]:text-[33px] [&_h2]:tracking-[-1px]
          [&_h2]:mb-[35px] [&_details]:[border-bottom:1px_solid_#e1e5dc] [&_details]:p-[20px_0]
          [&_summary]:list-none [&_summary]:cursor-pointer [&_summary]:flex [&_summary]:items-center
          [&_summary]:justify-between [&_summary]:gap-5 [&_summary]:text-[13px]
          [&_summary::-webkit-details-marker]:hidden [&_summary_>_svg]:text-[#9fa798]
          [&_summary_>_svg]:shrink-0 [&_details[open]_summary_>_svg]:[transform:rotate(45deg)]
          [&_details_>_p]:text-[12px] [&_details_>_p]:leading-[1.8] [&_details_>_p]:text-[#7f8979]
          [&_details_>_p]:pt-3.5 [&_details_>_p]:max-w-152.5 max-[700px]:m-[0_24px_65px]
          max-[700px]:[&_h2]:text-[29px]
        `}
      >
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
