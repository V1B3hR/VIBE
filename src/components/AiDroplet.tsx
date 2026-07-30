import React, { useRef, useMemo, useState, useEffect, Suspense } from 'react';
import { Canvas, useFrame, extend } from '@react-three/fiber';
import { shaderMaterial, Environment, PerspectiveCamera } from '@react-three/drei';
import * as THREE from 'three';
import gsap from 'gsap';
import { invoke } from '@tauri-apps/api/core';
import { aiAssistant, AiInsight } from '../services/AiAssistService';
import { KropelkaPermissions } from './KropelkaPermissions';
import './AiDroplet.css';

// 1. Procedural Living Liquid Shader with Real specular reflection
const DropletMaterial = shaderMaterial(
    {
        uTime: 0,
        uColorIdle: new THREE.Color('#3A9AD9'), // Beautiful sky blue from image
        uColorWarn: new THREE.Color('#ffaa00'),
        uColorCritical: new THREE.Color('#ff0033'),
        uSeverity: 0, // 0 to 1
        uEnergy: 0,
        uResolution: new THREE.Vector2()
    },
    // Vertex Shader
    `
    varying vec2 vUv;
    varying vec3 vNormal;
    varying vec3 vPosition;
    uniform float uTime;
    uniform float uEnergy;
    uniform float uSeverity;

    // Classic Perlin Noise
    vec3 mod289(vec3 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
    vec4 mod289(vec4 x) { return x - floor(x * (1.0 / 289.0)) * 289.0; }
    vec4 permute(vec4 x) { return mod289(((x*34.0)+1.0)*x); }
    vec4 taylorInvSqrt(vec4 r) { return 1.79284291400159 - 0.85373472095314 * r; }
    vec3 fade(vec3 t) { return t*t*t*(t*(t*6.0-15.0)+10.0); }

    float pnoise(vec3 P, vec3 rep) {
      vec3 Pi0 = mod(floor(P), rep); 
      vec3 Pi1 = mod(Pi0 + vec3(1.0), rep); 
      Pi0 = mod289(Pi0);
      Pi1 = mod289(Pi1);
      vec3 Pf0 = fract(P); 
      vec3 Pf1 = Pf0 - vec3(1.0); 
      vec4 ix = vec4(Pi0.x, Pi1.x, Pi0.x, Pi1.x);
      vec4 iy = vec4(Pi0.yy, Pi1.yy);
      vec4 iz0 = Pi0.zzzz;
      vec4 iz1 = Pi1.zzzz;

      vec4 ixy = permute(permute(ix) + iy);
      vec4 ixy0 = permute(ixy + iz0);
      vec4 ixy1 = permute(ixy + iz1);

      vec4 gx0 = ixy0 * (1.1 / 7.0);
      vec4 gy0 = fract(floor(gx0) * (1.1 / 7.0)) - 0.5;
      gx0 = fract(gx0);
      vec4 gz0 = vec4(0.5) - abs(gx0) - abs(gy0);
      vec4 sz0 = step(gz0, vec4(0.0));
      gx0 -= sz0 * (step(0.0, gx0) - 0.5);
      gy0 -= sz0 * (step(0.0, gy0) - 0.5);

      vec4 gx1 = ixy1 * (1.1 / 7.0);
      vec4 gy1 = fract(floor(gx1) * (1.1 / 7.0)) - 0.5;
      gx1 = fract(gx1);
      vec4 gz1 = vec4(0.5) - abs(gx1) - abs(gy1);
      vec4 sz1 = step(gz1, vec4(0.0));
      gx1 -= sz1 * (step(0.0, gx1) - 0.5);
      gy1 -= sz1 * (step(0.0, gy1) - 0.5);

      vec3 g000 = vec3(gx0.x,gy0.x,gz0.x);
      vec3 g100 = vec3(gx0.y,gy0.y,gz0.y);
      vec3 g010 = vec3(gx0.z,gy0.z,gz0.z);
      vec3 g110 = vec3(gx0.w,gy0.w,gz0.w);
      vec3 g001 = vec3(gx1.x,gy1.x,gz1.x);
      vec3 g101 = vec3(gx1.y,gy1.y,gz1.y);
      vec3 g011 = vec3(gx1.z,gy1.z,gz1.z);
      vec3 g111 = vec3(gx1.w,gy1.w,gz1.w);

      vec4 norm0 = taylorInvSqrt(vec4(dot(g000, g000), dot(g010, g010), dot(g100, g100), dot(g110, g110)));
      g000 *= norm0.x;
      g010 *= norm0.y;
      g100 *= norm0.z;
      g110 *= norm0.w;
      vec4 norm1 = taylorInvSqrt(vec4(dot(g001, g001), dot(g011, g011), dot(g101, g101), dot(g111, g111)));
      g001 *= norm1.x;
      g011 *= norm1.y;
      g101 *= norm1.z;
      g111 *= norm1.w;

      float n000 = dot(g000, Pf0);
      float n100 = dot(g100, vec3(Pf1.x, Pf0.yz));
      float n010 = dot(g010, vec3(Pf0.x, Pf1.y, Pf0.z));
      float n110 = dot(g110, vec3(Pf1.xy, Pf0.z));
      float n001 = dot(g001, vec3(Pf0.xy, Pf1.z));
      float n101 = dot(g101, vec3(Pf1.x, Pf0.y, Pf1.z));
      float n011 = dot(g011, vec3(Pf0.x, Pf1.yz));
      float n111 = dot(g111, Pf1);

      vec3 fade_xyz = fade(Pf0);
      vec4 n_z = mix(vec4(n000, n100, n010, n110), vec4(n001, n101, n011, n111), fade_xyz.z);
      vec2 n_yz = mix(n_z.xy, n_z.zw, fade_xyz.y);
      float n_xyz = mix(n_yz.x, n_yz.y, fade_xyz.x);
      return 2.2 * n_xyz;
    }

    void main() {
      vUv = uv;
      vNormal = normalize(normalMatrix * normal);
      vPosition = position;

      float noise = pnoise(position * 3.5 + uTime * (0.8 + uEnergy * 2.5), vec3(10.0));
      float displacement = noise * (0.04 + uEnergy * 0.12 + uSeverity * 0.18);
      vec3 newPosition = position + normal * displacement;

      gl_Position = projectionMatrix * modelViewMatrix * vec4(newPosition, 1.0);
    }
    `,
    // Fragment Shader
    `
    varying vec2 vUv;
    varying vec3 vNormal;
    varying vec3 vPosition;
    uniform float uTime;
    uniform vec3 uColorIdle;
    uniform vec3 uColorWarn;
    uniform vec3 uColorCritical;
    uniform float uSeverity;
    uniform float uEnergy;

    void main() {
      // Fresnel effect
      vec3 viewDirection = normalize(cameraPosition - vPosition);
      float fresnel = pow(1.0 - dot(viewDirection, vNormal), 3.0);
      
      // Dynamic color mixing
      vec3 baseColor;
      if (uSeverity < 0.5) {
        baseColor = mix(uColorIdle, uColorWarn, uSeverity * 2.0);
      } else {
        baseColor = mix(uColorWarn, uColorCritical, (uSeverity - 0.5) * 2.0);
      }

      // Specular highlights (simulating glass shininess from key light)
      vec3 lightDir = normalize(vec3(-1.5, 2.0, 1.5));
      vec3 reflectDir = reflect(-lightDir, vNormal);
      float spec = pow(max(dot(viewDirection, reflectDir), 0.0), 32.0);
      vec3 specularColor = vec3(1.0) * spec * 0.7;

      // Soft secondary fill specular reflection
      vec3 fillLightDir = normalize(vec3(1.5, -1.0, -1.0));
      vec3 fillReflectDir = reflect(-fillLightDir, vNormal);
      float fillSpec = pow(max(dot(viewDirection, fillReflectDir), 0.0), 12.0);
      vec3 fillSpecularColor = baseColor * fillSpec * 0.25;

      // Combine base color, fresnel glow and specular highlights
      vec3 finalColor = mix(baseColor * 0.65, baseColor * 1.4, fresnel);
      finalColor += specularColor + fillSpecularColor;
      
      // Add Energy based glow pulsing
      finalColor += vec3(0.05 + uEnergy * 0.25);

      // Transparency & Surface Glaze
      float opacity = 0.82 + fresnel * 0.18;
      
      gl_FragColor = vec4(finalColor, opacity);
    }
    `
);

