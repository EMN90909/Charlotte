import type { Metadata } from 'next';
import Link from 'next/link';

export const metadata: Metadata = {
  title: 'Licensing and attribution | Charlotte',
  description: 'Original MIT-licensed rights and the Noethric Charlotte commercial distribution.',
};

export default function LicensingPage() {
  return (
    <main className="shell" style={{ paddingBlock: '5rem', maxWidth: 860 }}>
      <span className="eyebrow">Charlotte by Noethric</span>
      <h1>Licensing &amp; attribution</h1>
      <p>
        Charlotte is a Noethric-branded commercial browser distribution built on
        software originally published under the MIT License by Samuel Abolo.
      </p>
      <h2>Original author: Samuel Abolo</h2>
      <p>
        Copyright (c) 2026 Samuel Abolo. The original MIT-licensed code retains
        its existing permissions, including reuse, modification and distribution
        subject to preserving the original MIT licence and copyright notice.
        Neither the Charlotte name nor Noethric commercial distribution takes
        those rights away.
      </p>
      <h2>Noethric commercial material</h2>
      <p>
        Noethric reserves proprietary rights only in content it actually owns
        or is authorized to license on proprietary terms. This does not
        relicense the original MIT code, alter third-party licences or
        transfer the original author&apos;s copyright.
      </p>
      <p>
        See the repository&apos;s original MIT LICENSE and
        browser/COMMERCIAL-LICENSE.md for the complete rights notices.
        Written authorization from the appropriate copyright holder is needed
        for any exclusive assignment of pre-existing code.
      </p>
      <Link href="/">Return to Charlotte</Link>
    </main>
  );
}
