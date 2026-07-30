import { invoke } from '@tauri-apps/api/core';

export interface AiInsight {
    category: 'Mixing' | 'Mastering' | 'Arrangement' | 'Theory' | 'Technical' | 'Plugin' | 'Sound Design' | 'Vibe' | 'AI';
    message: string;
    severity: number; // 0 to 1
    targetElement?: string;
    choices?: string[];
    action?: string;
    pluginDetails?: {
        name: string;
        type: string;
        brand: string;
        vibe: string;
    };
}

export const PLUGIN_DATABASE: Record<string, any[]> = {
    compressors: [
        { name: "UAD 1176", brand: "Universal Audio", type: "FET Compressor", vibe: "Fast, aggressive, adds character and punch. Great on drums and vocals." },
        { name: "LA-2A", brand: "Teletronix", type: "Opto Leveler", vibe: "Smooth, musical, perfect for bass and soft vocal leveling." },
        { name: "CL 1B", brand: "Softube", type: "Tube Compressor", vibe: "Warm, creamy, the gold standard for rap vocals." },
        { name: "Distressor", brand: "Slate Digital", type: "VCA/FET Hybrid", vibe: "Versatile beast, can be clean or grit-heavy." },
        { name: "Pro-C 2", brand: "FabFilter", type: "Modern Compressor", vibe: "Swiss-army knife, transparent, versatile sidechaining." },
        { name: "TrackComp 2", brand: "DMG Audio", type: "Digital Emulation", vibe: "Incredibly precise modeling of classic analog gear." },
        { name: "Kotelnikov GE", brand: "Tokyo Dawn", type: "Mastering Compressor", vibe: "Pristine transparent dynamic control." },
        { name: "SSL G-Bus", brand: "SSL", type: "VCA Compressor", vibe: "The 'Glue' that brings a mix together on the master bus." }
    ],
    limiters: [
        { name: "Pro-L 2", brand: "FabFilter", type: "Mastering Limiter", vibe: "Transparent, flexible, industry standard for finishing." },
        { name: "Limitless", brand: "DMG Audio", type: "Multiband Limiter", vibe: "Incredibly loud without distortion, clinical." },
        { name: "Ozone Maximizer", brand: "iZotope", type: "Intelligent Limiter", vibe: "Smart ceiling control with IRC technology." },
        { name: "bx_limiter True Peak", brand: "Brainworx", type: "True Peak Limiter", vibe: "Focuses on preventing ISP distortion in digital masters." },
        { name: "L2 / L3", brand: "Waves", type: "Classic Limiter", vibe: "The sound of early 2000s loudness wars, still punchy." }
    ],
    equalizers: [
        { name: "Pro-Q 3", brand: "FabFilter", type: "Digital/Dynamic EQ", vibe: "Surgical precision, dynamic bands, workflow king." },
        { name: "Equilibrium", brand: "DMG Audio", type: "Digital EQ", vibe: "The ultimate flexible EQ for mastering." },
        { name: "Pultec EQP-1A", brand: "Pulse Techniques", type: "Analog Tube EQ", vibe: "Low-end magic, silky highs, the 'un-muddifier'." },
        { name: "Neve 1073", brand: "Neve", type: "Analog Preamp/EQ", vibe: "The classic 'British' sound, harmonic richness and punch." },
        { name: "Maag EQ4", brand: "Maag Audio", type: "Air Band EQ", vibe: "Unmatchable high-frequency airiness." },
        { name: "SSL E-Channel", brand: "SSL", type: "Analog Console EQ", vibe: "Aggressive, tight, standard for rock and pop drums." }
    ],
    dynamic_control: [
        { name: "Pro-MB", brand: "FabFilter", type: "Multiband Compressor", vibe: "Modern interface, phase linear options, extreme control." },
        { name: "DynOne", brand: "Leapwing", type: "Parallel Multiband", vibe: "Lush, wide, incredible for mastering and drum buses." },
        { name: "Nova GE", brand: "Tokyo Dawn", type: "Dynamic EQ", vibe: "Parallel dynamic equalization, very musical." },
        { name: "Transient Designer", brand: "SPL", type: "Transient Shaper", vibe: "The original punch-maker, simple attack and sustain controls." },
        { name: "Soothe2", brand: "Oeksound", type: "Dynamic Resonance Control", vibe: "Magic harshness remover, essential for modern vocals." }
    ],
    reverbs: [
        { name: "VintageVerb", brand: "Valhalla", type: "Algorithmic Reverb", vibe: "Inspired by classic digital hardware, massive sound." },
        { name: "Seventh Heaven", brand: "LiquidSonics", type: "Convolution Reverb", vibe: "Bricasti emulation, the most realistic room and plate spaces." },
        { name: "Pro-R", brand: "FabFilter", type: "Algorithmic Reverb", vibe: "Natural, intelligently designed tail controls." },
        { name: "Blackhole", brand: "Eventide", type: "Creative Reverb", vibe: "Massive, evolving ambient spaces." },
        { name: "Adaptiverb", brand: "Zynaptiq", type: "Harmonic Reverb", vibe: "Resynthesizes reverb tails based on input harmonics." }
    ],
    delays: [
        { name: "EchoBoy", brand: "Soundtoys", type: "Analog Delay Emulator", vibe: "The ultimate delay unit, recreates every classic echo ever." },
        { name: "Timeless 3", brand: "FabFilter", type: "Creative Delay", vibe: "Modulation heavy, tape pitch effects, limitless." },
        { name: "Galaxy Tape Echo", brand: "UAD", type: "Tape Delay", vibe: "Gritty, wobbly Space Echo emulation." }
    ],
    saturation: [
        { name: "Decapitator", brand: "Soundtoys", type: "Analog Saturation", vibe: "The best for adding grit, edge, and analog flavor." },
        { name: "Saturn 2", brand: "FabFilter", type: "Multiband Saturation", vibe: "Complex harmonic shaping across the spectrum." },
        { name: "Studer A800", brand: "UAD", type: "Tape Emulation", vibe: "Glue, warmth, and low-end bump." },
        { name: "HG-2", brand: "Black Box", type: "Tube Saturation", vibe: "Incredible mastering saturation for loudness and harmonics." }
    ],
    synths: [
        { name: "Serum", brand: "Xfer", type: "Wavetable Synth", vibe: "Limitless modulation, clear sound, sound design powerhouse." },
        { name: "Diva", brand: "u-he", type: "Analog Emulation", vibe: "Authentic hardware warmth, cpu-intensive but stunning." },
        { name: "Omnisphere", brand: "Spectrasonics", type: "Hybrid Synth", vibe: "Infinite soundscapes, massive library, cinematic quality." },
        { name: "Phase Plant", brand: "Kilohearts", type: "Modular Synth", vibe: "Incredible routing, modern sound design dream." },
        { name: "Pigments", brand: "Arturia", type: "Polychrome Synth", vibe: "Granular, analog, and wavetable with beautiful visual feedback." }
    ],
    spatial: [
        { name: "Ozone Imager", brand: "iZotope", type: "Stereo Imager", vibe: "Multiband widening without phasing." },
        { name: "Mid/Side", brand: "Goodhertz", type: "M/S Matrix", vibe: "Surgical control over the mid and side channels." },
        { name: "Stereoizer", brand: "Nugen", type: "Spatial Enhancer", vibe: "Mono-compatible widening." }
    ],
    drums: [
        { name: "Superior Drummer 3", brand: "Toontrack", type: "Acoustic Drums", vibe: "The most realistic room recordings available." },
        { name: "Battery 4", brand: "Native Instruments", type: "Drum Sampler", vibe: "Electronic and hip-hop standard, fast workflow." },
        { name: "Addictive Drums 2", brand: "XLN Audio", type: "Acoustic Drums", vibe: "Mix-ready kits with great built-in effects." }
    ],
    creative_fx: [
        { name: "ShaperBox 3", brand: "Cableguys", type: "Modulation FX", vibe: "Volume ducking, time-twisting, filtering on a curve." },
        { name: "Movement", brand: "Output", type: "Rhythmic FX", vibe: "Turns any sound into a rhythmic pulse." },
        { name: "Effectrix", brand: "Sugar Bytes", type: "Sequenced FX", vibe: "Classic glitch and stutter effects." }
    ],
    utility: [
        { name: "Insight 2", brand: "iZotope", type: "Analysis Suite", vibe: "Everything you need to see about your mix." },
        { name: "Youlean Loudness Meter", brand: "Youlean", type: "LUFS Meter", vibe: "The standard for checking streaming platform loudness targets." },
        { name: "Span", brand: "Voxengo", type: "Spectrum Analyzer", vibe: "Free, accurate, indispensable for checking frequency balance." }
    ]
};

