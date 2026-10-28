import Link from 'next/link';
import { site } from '@/lib/site';
import { ArrowRightIcon, GithubIcon, CharlotteMark } from '@/components/icons';
import styles from './CTASection.module.css';

export function CTASection() {
  return (
    <section className={styles.section}>
      <div className={`shell ${styles.inner}`}>
        <CharlotteMark size={44} className={styles.kite} />
        <h2 className={styles.title}>Download Charlotte.</h2>
        <p className={styles.body}>
          Charlotte is a Noethric-branded commercial distribution. Original code remains subject to Samuel Abolo&apos;s MIT licence.
        </p>
        <div className={styles.ctas}>
          <Link href="/download" className="btn btn--primary">
            Download for free
            <ArrowRightIcon size={15} />
          </Link>
          <a href={site.githubRepo} target="_blank" rel="noreferrer" className="btn btn--ghost">
            <GithubIcon size={16} />
            Star on GitHub
          </a>
        </div>
      </div>
    </section>
  );
}
