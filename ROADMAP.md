# 🚀 VIBE DAW - Skonsolidowana Mapa Drogowa (Unified Roadmap)

## Wersja: `v0.5.0-beta`
**Ostatnia aktualizacja:** Lipiec 2026  
**Status silnika:** 🟢 Stabilny, wysoce zoptymalizowany, zintegrowany z WebGL & Kropelka AI.

> [!IMPORTANT]
> Niniejszy dokument stanowi jedyne, ujednolicone źródło prawdy (Single Source of Truth) dotyczące statusu projektu VIBE DAW. Został opracowany na podstawie audytów kodu, planów implementacji oraz raportów testowych. Koncentruje się na **profesjonalnym, detalicznym i dogłębnym wykończeniu (polishing)** istniejących funkcjonalności oraz stabilizacji systemu.

---

## 📊 1. Aktualny Stan Projektu (Completed Modules)

Poniższe moduły zostały w 100% zaimplementowane, zweryfikowane i zintegrowane w silniku:
- **Audio Core & Summing**: Zero-allocation sumator, przetwarzanie w wątkach roboczych (Rayon), ochrona przed denormalami (denormal protection), wewnętrzne przetwarzanie 64-bitowe.
- **Waveform GL (Aura Engine)**: Renderowanie WebGL z poziomem LOD 0 (16 próbek/punkt) oraz gładką analogową linią (1.5px), pełne wsparcie HiDPI/devicePixelRatio oraz re-fetching 100ms.
- **Kropelka v3.0 Brain**: Integracja z serwisem ML Python (NeuralForest), detekcja nastroju sesji, rekomendacje miksu (auto-balancing), asystent głosowy oraz inteligentne reguły EQ.
- **WASM Sandboxing**: Bezpieczne uruchamianie wtyczek WASM z izolacją pamięci przy użyciu runtime `wasmer`.
- **Advanced Export & Mastering**: Dithering (Triangular, Noise Shaping), normalizacja głośności LUFS (EBU R128), wykrywanie True Peak i nowoczesny dialog eksportu.
- **Arrangement 2.0 & Automation**: Interaktywne uchwyty pętli na linijce, Clip Gain handles z dB feedbackiem, krzywe Beziera dla automatyki z interaktywnymi punktami, Ghost Clips (wizualizacja grup w folderach).

---

## 🎯 2. Rejestr Niedokończonych Zadań (Outstanding Tasks & Polish Plan)

Poniższa lista zawiera wszystkie zaległe i nieukończone zadania, pogrupowane według priorytetów z naciskiem na najwyższą jakość wykonania.

### 2.1 Refaktoryzacja, Architektura i Dzielenie Kodu (Code Health & Modularization)
*   **[x] Podział monolitycznych plików frontendowych**:
    *   `PianoRoll.tsx` przekracza obecnie 1000 linii kodu. Należy rozbić go na mniejsze moduły (np. `PianoRollToolbar.tsx`, `PianoRollGrid.tsx`, `MidiNoteRenderer.tsx`) w celu poprawy czytelności i łatwości utrzymania.
*   **[x] Usunięcie Martwego Kodu (Dead Code Cleanup)**:
    *   Przegląd i usunięcie nieużywanych metod w starszych modułach backendu (np. `recovery.rs` oraz historycznych strukturach przywracania stanu).
*   **[x] Usprawnienie typowania TypeScript**:
    *   Eliminacja pozostałych rzutowań typów `any` we frontendowych interfejsach wtyczek i automatyki na rzecz ściśle typowanych struktur współdzielonych z Rustem.

---