class AiAssistService {
    private listeners: ((insight: AiInsight) => void)[] = [];

    // UI Context State
    public focusedPlugin: string | null = null;
    public focusedParameter: string | null = null;
    public recentChanges: { paramName: string, value: number, time: number }[] = [];

    public setFocusedElement(plugin: string | null, parameter: string | null = null) {
        this.focusedPlugin = plugin;
        this.focusedParameter = parameter;
    }

    public recordParameterChange(paramName: string, value: number) {
        this.recentChanges.unshift({ paramName, value, time: Date.now() });
        if (this.recentChanges.length > 5) {
            this.recentChanges.pop();
        }
    }

    public getContextData(): string {
        // Clear old recent changes (> 10 seconds)
        const now = Date.now();
        this.recentChanges = this.recentChanges.filter(c => (now - c.time) < 10000);

        return JSON.stringify({
            focus: this.focusedPlugin ? `${this.focusedPlugin}${this.focusedParameter ? ':' + this.focusedParameter : ''}` : 'None',
            recentChanges: this.recentChanges.map(c => `${c.paramName}:${c.value.toFixed(2)}`)
        });
    }

    public onInsight(callback: (insight: AiInsight) => void) {
        this.listeners.push(callback);
    }

    public async providePluginTip() {
        const categories = Object.keys(PLUGIN_DATABASE);
        const cat = categories[Math.floor(Math.random() * categories.length)];
        const list = PLUGIN_DATABASE[cat];
        const plugin = list[Math.floor(Math.random() * list.length)];

        this.broadcast({
            category: 'Plugin',
            message: `Engineering Tip: The ${plugin.name} is a ${plugin.type}. It's legendary for its ${plugin.vibe}`,
            severity: 0.2,
            pluginDetails: plugin
        });
    }

