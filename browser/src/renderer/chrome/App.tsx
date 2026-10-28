import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  NavState,
  TabInfo,
  SafetySettings,
  ThemePayload,
  BookmarkEntry,
  BookmarkFolder,
  DomainTrustLists,
  ProxySettings,
  AdBlockStatus,
  DownloadRecord,
  PermissionRequestPayload,
  SubtitleSettings,
  PageTranslateSettings,
  PageTranslateStatusPayload,
  WhisperStatus,
} from '../../shared/types';
import {
  DEFAULT_SAFETY_SETTINGS,
  DEFAULT_DOMAIN_TRUST_LISTS,
  DEFAULT_PROXY_SETTINGS,
  DEFAULT_AD_BLOCK_SETTINGS,
  DEFAULT_SUBTITLE_SETTINGS,
  DEFAULT_PAGE_TRANSLATE_SETTINGS,
} from '../../shared/types';
import { TabStrip } from './components/TabStrip';
import { Toolbar } from './components/Toolbar';
import { SettingsModal } from './components/settings/SettingsModal';
import { FindBar } from './components/FindBar';
import { DownloadsPanel } from './components/DownloadsPanel';
import { PermissionPrompt } from './components/PermissionPrompt';
import { WhisperRequiredModal } from './components/WhisperRequiredModal';
import { AudioCaptureErrorModal } from './components/AudioCaptureErrorModal';
import { NoAudioDetectedModal } from './components/NoAudioDetectedModal';
import { OnboardingModal } from './components/onboarding/OnboardingModal';
import { ToolbarTour } from './components/onboarding/ToolbarTour';
import { startAudioCapture, type AudioCaptureHandle } from './audioCapture';