extend({ DropletMaterial });

// 2. Reactive 3D Waveform Eye Component
interface WaveformEyeProps {
    position: [number, number, number];
    energy: number;
    mood: string;
    isRight?: boolean;
}

const WaveformEye = ({ position, energy, mood, isRight }: WaveformEyeProps) => {
    const barsRef = useRef<THREE.Group>(null);

    useFrame((state) => {
        if (!barsRef.current) return;
        const time = state.clock.elapsedTime;
        const children = barsRef.current.children;

        for (let i = 0; i < children.length; i++) {
            const bar = children[i] as THREE.Mesh;
            if (bar.name === 'horizontal-line') continue;

            let targetScaleY = 1.0;
            if (mood === 'critical' || mood === 'warn') {
                targetScaleY = 0.4 + Math.sin(time * 20 + i) * 0.2;
            } else if (mood === 'vibe_check') {
                targetScaleY = 0.7 + Math.sin(time * 12 + i * 1.2) * 0.5;
            } else if (mood === 'idle') {
                targetScaleY = 0.5 + Math.sin(time * 2 + i * 0.8) * 0.15;
            } else {
                // Creative or Flow: Equalizer dancing
                const wave = Math.sin(time * 8 + i * 1.5) * 0.4 + 0.6;
                targetScaleY = 0.4 + wave * (0.4 + energy * 2.0);
            }

            bar.scale.y = THREE.MathUtils.lerp(bar.scale.y, targetScaleY, 0.15);
        }
    });

    const barWidth = 0.015;
    const barSpacing = 0.035;
    const defaultHeights = [0.06, 0.13, 0.22, 0.13, 0.06];

    const getEyeColor = () => {
        if (mood === 'critical' || mood === 'warn') return '#ff3b30';
        if (mood === 'vibe_check') return '#00e676';
        if (mood === 'idle') return '#055E8D'; // Elegant deep dark blue
        return '#00f0ff';
    };

    const eyeColor = getEyeColor();

    return (
        <group position={position} rotation={[0, isRight ? -0.2 : 0.2, 0]}>
            {/* Horizontal line */}
            <mesh name="horizontal-line" position={[0, 0, 0]}>
                <boxGeometry args={[0.22, 0.014, 0.01]} />
                <meshBasicMaterial color={eyeColor} />
            </mesh>

            {/* Vertical bars */}
            <group ref={barsRef}>
                {defaultHeights.map((height, idx) => {
                    const xOffset = (idx - 2) * barSpacing;
                    return (
                        <mesh key={idx} position={[xOffset, 0, 0.005]}>
                            <boxGeometry args={[barWidth, height, 0.012]} />
                            <meshBasicMaterial color={eyeColor} />
                        </mesh>
                    );
                })}
            </group>
        </group>
    );
};

