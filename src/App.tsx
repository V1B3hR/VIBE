import { useState, useEffect } from "react";
import { listen, type Event } from "@tauri-apps/api/event";

import vibeLogo from "./assets/logo.png";

import "./App.css";

import { Mixer } from "./components/Mixer";
import { Timeline } from "./components/Timeline";
import { Transport } from "./components/Transport";
import { Library } from "./components/Library";
import { VirtualKeyboard } from "./components/VirtualKeyboard";
import { HistoryGraph } from "./components/HistoryGraph";
import { SpectrumAnalyzer } from "./components/SpectrumAnalyzer";
import { MasterMeters } from "./components/MasterMeters";
import { AudioSettings } from "./components/AudioSettings";
import IoSettings from "./components/IoSettings";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ToastProvider, useToast } from "./components/Toast";
import { FileMenu } from "./components/FileMenu";
import { SaveDialog } from "./components/dialogs/SaveDialog";
import { LoadDialog } from "./components/dialogs/LoadDialog";
import { ExportDialog } from "./components/dialogs/ExportDialog";
import { StatusBar } from "./components/StatusBar";
import { AiDroplet } from "./components/AiDroplet";
import { KropelkaPanel } from "./components/KropelkaPanel";
import { useAudioStats } from "./hooks/useAudioStats";
import { VideoPlayer } from "./components/VideoPlayer";

import "./services/Telemetry";
import { MidiLearnProvider } from "./context/MidiLearnContext";
import { safeInvoke } from "./services/SafeInvoke";

