# 💧 Kropelka: Vision & Permissions Architecture
## Filozofia: "Zosia Samosia, ale nie upierdliwa"

Kropelka w wersji docelowej (v4.0+) przestaje być tylko "spinaczem" z podpowiedziami. Staje się autonomicznym inżynierem systemu VIBE, działającym w tle. Jej cel to **Utrzymanie Flow** – użytkownik ma tworzyć muzykę, a nie walczyć z systemem.

---

## 🔐 Model Uprawnień (Permission Tiers)

Aby zachować zaufanie użytkownika, Kropelka posiada ściśle zdefiniowane poziomy dostępu.

### Poziom 1: Obserwator (The Silent Observer)
*Dostęp domyślny. Kropelka widzi wszystko, ale nie dotyka.*
- [x] **System**: Monitorowanie CPU, RAM, Latencji.
- [x] **Projekt**: Analiza struktury utworu, tonacji (Key), tempa (BPM).
- [x] **Audio**: Analiza spektralna, wykrywanie clippingu, poziomu LUFS.
- [x] **Multi-language**: Obsługa 7 języków (PL, EN, IT, ES, DE, FR, ZH). 
- [x] **Local Voice**: 100% offline, brak telemetrii.

### Poziom 2: Gosposia (The Housekeeper)
*Działania porządkowe, które nie wpływają na brzmienie, ale ułatwiają życie.*

- [x] **1. Smart Foldering**: Auto-foldery (Drums, Bass, Vocals itp.) na podstawie nazw/contentu.
- [x] **2. Dead Track Cleaner**: Wykrywanie pustych ścieżek, wyciszonych klipów i pluginów bez I/O.
- [x] **3. Plugin Dusting**: Wyszukiwanie nieużywanych lub bypassowanych od 20 min pluginów.
- [x] **4. Clip Tidy**: Przycinanie pustych końcówek, usuwanie fragmentów < 50ms, wyrównywanie do siatki.
- [x] **5. Timeline Organizer**: Wyrównywanie markerów, autolabeling (Intro, Drop), porządkowanie automatyki.
- [x] **6. Sample Librarian**: Duplikaty, relinkowanie i usuwanie nieużywanych sampli z projektu.
- [x] **7. Automation Cleaner**: Usuwanie płaskich linii i "szumów" automatyki MIDI.
- [x] **8. Track Role Detection**: Rozpoznawanie roli (Lead, Pad, Bass) i ustawianie ikon/kolorów.
- [x] **Smart Backup Rotation**: Snapshoty co 5/30/60 min oraz sesyjne bez zapychania dysku.

### Poziom 3: Technik (The System Guardian)
*Naprawia problemy techniczne zanim użytkownik zdąży się zdenerwować.*

- [x] **Panic Guard (CRITICAL)**: Natychmiastowy Hard Limiter przy skokach > +6dB (Auto-engage).
- [x] **9. Project Health Monitor**: Monitoring CPU/RAM/Dysku/Latencji pluginów z alertami.
- [x] **10. Freeze Advisor**: Sugestie zamrażania ścieżek zjadających CPU (>15%).
- [x] **11. Dependency Checker**: Sprawdzanie brakujących pluginów i wersji (VST2 vs VST3).
- [x] **12. Project Integrity Scan**: "SFC /scannow" dla projektu - uszkodzone klipy, osierocone dane.
- [x] **13. Crash Recovery Guardian**: Snapshoty przed importem bibliotek i stabilizacja starych projektów.
- [x] **14. Silence Sweeper**: Wykrywanie i usuwanie długich odcinków ciszy/niepotrzebnych taili.
- [x] **15. Latency Control**: Automatyczna zmiana Buffer Size (Recording vs Mixing).

### Poziom 4: Co-Pilot (Creative Partner)
*Ingerencja w materiał twórczy. Zawsze wymaga potwierdzenia (Advisory Mode).*

- [x] **Smart Theory Advisor**: Akordy, Modulacje, CoF, Negative Harmony.
- [x] **Gain Staging & Mixing**: De-masking (Smart EQ), de-mudding i sugestie LUFS.
- [x] **Groove Insights**: Sugerowanie swingu (MPC 60) dla "sztywnych" rytmów.
- [x] **Adaptive Suggestions**: Sugestie zależne od umiejętności. Jeśli loop nie pasuje, Kropelka sugeruje nowy.
- [x] **Melody Inpainting**: Dokończenie melodii na podstawie istniejących nut.
- [x] **Chord Progression Wizard**: Sugerowanie następnych akordów w progresji.
- [x] **Groove Genetix**: Generowanie partii perkusyjnych od podstaw.

---

## 🤖 Tryby Interakcji: "Nie upierdliwa"

### 1. Silent Mode
*Techniczne naprawy w tle. Tylko zielona dioda Health.*

### 2. Advisory Mode [DOMYŚLNY]
*Wszystkie sugestie wymagają akceptacji. Przyciski: [Applica] [Ignora] [Spiega].*

### 3. Emergency Mode
*Panic Guard! Ochrona hardware'u i zdrowia. Akcja natychmiastowa.*

---

## 🛠️ Implementacja (Plan Techniczny)

1. **KropelkaBrain (Rust)**: Rdzeń logiczny z bazą wiedzy JSON.
2. **MixAnalyzer**: Real-time DSP monitoring (RMS, Peak, LUFS, FFT).
3. **Localization Engine**: Dynamiczne wsparcie dla wielu języków (locales.json).
4. **ActionQueue**: Zabezpieczone IPC do silnika Audio przez CommandManager.