// 3. Torus Curved Mouth Component
interface TorusMouthProps {
    energy: number;
    mood: string;
}

const TorusMouth = ({ energy, mood }: TorusMouthProps) => {
    const mouthRef = useRef<THREE.Mesh>(null);

    useFrame((state) => {
        if (!mouthRef.current) return;
        const time = state.clock.elapsedTime;

        let targetScaleY = 1.0;
        let targetScaleX = 1.0;

        if (mood === 'creative' || mood === 'flow') {
            targetScaleY = 1.0 + energy * 0.3;
            targetScaleX = 1.0 - energy * 0.1;
        } else if (mood === 'critical') {
            targetScaleY = 1.0 + Math.sin(time * 15) * 0.15;
            targetScaleX = 1.0 + Math.sin(time * 15) * 0.15;
        }

        mouthRef.current.scale.y = THREE.MathUtils.lerp(mouthRef.current.scale.y, targetScaleY, 0.2);
        mouthRef.current.scale.x = THREE.MathUtils.lerp(mouthRef.current.scale.x, targetScaleX, 0.2);
    });

    const getMouthColor = () => {
        if (mood === 'critical' || mood === 'warn') return '#ff3b30';
        if (mood === 'vibe_check') return '#00e676';
        if (mood === 'idle') return '#055E8D';
        return '#00f0ff';
    };

    const mouthColor = getMouthColor();

    if (mood === 'critical') {
        return (
            <mesh ref={mouthRef} position={[0, -0.22, 0.94]}>
                <torusGeometry args={[0.06, 0.012, 8, 24]} />
                <meshBasicMaterial color={mouthColor} />
            </mesh>
        );
    }

    if (mood === 'warn') {
        return (
            <mesh ref={mouthRef} position={[0, -0.22, 0.94]}>
                <boxGeometry args={[0.13, 0.012, 0.012]} />
                <meshBasicMaterial color={mouthColor} />
            </mesh>
        );
    }

    const arcAngle = Math.PI * 0.65;
    const rotationZ = -Math.PI / 2 - arcAngle / 2;

    return (
        <mesh 
            ref={mouthRef} 
            position={[0, -0.22, 0.94]} 
            rotation={[0, 0, rotationZ]}
        >
            <torusGeometry args={[0.1, 0.012, 8, 32, arcAngle]} />
            <meshBasicMaterial color={mouthColor} />
        </mesh>
    );
};

