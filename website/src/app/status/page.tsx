import type { Metadata } from 'next';
import { unavailableStatus } from '@/lib/status';
import { ConnectionPanel } from '@/components/ConnectionPanel';
import { StatusBoard } from './StatusBoard';
import styles from './status.module.css';

export const metadata: Metadata = {
  title: 'Live status | Charlotte',
  description: 'Unique rooms, total users, liveness, and response time for the public Charlotte chat server.',
};

export default function StatusPage() {
  return (
    <main className={styles.page}>
      <div className={`shell ${styles.header}`}>
        <span className="eyebrow">Live status</span>
        <h1 className={styles.h1}>Is the public server up?</h1>
        <p className={styles.lede}>
          Checks the configured chat server directly from your browser every few seconds. If it blocks browser cross-origin requests, live status is unavailable.
        </p>
      </div>

      <div className={`shell ${styles.layout}`}>
        <StatusBoard initial={unavailableStatus} />
        <ConnectionPanel />
      </div>
    </main>
  );
}
