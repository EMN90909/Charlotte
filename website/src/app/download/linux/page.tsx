import type { Metadata } from 'next';
import Link from 'next/link';
import { getLatestBrowserRelease, type LinuxAssets } from '@/lib/releases';
import { site } from '@/lib/site';
import { LinuxIcon, ChipIcon, ArrowRightIcon, GithubIcon } from '@/components/icons';
import { AssetButton } from '../AssetButton';
import headerStyles from '../download.module.css';
import styles from '../platform.module.css';

export const metadata: Metadata = {
  title: 'Download Charlotte for Linux',
  description: 'Get the Charlotte browser for Linux: .deb, .rpm, and portable .zip builds for x64 and ARM64.',
};

function FormatButtons({ assets }: { assets: LinuxAssets }) {
  return (
    <div className={styles.assetList}>
      {assets.deb ? (
        <AssetButton asset={assets.deb} label="Debian, Ubuntu, Mint" hint=".deb" primary />
      ) : (
        <p className={styles.unavailable}>.deb not available in the latest release.</p>
      )}
      {assets.rpm ? (
        <AssetButton asset={assets.rpm} label="Fedora, RHEL, openSUSE" hint=".rpm" />
      ) : (
        <p className={styles.unavailable}>.rpm not available in the latest release.</p>
      )}
      {assets.zip ? (
        <AssetButton asset={assets.zip} label="Any distro (portable)" hint=".zip, extract and run" />
      ) : (
        <p className={styles.unavailable}>.zip not available in the latest release.</p>
      )}
      {assets.checksums && (
        <a href={assets.checksums.url} className={`mono ${styles.checksumLink}`}>
          Verify with SHA-256 checksums →
        </a>
      )}
    </div>
  );
}

export default async function LinuxDownloadPage() {
  const release = await getLatestBrowserRelease();

  return (
    <main className={headerStyles.page}>
      <div className={`shell ${headerStyles.header}`}>
        <Link href="/download" className={headerStyles.back}>
          ← All platforms
        </Link>
        <span className="eyebrow">
          <LinuxIcon size={13} /> Linux
        </span>
        <h1 className={headerStyles.h1}>Download Charlotte for Linux.</h1>
        <p className={headerStyles.lede}>
          {release ? (
            <>
              Version <span className="mono">{release.version}</span>. Choose your CPU
              architecture, then your distro&apos;s package format.
            </>
          ) : (
            "Choose your CPU architecture, then your distro's package format."
          )}
        </p>
      </div>

      <div className={`shell ${styles.archGrid}`}>
        <section className={styles.archCard}>
          <div className={styles.archHead}>
            <ChipIcon size={16} />
            <h2>x64</h2>
            
          </div>
          <p className={styles.archNote}>Intel and AMD processors, used in most desktops and laptops.</p>
          {release ? <FormatButtons assets={release.linux.x64} /> : <p className={styles.unavailable}>Release data unavailable.</p>}
        </section>

        <section className={styles.archCard}>
          <div className={styles.archHead}>
            <ChipIcon size={16} />
            <h2>ARM64</h2>
            
          </div>
          <p className={styles.archNote}>ARM-based devices, including Raspberry Pi 4/5, ARM laptops, and some SBCs.</p>
          {release ? <FormatButtons assets={release.linux.arm64} /> : <p className={styles.unavailable}>Release data unavailable.</p>}
        </section>
      </div>

      <div className={`shell ${styles.footer}`}>
        {release && (
          <a href={release.htmlUrl} target="_blank" rel="noreferrer" className={styles.footLink}>
            Release notes for {release.tag}
            <ArrowRightIcon size={12} />
          </a>
        )}
        <a href={site.githubReleases} target="_blank" rel="noreferrer" className={styles.footLink}>
          <GithubIcon size={13} />
          Browse all releases on GitHub
        </a>
      </div>
    </main>
  );
}