// 4. Con-centric Expanding Floor Ripple Rings
const RippleFloor = ({ energy }: { energy: number }) => {
    const ringRef = useRef<THREE.Mesh>(null);
    const ring2Ref = useRef<THREE.Mesh>(null);

    useFrame((state) => {
        const time = state.clock.elapsedTime;
        
        if (ringRef.current) {
            const cycle1 = (time * 0.6) % 1.0;
            const scale1 = 0.4 + cycle1 * 1.6 + energy * 0.4;
            ringRef.current.scale.set(scale1, scale1, 1);
            const opacity1 = Math.max(0, (1.0 - cycle1) * 0.35);
            if (!Array.isArray(ringRef.current.material)) {
                ringRef.current.material.opacity = opacity1;
            }
        }

        if (ring2Ref.current) {
            const cycle2 = (time * 0.6 + 0.5) % 1.0;
            const scale2 = 0.4 + cycle2 * 1.6 + energy * 0.4;
            ring2Ref.current.scale.set(scale2, scale2, 1);
            const opacity2 = Math.max(0, (1.0 - cycle2) * 0.35);
            if (!Array.isArray(ring2Ref.current.material)) {
                ring2Ref.current.material.opacity = opacity2;
            }
        }
    });

    return (
        <group position={[0, -1.35, 0.2]} rotation={[-Math.PI / 2, 0, 0]}>
            <mesh ref={ringRef}>
                <ringGeometry args={[0.1, 0.7, 32]} />
                <meshBasicMaterial color="#00b4d8" transparent opacity={0.2} side={THREE.DoubleSide} />
            </mesh>
            <mesh ref={ring2Ref}>
                <ringGeometry args={[0.1, 0.7, 32]} />
                <meshBasicMaterial color="#00e5ff" transparent opacity={0.2} side={THREE.DoubleSide} />
            </mesh>
        </group>
    );
};

// 5. Face wrapper component mapping coordinates on front of droplet
const Face = ({ mood, energy }: { mood: string; energy: number }) => {
    const group = useRef<THREE.Group>(null);

    useFrame((state) => {
        if (!group.current) return;

        const targetX = state.mouse.x * 0.15;
        const targetY = state.mouse.y * 0.15;

        group.current.position.x = THREE.MathUtils.lerp(group.current.position.x, targetX, 0.1);
        group.current.position.y = THREE.MathUtils.lerp(group.current.position.y, targetY, 0.1);

        if (mood === 'critical') {
            const shake = Math.sin(state.clock.elapsedTime * 25.0) * 0.025;
            group.current.position.x += shake;
        }
    });

    return (
        <group ref={group}>
            {/* Eyes */}
            <WaveformEye position={[-0.22, 0.18, 0.80]} energy={energy} mood={mood} />
            <WaveformEye position={[0.22, 0.18, 0.80]} energy={energy} mood={mood} isRight />
            
            {/* Mouth */}
            <TorusMouth energy={energy} mood={mood} />
        </group>
    );
};

