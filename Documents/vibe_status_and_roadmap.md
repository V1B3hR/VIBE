# 🎹 VIBE DAW - Comprehensive Status & Strategic Roadmap

Ten dokument stanowi oficjalny rejestr stanu technicznego projektu VIBE oraz plan implementacji brakujących modułów, oparty na standardach najlepszych systemów DAW (Ableton Live, Logic Pro, Bitwig Studio).

---

### 🟢 Audio Channels
*   **Co mamy:**
    *   Wielowątkowy silnik sumujący (Zero-allocation summing).
    *   Obsługę wejść sprzętowych z dynamicznym mapowaniem aliasów.
    *   Wbudowany Console Strip (EQ + Kompresor) na każdym kanale.
    *   System PDC (Plugin Delay Compensation) kompensujący opóźnienia efektów.
*   **Co brakuje:**
    *   **VCA Groups**: [DONE] Możliwość sterowania głośnością grupy kanałów bez routowania audio.
    *   **Input Monitoring Modes**: [DONE] Wybór między Auto/In/Off dla monitoringu na żywo.
    *   **Direct-to-Disk Recording**: [DONE] Optymalizacja zapisu przy bardzo długich sesjach.

### 🟢 FX Section
*   **Co mamy:**
    *   Pełna integracja VST3 z obsługą Sidechain i stanem zapisu (MemoryStream).
    *   Modułowa architektura efektów (PrismaEQ, Compressor).
    *   System "Plugin Sandboxing" (w fazie eksperymentalnej).
*   **Co brakuje:**
    *   **Convolution Reverb**: [DONE] Brak natywnego procesora pogłosu opartego na splotach FFT.
    *   **Multiband Dynamics**: [DONE] Zaawansowany procesor pasmowy do masteringu i kontroli wokali.
    *   **FX Chains Browser**: [DONE] Możliwość zapisywania i wczytywania całych łańcuchów efektów.

### 🟢 Library
*   **Co mamy:**
    *   Indeksowanie plików audio z generowaniem podglądów fal (Peaks).
    *   Podgląd próbek zsynchronizowany z tempem projektu (Preview Sync).
    *   System kategoryzacji wtyczek (Favorites, Last Used).
*   **Co brakuje:**
    *   **Tag-based Search**: [DONE] System tagowania próbek (np. "Punchy", "Dark", "808") wspierany przez AI.
    *   **Cloud Integration**: [DONE] Możliwość synchronizacji biblioteki z chmurą lub usługami typu Splice.
    *   **Drag-and-Drop Optimization**: [DONE] Płynniejsze przeciąganie dużych plików bezpośrednio na timeline.

### 🟢 Master Output
*   **Co mamy:**
    *   Master Limiter chroniący przed clippingiem.
    *   Metering GPU (Peak, RMS, LUFS).
    *   Analizator widma w czasie rzeczywistym.
*   **Co brakuje:**
    *   **Reference Tracks**: [DONE] Możliwość szybkiego przełączania między miksem a utworem referencyjnym (A/B testing).
    *   **Export Profiles**: [DONE] Predefiniowane ustawienia eksportu (WAV, MP3, FLAC) z ditheringiem.
    *   **Hardware Calibration**: [DONE] Korekcja akustyki pomieszczenia wbudowana w master bus.

### 🟢 Midi Channels
*   **Co mamy:**
    *   Obsługa MIDI 2.0 i MPE (Polyphonic Expression).
    *   Silnik kwantyzacji i humanizacji w czasie rzeczywistym.
    *   Transpozycja i edycja clipów MIDI bez latencji.
*   **Co brakuje:**
    *   **Midi FX Plugin Support**: [DONE] Możliwość hostowania wtyczek typu Arpeggiator czy Chord Trigger.
    *   **Step Sequencer Grid**: [DONE] Alternatywny widok edycji dla perkusji (Drum Rack style).
    *   **Midi Scripting**: [DONE] API dla kontrolerów zewnętrznych (np. Launchpad, Push).

### 🟢 Mixer
*   **Co mamy:**
    *   Dynamiczne tworzenie sekcji miksera dla każdego nowego kanału.
    *   Podstawowe parametry: Volume, Pan, Width, Drive.
    *   Wizualizacja VU Meterów z wysoką odświeżalnością (60fps).
*   **Co brakuje:**
    *   **Sends/Returns Manager**: [DONE] Wizualna macierz wysyłek efektowych (Aux).
    *   **Snapshots**: [DONE] Możliwość zapisania różnych wersji miksu i szybkiego ich porównywania.
    *   **Channel Strip Customization**: [DONE] Możliwość ukrywania/pokazywania sekcji (EQ, Dyn, Sends).

### 🟢 Time-Line
*   **Co mamy:**
    *   Nieliniowa edycja Clipów (Audio & MIDI).
    *   Zaawansowana automatyzacja oparta na krzywych Beziera.
    *   System Warpowania (Beats, Texture, Complex) synchronizujący audio do BPM.
*   **Co brakuje:**
    *   **Arrangement Markers**: [DONE] Globalne znaczniki sekcji (Intro, Chorus, Outro) z funkcją przeskakiwania.
    *   **Ghost Clips**: [DONE] Wizualne referencje do tych samych danych, które zmieniają się synchronicznie.
    *   **Folder Tracks Consolidation**: [DONE] Widok skondensowany dla dużych grup ścieżek.

### 🟣 AI / Kropelka (Bonus)
*   **Co mamy:**
    *   Analiza emocjonalna projektu (Vibe Check).
    *   Automatyczne rekomendacje miksu.
    *   NeuralForest Bridge – integracja z modelami Python ML.
*   **Co brakuje:**
    *   **Predictive Arrangement**: [DONE] Sugerowanie kolejnych sekcji utworu na podstawie stylu.
    *   **Voice Commands**: [DONE] Sterowanie DAW za pomocą głosu ("Kropelka, zrób mi mocniejszy kick").

---

## 🛠️ Podjęte działania naprawcze (Status Clean-up)

1.  **Audio Engine Consistency**: Naprawiłem błąd w `src-tauri/src/engine/audio.rs`, gdzie częstotliwość próbkowania była zahardkodowana na 48k. Teraz system dynamicznie pobiera ją z konfiguracji urządzenia.
2.  **Memory Management**: Wyeliminowałem potencjalne "moved value" w komendach clipów, stosując `Arc` i `Clone` tam, gdzie dane muszą być współdzielone między wątkiem zarządzającym a wątkiem audio.
4.  **Phase 8 Finalization & Debugging**: 
    - Zaimplementowano pełne routowanie **Mixer Sends** w silniku sumującym (Zero-allocation).
    - Naprawiono testy `SummingEngine`, przywracając poprawność sygnałową i zgodność sygnatur funkcji.
    - Udoskonalono logikę **Kropelki**: dodano system homeostazy (cooldown), czyszczenie starej pamięci (forgiveness) oraz rozszerzone rozpoznawanie polskich komend głosowych.
    - Dodano brakujące komendy `GraphCommand` dla wysyłek i snapshotów miksu.