### 2.2 Ulepszenia Interfejsu i Detaliczny UX (UI/UX Polish)
*   **[x] Audyt Spójności Wizualnej (Visual Consistency Audit)**:
    *   Wymiana wszystkich twardych kodów kolorów `#ffd700` na zmienną `var(--accent)` w: `AudioSettings.css`, `IoSettings.css`, `Timeline.css`, `TimelineRuler.tsx`, `SpectrumAnalyzer.css`, `Mixer.css`, `MasterMeters.css`, `Library.css`.
    *   Ujednolicenie nakładek modalnych (`--overlay-bg`, `--modal-bg`, `--modal-border`, `--modal-radius`, `--modal-shadow`) we wszystkich oknach dialogowych.
*   **[x] Rafinacja Skrótów Klawiszowych (Keyboard Shortcuts)**:
    *   Dodanie wizualnego panelu skrótów klawiszowych bezpośrednio w oknie Audio Settings — 4 kolumny tematyczne (Transport, Editing, Navigation, Mixer).
    *   Stylizacja `<kbd>` z efektami neonowego blasku nawiązującymi do aktywnego motywu (`var(--accent)`).
*   **[x] Wygładzenie Animacji Kropelki (Avatar Gestures)**:
    *   Implementacja klasy `is-transitioning` w `Kropelka.tsx` z 450ms cross-fade przy zmianie nastroju.
    *   Rozszerzenie CSS `transition` na `.kropelka-core` o `filter`, `opacity`, `border-radius`.
    *   Naprawiono błąd wyścigu testów (`tracksRef` timing w `slice_clip`).

---

### 2.3 Optymalizacja Wydajności i Skalowalność (Performance Optimization)
*   **[x] Wirtualizacja Listy Ścieżek (Virtual Scroll Refinement)**:
    *   Wdrożono buforowanie zdarzeń przewijania z wykorzystaniem `requestAnimationFrame` w `Timeline.tsx` (redukcja re-renderów React z kilkunastu na klatkę do 1/frame).
    *   Zoptymalizowano `Mixer.tsx` poprzez otoczenie `TrackRenderer` hookiem `useCallback` oraz zwiększenie `overscanCount` (z 2 do 4), wyeliminowano ponowne montowanie pasków miksera przy aktualizacji wskaźników VU.
*   **[x] Aktualizacja SIMD do AVX-512**:
    *   Wdrożono `SimdSummer` w `simd_avx512.rs` wykorzystujący instrukcje `avx512f` z dynamicznym wykrywaniem CPU (`is_x86_feature_detected!`). Przetwarza 8 ramek `f64` jednocześnie, z bezpiecznym powrotem do 4-wide `wide::f64x4` dla procesorów bez AVX-512.

---

### 2.4 Treści Audio i Baza Fabryczna (Factory Content & Presets)
*   **[x] Budowa Bazy Presetów Fabrycznych dla V-One Synth**:
    *   Stworzenie zestawu domyślnych wbudowanych brzmeiń (Lead, Bass, Pad, FX) zapisanych jako pliki `.json` (z rozszerzeniem `.vone`).
    *   Zweryfikowanie mechanizmu ich automatycznego ładowania podczas inicjalizacji syntezatora.
*   **[x] Weryfikacja Mod Matrix w Praktyce**:
    *   Dodano wygładzanie czestotliwości odcięcia (1-pole IIR low-pass filter smoothing `cutoff_smooth` w `synth.rs`), które eliminują cyfrowe kliknięcia i szumy ("zipper noise") podczas szybkiej modulacji LFO -> Cutoff.

---

### 2.5 Długoterminowa Infrastruktura (Advanced DAW Features)
*   **[x] Przetwarzanie Rozproszone Wtyczek (Remote Plugin Processing)**:
    *   Zaimplementowano moduł `remote_dsp.rs` z protokołem pakietowym `DspNetworkPacket`, wykrywaniem węzłów LAN i dystrybucją obciążenia DSP między maszynami w sieci lokalnej.
*   **[x] Wyszukiwanie Semantyczne Dźwięków (ML Sound Similarity)**:
    *   Zaimplementowano moduł `sound_similarity.rs` tworzący 16-wymiarowe osadzenia cech akustycznych (ZCR, RMS Energy, Filterbank) oraz indeks wyszukiwania próbek oparty na odległości podobieństwa kosinusowego.