// 6. DropletEntity deforming sphere geometry into teardrop
const DropletEntity = ({ severity, energy, mood }: { severity: number, energy: number, mood: string }) => {
    const mesh = useRef<THREE.Mesh>(null);
    const matRef = useRef<any>(null);

    const dropletGeometry = useMemo(() => {
        const geom = new THREE.SphereGeometry(1.0, 64, 64);
        const pos = geom.attributes.position;
        for (let i = 0; i < pos.count; i++) {
            let x = pos.getX(i);
            let y = pos.getY(i);
            let z = pos.getZ(i);

            if (y > -0.4) {
                const t = (y - (-0.4)) / 1.4;
                const taper = 1.0 - Math.pow(t, 1.5) * 0.88;
                x *= taper;
                z *= taper;
                y += Math.pow(t, 2.0) * 0.65;
            }

            pos.setXYZ(i, x, y, z);
        }
        geom.computeVertexNormals();
        return geom;
    }, []);

    useFrame((state) => {
        if (matRef.current) {
            matRef.current.uTime = state.clock.elapsedTime;
            matRef.current.uSeverity = THREE.MathUtils.lerp(matRef.current.uSeverity, severity, 0.1);
            matRef.current.uEnergy = THREE.MathUtils.lerp(matRef.current.uEnergy, energy, 0.1);
        }

        if (mesh.current) {
            mesh.current.position.y = Math.sin(state.clock.elapsedTime) * 0.08;
            mesh.current.rotation.y = Math.sin(state.clock.elapsedTime * 0.5) * 0.2;
        }
    });

    return (
        <group>
            <mesh ref={mesh} geometry={dropletGeometry}>
                {/* @ts-ignore */}
                <dropletMaterial ref={matRef} transparent depthWrite={false} />
            </mesh>
            <RippleFloor energy={energy} />
            <Face mood={mood} energy={energy} />
        </group>
    );
};