    public async analyzeMixing(trackIdx: number) {
        this.broadcast({
            category: 'Mixing',
            message: "This vocal track could use some serial compression. Try a fast FET like the 1176 followed by a slow Opto like the LA-2A.",
            severity: 0.4
        });
    }

    public async checkMastering(masterLevel: number) {
        if (masterLevel > 0.99) {
            this.broadcast({
                category: 'Mastering',
                message: "Clipping detected! We need more headroom. Maybe push into a Pro-L 2 or use some soft-clipping for saturation.",
                severity: 0.9,
                targetElement: 'vibe-master-meters'
            });
        }
    }

    public async suggestSoundDesign() {
        this.broadcast({
            category: 'Sound Design',
            message: "Try adding some Decapitator for parallel saturation on your drums to get that 90s grit.",
            severity: 0.3
        });
    }

    public async provideMixingGuidance() {
        this.broadcast({
            category: 'Mixing',
            message: "Common practice: EQ before compression to remove mud, or after to restore lost frequencies. Experiment with both!",
            severity: 0.2
        });
    }

    public async analyzeMidSide() {
        this.broadcast({
            category: 'Mixing',
            message: "Maybe try cleaning up the side signals below 200Hz to tighten up the stereo image?",
            severity: 0.4
        });
    }

    public async suggestArrangement(trackCount: number = 0) {
        this.broadcast({
            category: 'Arrangement',
            message: "Consider adding an ear-candy layer here—maybe a reversed delay tail or some foley textures.",
            severity: 0.2
        });
    }

    public async provideDeepKnowledge(context: string = 'General') {
        try {
            const tip = await invoke<any>('get_assistant_knowledge_tip', { context });
            this.broadcast({
                category: tip.category as any,
                message: `Deep Dive: ${tip.title} - ${tip.body}`,
                severity: tip.importance,
            });
        } catch (e) {
            console.error("Assistant knowledge failed:", e);
        }
    }

    public async provideLlmInsight() {
        try {
            const context = JSON.stringify({
                projectState: this.recentChanges.length > 0 ? 'Mixing' : 'General',
                uiData: { focus: this.focusedPlugin || 'None' }
            });
            const insight: any = await invoke('get_kropelka_suggestion', { context });

            if (insight) {
                this.broadcast({
                    category: insight.category || 'AI',
                    message: insight.text,
                    severity: 0.8
                });
            }
        } catch (e) {
            console.error("LLM Insight failed:", e);
        }
    }

    private broadcast(insight: AiInsight) {
        this.listeners.forEach(l => l(insight));
    }

    public async getStats(): Promise<any> {
        try {
            return await invoke('get_kropelka_stats');
        } catch (e) {
            console.error("Failed to get stats:", e);
            return null;
        }
    }

    public async resetMemory(): Promise<void> {
        try {
            await invoke('reset_kropelka_memory');
        } catch (e) {
            console.error("Failed to reset memory:", e);
        }
    }
}

export const aiAssistant = new AiAssistService();