function App() {

  const [view, setView] = useState<"splash" | "mixer">("splash");
  const [showHistory, setShowHistory] = useState(false);
  const [showAudioSettings, setShowAudioSettings] = useState(false);
  const [showIoSettings, setShowIoSettings] = useState(false);
  const [theme, setTheme] = useState<"vone" | "gold" | "retro">("vone");
  const [showKropelka, setShowKropelka] = useState(true);
  const [showKropelkaPanel, setShowKropelkaPanel] = useState(false);
  const [showVideo, setShowVideo] = useState(false);

  const stats = useAudioStats();

  // Project Management State
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [showLoadDialog, setShowLoadDialog] = useState(false);
  const [showExportDialog, setShowExportDialog] = useState(false);
  const [currentProjectPath, setCurrentProjectPath] = useState<string>();
  const [isSaveAs, setIsSaveAs] = useState(false);
  const [recentProjects, setRecentProjects] = useState<Array<{ path: string; name: string }>>([]);

  useEffect(() => {
    // Immediate load to prevent flicker
    const savedScale = localStorage.getItem("vibe-ui-scale");
    if (savedScale) {
      document.documentElement.style.setProperty("--zoom-factor", savedScale);
    }

    // Load recent projects
    const recents = JSON.parse(localStorage.getItem('vibe-recent-projects') || '[]');
    setRecentProjects(recents);

    // Load last project path
    const lastPath = localStorage.getItem('lastProjectPath');
    if (lastPath) {
      setCurrentProjectPath(lastPath);
    }

    // Keyboard shortcuts
    const handleKeyboard = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 's') {
        e.preventDefault();
        handleSave();
      }
      if (e.ctrlKey && e.shiftKey && e.key === 'S') {
        e.preventDefault();
        handleSaveAs();
      }
      if (e.ctrlKey && e.key === 'o') {
        e.preventDefault();
        setShowLoadDialog(true);
      }
      if (e.ctrlKey && e.key === 'n') {
        e.preventDefault();
        handleNew();
      }
      if (e.ctrlKey && e.key === 'e') {
        e.preventDefault();
        setShowExportDialog(true);
      }
    };
    window.addEventListener('keydown', handleKeyboard);
    return () => window.removeEventListener('keydown', handleKeyboard);

    // Global File Drop Listener
    const unlistenPromise = listen('tauri://drop', async (event: Event<string[] | { paths: string[] }>) => {
      const payload = event.payload as { paths: string[] } | string[]; // V2 payload might vary, check docs handling
      // Usually { paths: [], ... } for some versions, or just array. 
      // Safe casting:
      const paths = Array.isArray(payload) ? payload : (payload as any).paths;

      if (paths && Array.isArray(paths)) {
        console.log("📂 Files dropped:", paths);
        for (const path of paths) {
          // Check extension roughly
          if (path.match(/\.(wav|mp3|flac|ogg|aiff)$/i)) {
            try {
              // Call the NEW ASYNC import command with safe invoke
              await safeInvoke("import_to_library", { path });
            } catch (e) {
              console.error("Import failed for", path, e);
            }
          }
        }
      }
    });

    return () => {
      unlistenPromise.then((unlisten: () => void) => unlisten());
    };
  }, []);

  const changeTheme = (newTheme: "vone" | "gold" | "retro") => {
    setTheme(newTheme);
    document.body.className = newTheme === "vone" ? "" : `theme-${newTheme}`;
  };

  // Project Management Handlers
  const handleNew = async () => {
    if (confirm('Create new project? Unsaved changes will be lost.')) {
      try {
        await safeInvoke('new_project');
        setCurrentProjectPath(undefined);
        localStorage.removeItem('lastProjectPath');
      } catch (e) {
        console.error("Failed to create new project:", e);
      }
    }
  };

  const handleSave = () => {
    if (currentProjectPath) {
      // Quick save to existing path
      safeInvoke('save_project_file', { path: currentProjectPath });
    } else {
      setIsSaveAs(false);
      setShowSaveDialog(true);
    }
  };

  const handleSaveAs = () => {
    setIsSaveAs(true);
    setShowSaveDialog(true);
  };

  const handleLoadRecent = async (path: string) => {
    try {
      await safeInvoke('load_project_file', { path });
      setCurrentProjectPath(path);
    } catch (error) {
      console.error('Failed to load project:', error);
    }
  };

  const handleProjectLoaded = (path: string) => {
    setCurrentProjectPath(path);
    // Refresh recent projects list
    const recents = JSON.parse(localStorage.getItem('vibe-recent-projects') || '[]');
    setRecentProjects(recents);
  };

  return (
    <ErrorBoundary>
      <ToastProvider>
        <MidiLearnProvider>
          <main className="container">
            {view === "splash" ? (
              <>
                <div className="logo-container">
                  <img src={vibeLogo} className="logo" alt="VIBE logo" />
                </div>

                <div className="title-container">
                  <h1 className="vibe-title">VIBE</h1>
                  <p className="vibe-subtitle">Digital Audio Workstation</p>
                </div>

                <div className="action-row">
                  <button className="btn-primary" onClick={() => setView("mixer")}>
                    Launch VIBE Studio
                  </button>
                </div>
              </>
            ) : (
              <div className="daw-view">
                <div className="top-bar-container">
                  <div className="top-bar-left">
                    <div className="vibe-text-logo">VIBE</div>
                    <FileMenu
                      onNew={handleNew}
                      onOpen={() => setShowLoadDialog(true)}
                      onSave={handleSave}
                      onSaveAs={handleSaveAs}
                      onExport={() => setShowExportDialog(true)}
                      onRecentProject={handleLoadRecent}
                      recentProjects={recentProjects}
                    />
                  </div>

                  <div className="top-bar-center">
                    <Transport />
                  </div>

                  <div className="top-bar-right">
                    <div className="secondary-tools">
                      <button
                        className={`btn-icon ${showHistory ? 'active' : ''}`}
                        onClick={() => setShowHistory(!showHistory)}
                        title="Project History"
                      >
                        ⏱️
                      </button>
                      <button
                        className="btn-icon"
                        onClick={() => setShowAudioSettings(true)}
                        title="Audio Settings"
                      >
                        ⚙️
                      </button>
                      <button
                        className="btn-icon"
                        onClick={() => setShowIoSettings(true)}
                        title="I/O Settings"
                      >
                        🔌
                      </button>
                      <button
                        className={`btn-icon ${showKropelka ? 'active' : ''}`}
                        onClick={() => setShowKropelka(!showKropelka)}
                        title="Kropelka Assistant"
                      >
                        🪐
                      </button>
                      <button
                        className={`btn-icon ${showKropelkaPanel ? 'active' : ''}`}
                        onClick={() => setShowKropelkaPanel(!showKropelkaPanel)}
                        title="Kropelka Co-Producer"
                        style={{ color: '#00f2ff' }}
                      >
                        🧠
                      </button>
                      <button
                        className={`btn-icon ${showVideo ? 'active' : ''}`}
                        onClick={() => setShowVideo(!showVideo)}
                        title="Video Synchronizer"
                      >
                        🎬
                      </button>
                    </div>
                    <div className={`engine-badge active`}>
                      <div className="studio-ring" />
                      <div>
                        <div className="studio-text">STUDIO</div>
                        <div className="live-text">LIVE</div>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="daw-main-layout">
                  <Library />
                  <div className="main-work-area">
                    <div className="timeline-section">
                      <Timeline />
                    </div>
                    <div className="mixer-section">
                      <Mixer />
                      <div className="master-and-spectrum" style={{display: 'flex', flexDirection: 'column', gap: '8px', width: '350px'}}>
                         <MasterMeters />
                         <SpectrumAnalyzer />
                      </div>
                    </div>
                    <div className="keyboard-section">
                      <VirtualKeyboard />
                    </div>
                    {showHistory && <HistoryGraph />}
                    {showVideo && <VideoPlayer onClose={() => setShowVideo(false)} />}
                  </div>
                </div>

                {showKropelka && (
                  <AiDroplet
                    masterLevel={stats.masterLevel}
                    isPlaying={stats.isPlaying}
                  />
                )}

                <KropelkaPanel
                  isOpen={showKropelkaPanel}
                  onClose={() => setShowKropelkaPanel(false)}
                  onAddClipToTimeline={(clip) => console.log("Added clip", clip.name)}
                />

                {showAudioSettings && (
                  <AudioSettings onClose={() => setShowAudioSettings(false)} />
                )}

                {showIoSettings && (
                  <IoSettings onClose={() => setShowIoSettings(false)} />
                )}

                <StatusBar />

                {showSaveDialog && (
                  <SaveDialog
                    isOpen={showSaveDialog}
                    onClose={() => setShowSaveDialog(false)}
                    currentPath={currentProjectPath}
                    isSaveAs={isSaveAs}
                  />
                )}

                {showLoadDialog && (
                  <LoadDialog
                    isOpen={showLoadDialog}
                    onClose={() => setShowLoadDialog(false)}
                    onProjectLoaded={handleProjectLoaded}
                  />
                )}

                {showExportDialog && (
                  <ExportDialog
                    isOpen={showExportDialog}
                    onClose={() => setShowExportDialog(false)}
                  />
                )}
              </div>
            )}

            <div className="footer">
              VIBE v0.1.0-alpha | Powered by Rust & React
            </div>
          </main>
        </MidiLearnProvider>
      </ToastProvider>
    </ErrorBoundary>
  );
}

export default App;