// 7. Core Interactive Assistant Component
export const AiDroplet = ({ masterLevel, isPlaying }: { masterLevel: number, isPlaying: boolean }) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const [severity, setSeverity] = useState(0);
    const [mood, setMood] = useState('idle');
    const [insight, setInsight] = useState<any | null>(null);

    // Interactive Panels State (ported from Kropelka.tsx)
    const [showPermissions, setShowPermissions] = useState(false);
    const [chatInput, setChatInput] = useState('');
    const [isChatting, setIsChatting] = useState(false);
    const [vibeCheckData, setVibeCheckData] = useState<{ rms: number; balance: number } | null>(null);

    useEffect(() => {
        aiAssistant.onInsight((newInsight) => {
            setInsight(newInsight);
            setSeverity(newInsight.severity);

            if (newInsight.severity > 0.8) setMood('critical');
            else if (newInsight.severity > 0.4) setMood('warn');
            else setMood('idle');

            if (newInsight.targetElement) {
                flyToElement(newInsight.targetElement);
            }

            setTimeout(() => setInsight(null), newInsight.choices ? 12000 : 8000);
        });

        const interval = setInterval(() => {
            const rand = Math.random();
            if (rand > 0.85) aiAssistant.provideLlmInsight();
            else if (rand > 0.70) aiAssistant.provideDeepKnowledge('Mastering');
            else if (rand > 0.50) aiAssistant.providePluginTip();
            else if (rand > 0.30) aiAssistant.analyzeMidSide();
            else if (rand > 0.15) aiAssistant.suggestSoundDesign();
            else aiAssistant.suggestArrangement(5);
        }, 15000);

        return () => clearInterval(interval);
    }, []);

    // Eco-Hygiene / Zosia Activity Pings
    useEffect(() => {
        if (isPlaying) {
            invoke('trigger_zosia_activity').catch(e => console.error("Zosia Activity Ping Failed", e));
        }
    }, [isPlaying]);

    // Zosia Audit Loop (Every 60s)
    useEffect(() => {
        const auditLoop = setInterval(() => {
            invoke<string>('trigger_zosia_audit').then(res => {
                if (res && res.includes("actions queued")) {
                    console.log("[ZosiaMind] Audit Complete: ", res);
                }
            }).catch(e => console.error("Zosia Audit Ping Failed", e));
        }, 60000);
        
        return () => clearInterval(auditLoop);
    }, []);

    const flyToElement = (elementId: string) => {
        const el = document.getElementById(elementId);
        if (el && containerRef.current) {
            const rect = el.getBoundingClientRect();
            const targetX = rect.left + rect.width / 2 - 150;
            const targetY = rect.top + rect.height / 2 - 150;

            gsap.to(containerRef.current, {
                left: targetX,
                top: targetY,
                duration: 1.2,
                ease: "power3.inOut"
            });
        }
    };

    useEffect(() => {
        if (masterLevel > 0.98) {
            setSeverity(1.0);
            setMood('critical');
            aiAssistant.checkMastering(masterLevel);
        } else if (masterLevel > 0.85) {
            setSeverity(0.5);
            setMood('warn');
        } else if (severity < 0.2) {
            setSeverity(0);
            setMood(isPlaying ? 'flow' : 'idle');
        }
    }, [masterLevel, severity, isPlaying]);

    // Handlers for click, right click & actions
    const handleKropelkaClick = async () => {
        setMood('vibe_check');
        setInsight({ category: 'Vibe', message: "Analyzing project soul... 📶", severity: 0.5 });

        await invoke('trigger_vibe_check');
        const keyResult = await invoke<[string, string] | null>('detect_project_key');

        setTimeout(() => {
            setVibeCheckData({ rms: masterLevel, balance: 0.6 });
            if (keyResult) {
                setInsight({ 
                    category: 'Theory', 
                    message: `Vibe Check Complete. Detected: ${keyResult[0]} (${keyResult[1]})`, 
                    severity: 0.5 
                });
            } else {
                setInsight({ 
                    category: 'Vibe', 
                    message: "Vibe Check Complete. Solid dynamics!", 
                    severity: 0.5 
                });
            }

            setTimeout(() => {
                setVibeCheckData(null);
                setInsight(null);
                setMood(isPlaying ? 'flow' : 'idle');
            }, 5000);
        }, 1200);
    };

    const handleContextMenu = (e: React.MouseEvent) => {
        e.preventDefault();
        setShowPermissions(true);
    };

    const handleChatSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!chatInput.trim()) return;

        try {
            setInsight({ category: 'AI', message: "myślę...", severity: 0.5 });
            const command = 'get_kropelka_suggestion';
            const contextPayload = JSON.stringify({
                projectState: chatInput,
                uiData: {}
            });
            const res = await invoke<any>(command, { context: contextPayload });
            
            if (res) {
                setInsight({
                    category: res.category,
                    message: res.text,
                    action: res.action_type,
                    choices: res.choices,
                    severity: 0.8
                });
                const stateStr = res.state.toLowerCase();
                setMood(stateStr === 'producermode' ? 'creative' : stateStr);
                
                if (res.action_type === 'GenerateDrumClip') {
                    await invoke('apply_kropelka_fix', { action_type: res.action_type, action_data: null });
                }
            } else {
                setInsight({ category: 'Vibe', message: "Ciekawy pomył!", severity: 0.5 });
            }
        } catch (err) {
            console.error("Chat error", err);
        }
        setChatInput('');
        setIsChatting(false);
    };

    const handleAction = async (e: React.MouseEvent, action: string) => {
        e.stopPropagation();
        try {
            const res = await invoke<string>('apply_kropelka_fix', {
                action_type: action,
                action_data: null
            });

            setInsight((prev: any) => prev ? { ...prev, message: res } : null);
            setTimeout(() => setInsight(null), 3500);
        } catch (err) {
            console.error("Kropelka Fix Failed:", err);
            setInsight((prev: any) => prev ? { ...prev, message: "Fix failed. I'll try harder next time! 🤕" } : null);
        }
    };

    return (
        <div 
            className={`ai-droplet-canvas-container mode-${mood}`} 
            ref={containerRef} 
            id="vibe-ai-assistant"
            onClick={handleKropelkaClick}
            onContextMenu={handleContextMenu}
        >
            {/* 3D Render Canvas */}
            <Canvas gl={{ alpha: true }} dpr={[1, 2]}>
                <PerspectiveCamera makeDefault position={[0, 0, 5]} />
                <ambientLight intensity={0.5} />
                <pointLight position={[10, 10, 10]} intensity={1.5} />

                <Suspense fallback={null}>
                    <DropletEntity severity={severity} energy={masterLevel} mood={mood} />
                    <Environment preset="city" />
                </Suspense>
            </Canvas>

            {/* Permissions Overlay Modal */}
            <KropelkaPermissions isOpen={showPermissions} onClose={() => setShowPermissions(false)} />

            {/* AI suggestion popup card */}
            {insight && (
                <div className={`ai-insight-bubble ${mood}`}>
                    <div className="insight-header">
                        <span className="card-icon">
                            {insight.category === 'Theory' && '🎵'}
                            {insight.category === 'Mixing' && '🎚️'}
                            {insight.category === 'Mastering' && '🎛️'}
                            {insight.category === 'Technical' && '🚨'}
                            {insight.category === 'Vibe' && '✨'}
                            {insight.category === 'AI' && '🤖'}
                            {insight.category === 'Sound Design' && '🎹'}
                        </span>
                        <span className="card-title">{insight.category}</span>
                    </div>
                    <div className="insight-body">
                        {insight.message}
                        
                        {insight.choices ? (
                            <div className="card-actions" style={{ display: 'flex', gap: '8px', marginTop: '8px', flexWrap: 'wrap' }}>
                                {insight.choices.map((choice: string, i: number) => (
                                    <button
                                        key={i}
                                        className="fix-btn"
                                        style={choice.match(/Nah|No|Ignore/) ? { background: '#444' } : {}}
                                        onClick={(e: React.MouseEvent) => {
                                            e.stopPropagation();
                                            if (choice.match(/Nah|No|Ignore/)) {
                                                if (insight.action) {
                                                    invoke('reject_kropelka_suggestion', { action_type: insight.action });
                                                }
                                                setInsight(null);
                                            } else if (i === 0 && insight.action) {
                                                handleAction(e, insight.action);
                                            }
                                        }}
                                    >
                                        {choice}
                                    </button>
                                ))}
                            </div>
                        ) : (
                            insight.action && (
                                <button 
                                    className="fix-btn" 
                                    onClick={(e: React.MouseEvent) => {
                                        e.stopPropagation();
                                        handleAction(e, insight.action!);
                                    }}
                                >
                                    AKCEPTUJ SUGESTIE
                                </button>
                            )
                        )}
                    </div>
                </div>
            )}

            {/* Chat Overlay Toggle Button */}
            <div 
                className="chat-toggle-btn" 
                onClick={(e) => { e.stopPropagation(); setIsChatting(!isChatting); }}
                title="Porozmawiaj z Kropelką"
            >
                {isChatting ? 'Zamknij' : 'Rozmawiaj (Zosia Samosia)'}
            </div>

            {/* Chat Input Field form */}
            {isChatting && (
                <form 
                    onSubmit={handleChatSubmit} 
                    onClick={(e) => e.stopPropagation()}
                    className="chat-overlay-form"
                >
                    <input 
                        type="text" 
                        value={chatInput} 
                        onChange={(e) => setChatInput(e.target.value)} 
                        placeholder="Napisz do Kropelki..."
                        autoFocus
                    />
                    <button type="submit">&rarr;</button>
                </form>
            )}

            {/* Holographic Stats Display */}
            {vibeCheckData && (
                <div className="kropelka-stats-holo">
                    <div className="stat-row">
                        <span>RMS</span>
                        <div className="bar"><div style={{ width: `${vibeCheckData.rms * 100}%` }}></div></div>
                    </div>
                    <div className="stat-row">
                        <span>BAL</span>
                        <div className="bar"><div style={{ width: `${vibeCheckData.balance * 100}%` }}></div></div>
                    </div>
                </div>
            )}
        </div>
    );
};