---

### 2.6 Strategiczny Plan Rozwoju Komercyjnego (v0.6.0 → v1.0.0 Commercial Master Plan)

> [!NOTE]
> Zgodnie z decyzją strategiczną (na bazie danych telemetrycznych z rynku DAW), zarzucono dedykowany interfejs VR/AR na rzecz 4 filarów rynkowych: Uniwersalna Modulacja, Lokalny AI DSP, Kolaboracja w Chmurze/CRDT oraz Immersyjny Mastering Studio.

```
[STAN AKTUALNY] v0.5.0-beta (Rdzeń AVX-512 & ARA2)
       │
       ├──────────────────────────────────────────────┐
       ▼                                              ▼
   FAZA 1 (v0.6.0)                                FAZA 2 (v0.7.0)
Modulacja & Hybrydowy Workflow                Lokalny Silnik AI/DSP
- Lock-Free UnMod System                      - Kropelka-Extract HTDemucs Stem Separation
- Matryca Clip Launcher (WebGL)               - Wyszukiwarka Sampli Vector DB (HNSW + CLAP)
- Konwerter Sesji DAW (.als/.flp/.rpp)        - Edytor Strojenia VibeTune (YIN/PSOLA)
       │                                              │
       └──────────────────────┬───────────────────────┘
                              ▼
                       FAZA 3 (v0.8.0)
               Kolaboracja & Odporność Silnika
               - Synchroniczne CRDT P2P & Strumieniowanie Stems
               - Wersjonowanie w Stylu Git & Deltas Stems
               - Silnik Bezgłowy VIBE CLI (Cloud Render Farm)
                              │
                              ▼
                       FAZA 4 (v0.9.0)
               Immersyjny Mastering & Eksport
               - Eksport Metadanych ADM BWF Dolby Atmos
               - Mierniki EBU R128 & True-Peak Limiter (AVX-512)
               - Wizualne Płótno Naprawy Widmowej GPU (RX Alternative)
                              │
                              ▼
                       FAZA 5 (v1.0.0)
             Abstrakcja Sprzętowa & Wydanie Komercyjne
               - Niskopoziomowe Audio Sieciowe (AES67 / Dante / PipeWire)
               - Izolowane Wtyczki VST3 w IPC-shm (<50ms Auto-Recovery)
               - Otwarty Ekosystem Wtyczek WASM & SDK
```

---


### 2.7 Kompleksowa Dokumentacja Projektu (Documentation Sprint)
*   **[x] Przegląd Architektury (Architecture Overview)**:
    *   Stworzono zaktualizowany opis architektury z diagramami Mermaid przedstawiającymi przepływ sygnału audio oraz zdarzeń IPC między silnikiem w Rust a interfejsem w React (`Documents/ARCHITECTURE.md`).
*   **[x] Dokumentacja API**:
    *   Sporządzono dokumentację komend Tauri 2.0 i struktury magistrali zdarzeń (`Documents/API_REFERENCE.md`).
*   **[x] Podręcznik Użytkownika (User Manual)**:
    *   Stworzono kompleksowy podręcznik użytkownika obejmujący nawigację, miksowanie, edycję MIDI, syntezator V-One oraz asystenta Kropelka AI (`Documents/USER_MANUAL.md`).

---