export function App() {
  const [tabs, setTabs] = useState<TabInfo[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [navState, setNavState] = useState<NavState | null>(null);
  const [safety, setSafety] = useState<SafetySettings>(DEFAULT_SAFETY_SETTINGS);
  const [theme, setTheme] = useState<ThemePayload>({ source: 'system', isDark: false });
  const [chatOpen, setChatOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [onboardingOpen, setOnboardingOpen] = useState(false);
  const [tourOpen, setTourOpen] = useState(false);
  // Set only by onOpenChatServerSettings below (the chat panel's "fix this
  // server's username" CTA) - never by the normal gear-icon openSettings(),
  // so a plain Settings open doesn't jump sections. `token` increments on
  // every trigger so SettingsModal's deep-link effect re-fires even if the
  // same server is clicked twice in a row.
  const [pendingChatServerFocus, setPendingChatServerFocus] = useState<{ serverId: string; token: number } | null>(
    null,
  );
  const [bookmarks, setBookmarks] = useState<BookmarkEntry[]>([]);
  const [bookmarkFolders, setBookmarkFolders] = useState<BookmarkFolder[]>([]);
  const [bookmarkPopoverOpen, setBookmarkPopoverOpen] = useState(false);
  const [addressSuggestionsOpen, setAddressSuggestionsOpen] = useState(false);
  const [subtitleSettings, setSubtitleSettings] = useState<SubtitleSettings>(DEFAULT_SUBTITLE_SETTINGS);
  const [subtitlePopoverOpen, setSubtitlePopoverOpen] = useState(false);
  const [pageTranslateSettings, setPageTranslateSettings] = useState<PageTranslateSettings>(DEFAULT_PAGE_TRANSLATE_SETTINGS);
  const [pageTranslatePopoverOpen, setPageTranslatePopoverOpen] = useState(false);
  const [pageTranslateStatus, setPageTranslateStatus] = useState<PageTranslateStatusPayload>({ status: 'idle' });
  const [whisperStatus, setWhisperStatus] = useState<WhisperStatus | null>(null);
  const [whisperModalOpen, setWhisperModalOpen] = useState(false);
  const [noAudioModalOpen, setNoAudioModalOpen] = useState(false);
  const [captureErrorOpen, setCaptureErrorOpen] = useState(false);
  const captureRef = useRef<AudioCaptureHandle | null>(null);
  const [domainTrust, setDomainTrust] = useState<DomainTrustLists>(DEFAULT_DOMAIN_TRUST_LISTS);
  const [proxySettings, setProxySettings] = useState<ProxySettings>(DEFAULT_PROXY_SETTINGS);
  const [adBlockStatus, setAdBlockStatus] = useState<AdBlockStatus>({ settings: DEFAULT_AD_BLOCK_SETTINGS, blockedCount: 0 });
  const [findBarOpen, setFindBarOpen] = useState(false);
  const [findResult, setFindResult] = useState<{ activeMatchOrdinal: number; matches: number } | null>(null);
  const [downloads, setDownloads] = useState<DownloadRecord[]>([]);
  const [downloadsOpen, setDownloadsOpen] = useState(false);
  const [permissionQueue, setPermissionQueue] = useState<PermissionRequestPayload[]>([]);

  useEffect(() => {
    const unsubs = [
      window.charlotte.onTabsUpdated(({ tabs, activeId }) => {
        setTabs(tabs);
        setActiveId(activeId);
      }),
      window.charlotte.onNavState(setNavState),
      window.charlotte.onSafetySettings(setSafety),
      window.charlotte.onTheme(setTheme),
      window.charlotte.onBookmarksUpdated(setBookmarks),
      window.charlotte.onBookmarkFoldersUpdated(setBookmarkFolders),
      window.charlotte.onSubtitleSettings(setSubtitleSettings),
      window.charlotte.onPageTranslateSettings(setPageTranslateSettings),
      window.charlotte.onPageTranslateStatus(setPageTranslateStatus),
      window.charlotte.onWhisperStatus(setWhisperStatus),
      window.charlotte.onDomainTrust(setDomainTrust),
      window.charlotte.onProxySettings(setProxySettings),
      window.charlotte.onAdBlockStatus(setAdBlockStatus),
      window.charlotte.onFindBarOpen(() => {
        setFindResult(null);
        setFindBarOpen(true);
      }),
      window.charlotte.onFindResult(setFindResult),
      window.charlotte.onDownloadsUpdated(setDownloads),
      window.charlotte.onPermissionRequested((request) => setPermissionQueue((q) => [...q, request])),
      window.charlotte.onOnboardingState(({ completed }) => setOnboardingOpen(!completed)),
      window.charlotte.onOpenChatServerSettings((serverId) => {
        setSettingsOpen(true);
        setPendingChatServerFocus((current) => ({ serverId, token: (current?.token ?? 0) + 1 }));
      }),
    ];
    // Ask main for the current state now that listeners are attached.
    window.charlotte.ready();
    return () => unsubs.forEach((unsub) => unsub());
  }, []);

  const closeFindBar = () => {
    setFindBarOpen(false);
    setFindResult(null);
    window.charlotte.closeFindBar();
  };

  const bookmarkEntry = navState?.url ? (bookmarks.find((b) => b.url === navState.url) ?? null) : null;
  const hasActiveDownload = downloads.some((d) => d.state === 'progressing');
  const openInNewTab = (url: string) => window.charlotte.newTab(url);

  // Navigating away closes any open toolbar popover, same as Chrome - it's
  // about the page you were just on, not a persistent panel.
  useEffect(() => {
    setBookmarkPopoverOpen(false);
    setSubtitlePopoverOpen(false);
    setPageTranslatePopoverOpen(false);
  }, [navState?.url]);

  const onStarClick = () => {
    if (!navState?.url) return;
    setSubtitlePopoverOpen(false);
    if (!bookmarkEntry) {
      window.charlotte.toggleBookmark({ url: navState.url, title: navState.title });
      setBookmarkPopoverOpen(true);
    } else {
      setBookmarkPopoverOpen((open) => !open);
    }
  };

  const onSubtitlesClick = () => {
    setBookmarkPopoverOpen(false);
    setPageTranslatePopoverOpen(false);
    // Pressing the toolbar button again is the obvious "get me out of this"
    // gesture - close out any whisper/capture/permission modal it may have
    // opened, not just toggle the popover, so none of them can get stuck
    // open behind it.
    setWhisperModalOpen(false);
    setNoAudioModalOpen(false);
    setCaptureErrorOpen(false);
    setSubtitlePopoverOpen((open) => !open);
  };

  const onPageTranslateClick = () => {
    setBookmarkPopoverOpen(false);
    setSubtitlePopoverOpen(false);
    setPageTranslatePopoverOpen((open) => !open);
  };

  // No upfront gate the way subtitles has (whisper.cpp readiness) - page
  // translate only ever needs an agent, and the popover itself already
  // keeps the toggle disabled until one's picked (see
  // PageTranslatePopover's canEnable), so there's nothing to intercept
  // here beyond just relaying the change.
  const onChangePageTranslateSettings = (settings: PageTranslateSettings) => {
    window.charlotte.setPageTranslateSettings(settings);
  };

  // Blocks the actual enable (rather than letting main turn it on and then
  // immediately having nothing to feed the overlay) when whisper.cpp isn't
  // set up - see main/whisperStore.ts. There's no equivalent upfront check
  // for the Screen & System Audio Recording permission - macOS's own
  // systemPreferences.getMediaAccessStatus('screen') isn't trustworthy
  // (confirmed by hand: it reported 'granted' while System Settings showed
  // the app wasn't even in that list), so that gets caught reactively
  // instead, once real audio has actually been measured - see the capture
  // effect below and audioCapture.ts's RMS silence detection. Language/
  // agent edits while already on or off still go straight through.
  const onChangeSubtitleSettings = (settings: SubtitleSettings) => {
    if (settings.enabled && !subtitleSettings.enabled && !whisperStatus?.ready) {
      setWhisperModalOpen(true);
      return;
    }
    window.charlotte.setSubtitleSettings(settings);
  };

  // Starts/stops the actual tab-audio capture (see audioCapture.ts) to
  // track whichever tab is active and has translate turned on - this is
  // the only thing that decides whether capture is running, not the
  // popover directly, so switching tabs starts/stops it automatically too
  // (subtitleSettings itself already tracks the active tab - see
  // TabManager.switchTab/onActiveSubtitleSettingsChanged).
  useEffect(() => {
    if (subtitleSettings.enabled && !captureRef.current) {
      let cancelled = false;
      void startAudioCapture(
        (chunk) => window.charlotte.sendAudioChunk(chunk),
        // Real digital silence for several chunks running, not just a
        // quiet moment (see audioCapture.ts). Turn translate back off too -
        // leaving capture running against silence just burns CPU on
        // whisper hallucinating captions from nothing.
        () => {
          setNoAudioModalOpen(true);
          window.charlotte.setSubtitleSettings({ ...subtitleSettings, enabled: false });
        },
      )
        .then((handle) => {
          if (cancelled) {
            handle.stop();
            return;
          }
          captureRef.current = handle;
        })
        .catch((err) => {
          console.error('Failed to start tab audio capture:', err);
          setCaptureErrorOpen(true);
          window.charlotte.setSubtitleSettings({ ...subtitleSettings, enabled: false });
        });
      return () => {
        cancelled = true;
      };
    }
    if (!subtitleSettings.enabled && captureRef.current) {
      captureRef.current.stop();
      captureRef.current = null;
    }
  }, [subtitleSettings]);

  useEffect(() => {
    document.documentElement.dataset.theme = theme.isDark ? 'dark' : 'light';
  }, [theme.isDark]);

  // Settings, Downloads, and any full .modal-overlay-based dialog (the
  // permission prompt, the whisper/capture/no-audio modals) all
  // need the chrome view grown to full-window size (see
  // WindowManager.setChromeFullscreen) - a bare .modal-overlay's
  // `position: fixed; inset: 0` only covers chrome's OWN native bounds,
  // not the whole app window, unless this is set. Keep it grown as long as
  // any of them is open, only shrink back once all are closed.
  useEffect(() => {
    window.charlotte.setOverlayOpen(
      settingsOpen ||
        onboardingOpen ||
        tourOpen ||
        downloadsOpen ||
        permissionQueue.length > 0 ||
        whisperModalOpen ||
        noAudioModalOpen ||
        captureErrorOpen,
    );
  }, [settingsOpen, onboardingOpen, tourOpen, downloadsOpen, permissionQueue.length, whisperModalOpen, noAudioModalOpen, captureErrorOpen]);

  // Same idea, but a small reserved strip instead of the whole window -
  // see windowManager.ts's setToolbarPopoverOpen. The bookmark/subtitle
  // popovers and the address bar's autocomplete dropdown can't overlap in
  // practice (typing in the address bar isn't possible while either of
  // the other two is open), but main only needs to know "is something
  // open", not which one.
  useEffect(() => {
    window.charlotte.setToolbarPopoverOpen(
      bookmarkPopoverOpen || subtitlePopoverOpen || pageTranslatePopoverOpen || addressSuggestionsOpen,
    );
  }, [bookmarkPopoverOpen, subtitlePopoverOpen, pageTranslatePopoverOpen, addressSuggestionsOpen]);

  const toggleChat = () => {
    setChatOpen((open) => !open);
    window.charlotte.toggleChat();
  };

  const openSettings = () => setSettingsOpen(true);
  const closeSettings = () => {
    setSettingsOpen(false);
    // Clears any deep-link so the NEXT open (e.g. a plain gear-icon click)
    // starts on General instead of re-jumping to whatever server the last
    // CTA pointed at.
    setPendingChatServerFocus(null);
  };
  const finishOnboarding = () => {
    setOnboardingOpen(false);
    // Marked done now, not after the tour - the tour is just a pointer
    // walk, so quitting halfway through it shouldn't replay onboarding.
    window.charlotte.completeOnboarding();
    setTourOpen(true);
  };
  const closeTour = useCallback(() => setTourOpen(false), []);
  const openDownloads = () => setDownloadsOpen(true);
  const closeDownloads = () => setDownloadsOpen(false);

  const respondToPermission = (allow: boolean, remember: boolean) => {
    const request = permissionQueue[0];
    if (!request) return;
    window.charlotte.respondToPermission({ requestId: request.requestId, allow, remember });
    setPermissionQueue((q) => q.slice(1));
  };

  return (
    <div className="chrome">
      {/* Everything actually visible in the toolbar strip lives in this
          wrapper, which is exactly as tall as its real content (92px, or
          136px with the find bar open) - NOT the same as the native
          chrome view's own bounds, which grow taller than that to give a
          toolbar popover room to render (see windowManager.ts's
          TOOLBAR_POPOVER_HEIGHT). That extra space has no DOM content of
          its own, so it needs its own opaque background/border here
          rather than on .chrome - otherwise the border would draw at the
          bottom of the reserved (but empty) space, not at the toolbar's
          real edge, and the reserved space itself would show through to
          whatever the chrome view's native background falls back to
          instead of the actual page underneath. */}
      <div className="chrome__bar">
        <TabStrip
          tabs={tabs}
          activeId={activeId}
          onSwitch={(id) => window.charlotte.switchTab(id)}
          onClose={(id) => window.charlotte.closeTab(id)}
          onNewTab={() => window.charlotte.newTab()}
        />
        <Toolbar
          navState={navState}
          chatOpen={chatOpen}
          bookmarkEntry={bookmarkEntry}
          bookmarks={bookmarks}
          folders={bookmarkFolders}
          bookmarkPopoverOpen={bookmarkPopoverOpen}
          subtitleSettings={subtitleSettings}
          subtitlePopoverOpen={subtitlePopoverOpen}
          pageTranslateSettings={pageTranslateSettings}
          pageTranslatePopoverOpen={pageTranslatePopoverOpen}
          pageTranslateStatus={pageTranslateStatus}
          hasActiveDownload={hasActiveDownload}
          onBack={() => window.charlotte.goBack()}
          onForward={() => window.charlotte.goForward()}
          onReload={() => window.charlotte.reload()}
          onNavigate={(input) => window.charlotte.navigate(input)}
          onToggleChat={toggleChat}
          onOpenSettings={openSettings}
          onStarClick={onStarClick}
          onCloseBookmarkPopover={() => setBookmarkPopoverOpen(false)}
          onRenameBookmark={(id, title) => window.charlotte.renameBookmark({ id, title })}
          onMoveBookmark={(id, folderId) => window.charlotte.moveBookmark({ id, folderId })}
          onRemoveBookmark={(id) => window.charlotte.deleteBookmark(id)}
          onCreateBookmarkFolder={(name, parentId) => window.charlotte.createBookmarkFolder({ name, parentId })}
          onSubtitlesClick={onSubtitlesClick}
          onCloseSubtitlePopover={() => setSubtitlePopoverOpen(false)}
          onChangeSubtitleSettings={onChangeSubtitleSettings}
          onPageTranslateClick={onPageTranslateClick}
          onClosePageTranslatePopover={() => setPageTranslatePopoverOpen(false)}
          onChangePageTranslateSettings={onChangePageTranslateSettings}
          onOpenDownloads={openDownloads}
          onSuggestionsOpenChange={setAddressSuggestionsOpen}
        />
        {findBarOpen && (
          <FindBar
            result={findResult}
            onSearch={(text, forward, findNext) => window.charlotte.findInPage({ text, forward, findNext })}
            onClose={closeFindBar}
          />
        )}
      </div>
      {settingsOpen && (
        <SettingsModal
          safety={safety}
          themeSource={theme.source}
          bookmarks={bookmarks}
          bookmarkFolders={bookmarkFolders}
          domainTrust={domainTrust}
          proxySettings={proxySettings}
          adBlockStatus={adBlockStatus}
          focusChatServerId={pendingChatServerFocus?.serverId ?? null}
          focusToken={pendingChatServerFocus?.token ?? 0}
          onClose={closeSettings}
          onSaveSafety={(settings) => window.charlotte.setSafetySettings(settings)}
          onSaveTheme={(source) => window.charlotte.setTheme(source)}
          onDeleteBookmark={(id) => window.charlotte.deleteBookmark(id)}
          onRenameBookmark={(id, title) => window.charlotte.renameBookmark({ id, title })}
          onMoveBookmark={(id, folderId) => window.charlotte.moveBookmark({ id, folderId })}
          onCreateBookmarkFolder={(name, parentId) => window.charlotte.createBookmarkFolder({ name, parentId })}
          onRenameBookmarkFolder={(id, name) => window.charlotte.renameBookmarkFolder({ id, name })}
          onDeleteBookmarkFolder={(id) => window.charlotte.deleteBookmarkFolder(id)}
          onOpenInNewTab={openInNewTab}
          onSaveDomainTrust={(lists) => window.charlotte.setDomainTrust(lists)}
          onSaveProxySettings={(settings) => window.charlotte.setProxySettings(settings)}
          onSaveAdBlockSettings={(settings) => window.charlotte.setAdBlockSettings(settings)}
        />
      )}
      {downloadsOpen && (
        <DownloadsPanel
          downloads={downloads}
          onClose={closeDownloads}
          onCancel={(id) => window.charlotte.cancelDownload(id)}
          onOpen={(id) => window.charlotte.openDownload(id)}
          onShowInFolder={(id) => window.charlotte.showDownloadInFolder(id)}
          onClearFinished={() => window.charlotte.clearDownloads()}
        />
      )}
      {permissionQueue.length > 0 && <PermissionPrompt request={permissionQueue[0]} onRespond={respondToPermission} />}
      {noAudioModalOpen && (
        <NoAudioDetectedModal
          onOpenSettings={() => {
            setNoAudioModalOpen(false);
            openSettings();
          }}
          onDismiss={() => setNoAudioModalOpen(false)}
        />
      )}
      {whisperModalOpen && (
        <WhisperRequiredModal
          onOpenSettings={() => {
            setWhisperModalOpen(false);
            openSettings();
          }}
          onDismiss={() => setWhisperModalOpen(false)}
        />
      )}
      {captureErrorOpen && <AudioCaptureErrorModal onDismiss={() => setCaptureErrorOpen(false)} />}
      {/* Last, so it stacks above anything else that opens on first launch. */}
      {onboardingOpen && <OnboardingModal onFinish={finishOnboarding} />}
      {tourOpen && <ToolbarTour onDone={closeTour} />}
    </div>
  );
}
