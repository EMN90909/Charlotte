import Link from 'next/link';
import { ArrowRightIcon, CharlotteMark } from '@/components/icons';
import { BrowserMock } from './BrowserMock';
import styles from './Hero.module.css';

export function Hero() {
  return (
    <section className={styles.hero}>
      <div className={styles.kiteFloat} aria-hidden>
        <CharlotteMark size={64} />
      </div>

      <div className={`shell ${styles.grid}`}>
        <div className={styles.copy}>
          <span className="eyebrow">
            <span className={styles.eyebrowDot} />
            Charlotte by Noethric · Electron browser
          </span>

          <h1 className={styles.headline}>
            Every page you visit
            <br />
            has a room.
            <br />
            <span className={styles.headlineAccent}>You&apos;re never the only one in it.</span>
          </h1>

          <p className={styles.sub}>
            Charlotte is a browser with a chat panel built into every tab. Anyone else on the
            same URL right now can talk to you there. No accounts, no invites, no separate app.
            Bring an AI agent into the conversation, or run the server yourself.
          </p>

          <div className={styles.ctas}>
            <Link href="/download" className="btn btn--primary">
              Download Charlotte
              <ArrowRightIcon size={15} />
            </Link>
            <Link href="/licensing" className="btn btn--ghost">Licensing &amp; original author</Link>
          </div>

          <div className={styles.meta}>
            <span>Original MIT rights preserved</span>
            <span className={styles.metaSep} />
            <span>macOS · Windows · Linux</span>
            <span className={styles.metaSep} />
            <Link href="/protocol" className={styles.metaLink}>
              Read the wire protocol →
            </Link>
          </div>
        </div>

        <div className={styles.visual}>
          <BrowserMock />
        </div>
      </div>
    </section>
  );
}