### 2.8 Plan Rozwoju do Pełnej Wersji Studio 1.0 (Professional DAW Gaps & Finish Roadmap)
*   **[x] Faza 1: Comping, Take Lanes & Track Freeze (Critical P0)**:
    *   **Take Lanes & Swipe Comping**: Nagrywanie wielokrotnych podejść w pętli do pod-ścieżek oraz płynne łączenie optymalnych fragmentów narzędziem Swipe Comping (`comping.rs`, `TakeLanes.tsx`) z automatycznymi krosfejdami 10ms.
    *   **Track Freeze & Bounce-in-Place**: Renderowanie obciążających wtyczek w tle do 32-bit float PCM w celu zwolnienia CPU z możliwością bezstratnego odzamrożenia oraz powielania do nowej ścieżki (`freeze.rs`).
    *   **ARA2 Host Bridge**: Bezpośredni protokół bi-directional do integracji z Celemony Melodyne, VocAlign oraz iZotope RX (`ara2_bridge.rs`).
*   **[x] Faza 2: Zaawansowane Miksowanie & Time Warp (High P1)**:
    *   **Tłumiki VCA i Grupy DCA**: Dedykowane komendy Tauri (`commands/vca.rs`) oraz interfejs suwaków VCA Master w mikserze.
    *   **Silnik Time Warp & Transient Auto-Detection**: Detekcja transientów na strumieniu audio oraz algorytm WSOLA (`warp_engine.rs`) do płynnej zmiany tempa 50%-200% bez zmiany wysokości dźwięku.
    *   **Sidechain Spectrum Overlay**: Analizator widmowy 128 pasm (`SpectrumOverlay.tsx`) porównujący 2 ścieżki i podświetlający strefy kolizji częstotliwościowych (> -24dB, delta < 3dB).
*   **[x] Faza 3: Integracja Sprzętowa & Immersyjne Audio (Medium P2)**:
    *   **Protokoły MCU / HUI / MIDI Learn**: Silnik `mcu_protocol.rs` ze stanem powierzchni `McuDeviceState`, informacją zwrotną dla 14-bitowych zmotoryzowanych tłumików (Pitch Bend) oraz pierścieni LED V-Pot (CC 48-55).
    *   **Przestrzenny Panner 3D / Dolby Atmos 7.1.4**: Moduł `spatial_panner.rs` i komponent `SpatialPanner.tsx` z trójwymiarowym pozycjonowaniem VBAP (L, R, C, LFE, Ls, Rs, Ltb, Rtb, Ltf, Rtf) oraz symulacją binauralną HRTF (ITD/ILD).

---

### 2.9 Jakość Techniczna, Bezpieczeństwo i Precyzja DSP (Security & Quality Hardening)
*   **[x] Ochrona przed Traversal Ścieżek i Iniekcjami (`security_utils.rs`)**:
    *   Wdrożono kanoniczną walidację ścieżek `PathSecurity::validate_safe_path()` uniemożliwiającą ucieczkę z katalogu roboczego (`..`) oraz iniekcje bajtów zerowych (`\0`).
*   **[x] Bezpieczeństwo Deserializacji Projektu (`persistence.rs`)**:
    *   Wprowadzono limit rozmiaru danych projektu (500MB `MAX_PROJECT_DATA_SIZE`) oraz weryfikację sum kontrolnych CRC32 nagłówka przed deserializacją.
*   **[x] Izolacja Pamięci Wtyczek WASM i Ochrona Denormalna DSP**:
    *   Zweryfikowano granice pamięci liniowej WASM oraz zastosowano czyszczenie podnormalne (`flush_denormal_f64()`) we wszystkich pętlach DSP.

---

## 📈 3. Weryfikacja Jakościowa (Quality Checklist)

Przy wdrażaniu powyższych zadań należy bezwzględnie przestrzegać następujących reguł:
1.  **Test-Driven Execution**: Każde usprawnienie w kodzie Rust powinno być poparte testem jednostkowym lub integracyjnym w `vibe_v2_tests.rs`.
2.  **Linting & Compilation**: Brak ostrzeżeń (warnings) w kompilacji `cargo clippy` oraz zero błędów w `tsc --noEmit`.
3.  **Real-Time Safety**: Brak alokacji pamięci na stercie (heap allocations) oraz blokad (mutex locks) wewnątrz wątku audio.
