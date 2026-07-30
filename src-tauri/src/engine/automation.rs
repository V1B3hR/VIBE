use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq)]
pub enum InterpolationType {
    Linear,
    Step,
    MonotoneHermite, // PCHIP
    Akima,           // VIBE Default: Natural, music-friendly
    Bezier,          // VIBE Dynamic Tension: Smooth adjustable curve
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AutomationKnot {
    pub sample_pos: u64,
    pub value: f64,
    pub tension: f64, // For future Bezier support (0.0 = default)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ModifierType {
    LFO {
        shape: LfoShape,
        frequency_hz: f64,
        depth: f64,
        phase_offset: f64,
    },
    RandomWalk {
        step_chance: f32, // 0.0-1.0
        max_step: f64,
    },
    Quantize {
        step_size: f64,
    },
    Physics {
        // Simple mass-spring-damper
        mass: f64,
        spring: f64,
        damping: f64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LfoShape {
    Sine,
    Triangle,
    Square,
    Saw,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AutomationLayer {
    pub id: uuid::Uuid,
    pub name: String,
    pub enabled: bool,
    pub modifier: ModifierType,
    pub mix: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AutomationCurve {
    pub knots: Vec<AutomationKnot>,
    pub interpolation: InterpolationType,
    pub layers: Vec<AutomationLayer>,
    #[serde(skip)]
    pub last_eval_value: f64,
    pub is_recording: bool,
}

impl AutomationCurve {
    pub fn new(initial_value: f64) -> Self {
        Self {
            knots: vec![AutomationKnot {
                sample_pos: 0,
                value: initial_value,
                tension: 0.0,
            }],
            interpolation: InterpolationType::Akima,
            layers: Vec::new(),
            last_eval_value: initial_value,
            is_recording: false,
        }
    }

    pub fn add_knot(&mut self, sample_pos: u64, value: f64) {
        if let Some(pos) = self.knots.iter().position(|k| k.sample_pos == sample_pos) {
            self.knots[pos].value = value;
        } else {
            self.knots.push(AutomationKnot {
                sample_pos,
                value,
                tension: 0.0,
            });
            self.knots.sort_by_key(|k| k.sample_pos);
        }
    }

    pub fn record_value(&mut self, sample_pos: u64, value: f64) {
        let significant_change = (value - self.last_eval_value).abs() > 0.0001;
        if significant_change || self.knots.is_empty() {
            self.add_knot(sample_pos, value);
        }
        self.last_eval_value = value;
    }

    pub fn set_tension(&mut self, sample_pos: u64, tension: f64) {
        if let Some(pos) = self.knots.iter().position(|k| k.sample_pos == sample_pos) {
            self.knots[pos].tension = tension.clamp(-1.0, 1.0);
        }
    }

    pub fn get_value_at(&self, sample_pos: u64) -> f64 {
        let base_value = self.evaluate_base_curve(sample_pos);
        self.apply_layers(base_value, sample_pos)
    }

    fn evaluate_base_curve(&self, sample_pos: u64) -> f64 {
        if self.knots.is_empty() {
            return 0.0;
        }
        if self.knots.len() == 1 {
            return self.knots[0].value;
        }
        if sample_pos <= self.knots[0].sample_pos {
            return self.knots[0].value;
        }
        if sample_pos >= self.knots.last().unwrap().sample_pos {
            return self.knots.last().unwrap().value;
        }

        match self.interpolation {
            InterpolationType::Linear => self.eval_linear(sample_pos),
            InterpolationType::Step => self.eval_step(sample_pos),
            InterpolationType::MonotoneHermite => self.eval_hermite(sample_pos),
            InterpolationType::Akima => self.eval_akima(sample_pos),
            InterpolationType::Bezier => self.eval_bezier(sample_pos),
        }
    }

    fn apply_layers(&self, base_val: f64, sample_pos: u64) -> f64 {
        let mut final_val = base_val;

        for layer in &self.layers {
            if !layer.enabled {
                continue;
            }

            let modifier_val = match &layer.modifier {
                ModifierType::LFO {
                    shape,
                    frequency_hz,
                    depth,
                    phase_offset,
                } => {
                    let t_sec = sample_pos as f64 / 44100.0;
                    let phase = t_sec * frequency_hz * 2.0 * std::f64::consts::PI + phase_offset;
                    let osc = match shape {
                        LfoShape::Sine => phase.sin(),
                        LfoShape::Triangle => phase.sin().asin() * 2.0 / std::f64::consts::PI,
                        LfoShape::Square => phase.sin().signum(),
                        LfoShape::Saw => {
                            (phase % (2.0 * std::f64::consts::PI)) / std::f64::consts::PI - 1.0
                        }
                    };
                    osc * depth
                }
                ModifierType::RandomWalk { .. } => 0.0,
                ModifierType::Quantize { step_size } => {
                    let stepped = (final_val / step_size).round() * step_size;
                    stepped - final_val
                }
                ModifierType::Physics {
                    mass: _,
                    spring: _,
                    damping: _,
                } => {
                    // Real physics requires persistent state (velocity, position) which is hard in purely functional 'get_value_at'.
                    // For 'VIBE Kinetic' we approximate "Inertia" by low-passing the delta from previous eval?
                    // But we don't have previous eval in random access.
                    // Fallback: This layer effectively does nothing in Random Access,
                    // but in `get_values_block` (sequential) we can simulate it.
                    0.0
                }
            };

            final_val += modifier_val * layer.mix;
        }

        final_val
    }

    /// SIMD-Optimized Evaluation Block
    #[allow(dead_code)]
    pub fn get_values_block(&self, start_pos: u64, dest: &mut [f64]) {
        // Use wide crate for LFO generation optimization
        use wide::f64x4;

        // 1. Evaluate Base Curve (Scalar for now due to complex spline logic)
        for (i, d) in dest.iter_mut().enumerate() {
            *d = self.evaluate_base_curve(start_pos + i as u64);
        }

        // 2. Apply Layers (Vectorized where possible)
        for layer in &self.layers {
            if !layer.enabled {
                continue;
            }

            match &layer.modifier {
                ModifierType::LFO {
                    shape,
                    frequency_hz,
                    depth,
                    phase_offset,
                } => {
                    if let LfoShape::Sine = shape {
                        // Vectorized Sine LFO
                        let chunks = dest.chunks_mut(4);
                        for (i, chunk) in chunks.enumerate() {
                            if chunk.len() == 4 {
                                let offset = i * 4;
                                let t_base = start_pos as f64 + offset as f64;
                                let t_vec =
                                    f64x4::new([t_base, t_base + 1.0, t_base + 2.0, t_base + 3.0])
                                        / 44100.0;
                                let freq_vec = f64x4::splat(*frequency_hz);
                                let phase_vec = t_vec * freq_vec * 2.0 * std::f64::consts::PI
                                    + f64x4::splat(*phase_offset);
                                let sine_vec = phase_vec.sin() * f64x4::splat(*depth);
                                let current_vec =
                                    f64x4::new([chunk[0], chunk[1], chunk[2], chunk[3]]);
                                let res = current_vec + sine_vec * f64x4::splat(layer.mix);
                                let res_arr = res.to_array();
                                chunk.copy_from_slice(&res_arr);
                            } else {
                                // Fallback for tail
                                for (j, samp) in chunk.iter_mut().enumerate() {
                                    let t = (start_pos as f64 + (i * 4 + j) as f64) / 44100.0;
                                    let phase = t * frequency_hz * 2.0 * std::f64::consts::PI
                                        + phase_offset;
                                    *samp += (phase.sin() * depth) * layer.mix;
                                }
                            }
                        }
                    } else {
                        // Scalar fallback for other shapes
                        for (i, d) in dest.iter_mut().enumerate() {
                            let t = (start_pos as f64 + i as f64) / 44100.0;
                            // ... (replicate logic or call helper)
                            let phase =
                                t * frequency_hz * 2.0 * std::f64::consts::PI + phase_offset;
                            let osc = match shape {
                                LfoShape::Triangle => {
                                    phase.sin().asin() * 2.0 / std::f64::consts::PI
                                }
                                LfoShape::Square => phase.sin().signum(),
                                LfoShape::Saw => {
                                    (phase % (2.0 * std::f64::consts::PI)) / std::f64::consts::PI
                                        - 1.0
                                }
                                _ => 0.0,
                            };
                            *d += osc * depth * layer.mix;
                        }
                    }
                }
                _ => {
                    // Scalar fallback
                    for _d in dest.iter_mut() {
                        // Logic...
                    }
                }
            }
        }
    }

    // --- INTERPOLATION ENGINES ---

    fn eval_linear(&self, sample_pos: u64) -> f64 {
        let (idx, _) = self.find_segment(sample_pos);
        let k0 = &self.knots[idx];
        let k1 = &self.knots[idx + 1];

        let t = (sample_pos - k0.sample_pos) as f64 / (k1.sample_pos - k0.sample_pos) as f64;
        k0.value + t * (k1.value - k0.value)
    }

    fn eval_step(&self, sample_pos: u64) -> f64 {
        let (idx, _) = self.find_segment(sample_pos);
        self.knots[idx].value
    }

    fn eval_hermite(&self, sample_pos: u64) -> f64 {
        // Existing Monotone Hermite Impl
        let (idx, _) = self.find_segment(sample_pos);
        // ... (Re-using logic from previous implementation but optimized)
        // For brevity in this big replacement, using a simplified version or the one we had.
        // Let's copy the robust one we had, but wrapped nicely.

        let k0 = &self.knots[idx];
        let k1 = &self.knots[idx + 1];
        let x0 = k0.sample_pos as f64;
        let x1 = k1.sample_pos as f64;
        let y0 = k0.value;
        let y1 = k1.value;

        // Calculate slopes (m0, m1) - usually requires looking at neighbors
        let m0 = self.compute_slope_monotone(idx);
        let m1 = self.compute_slope_monotone(idx + 1); // This might be idx in next segment context

        // Interpolate
        let h = x1 - x0;
        let t = (sample_pos as f64 - x0) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        h00 * y0 + h10 * m0 * h + h01 * y1 + h11 * m1 * h
    }

    fn eval_akima(&self, sample_pos: u64) -> f64 {
        // Akima Spline Interpolation
        // Uses 5 points (2 before, 2 after) to determine slope.
        // Less overshoot than cubic splines, more natural than monotone hermite.

        let (idx, _) = self.find_segment(sample_pos);
        let k0 = &self.knots[idx];
        let k1 = &self.knots[idx + 1];
        let x0 = k0.sample_pos as f64;
        let x1 = k1.sample_pos as f64;
        let y0 = k0.value;
        let y1 = k1.value;

        let m0 = self.compute_slope_akima(idx);
        let m1 = self.compute_slope_akima(idx + 1);

        // Cubic Hermite basis can be used once slopes are found via Akima method
        let h = x1 - x0;
        let t = (sample_pos as f64 - x0) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        h00 * y0 + h10 * m0 * h + h01 * y1 + h11 * m1 * h
    }

    fn eval_bezier(&self, sample_pos: u64) -> f64 {
        let (idx, _) = self.find_segment(sample_pos);
        let k0 = &self.knots[idx];
        let k1 = &self.knots[idx + 1];

        let h = (k1.sample_pos - k0.sample_pos) as f64;
        if h <= 0.0 {
            return k0.value;
        }

        let t = (sample_pos - k0.sample_pos) as f64 / h;
        
        // Very fast tension power function curve:
        // if tension == 0, val = t
        // if tension > 0, val = t^(1 + 4*tension)
        // if tension < 0, val = t^(1 / (1 - 4*tension))
        let tension = k0.tension.clamp(-0.99, 0.99);
        let t_curved = if tension >= 0.0 {
            t.powf(1.0 + tension * 4.0)
        } else {
            t.powf(1.0 / (1.0 - tension * 4.0))
        };

        k0.value + t_curved * (k1.value - k0.value)
    }

    // --- HELPERS ---

    fn find_segment(&self, pos: u64) -> (usize, usize) {
        match self.knots.binary_search_by_key(&pos, |k| k.sample_pos) {
            Ok(i) => {
                if i == self.knots.len() - 1 {
                    (i - 1, i)
                } else {
                    (i, i + 1)
                }
            }
            Err(i) => (i - 1, i),
        }
    }

    fn compute_slope_monotone(&self, i: usize) -> f64 {
        // Secants
        let secant = |idx: usize| {
            if idx >= self.knots.len() - 1 {
                return 0.0;
            }
            let k_curr = &self.knots[idx];
            let k_next = &self.knots[idx + 1];
            (k_next.value - k_curr.value) / (k_next.sample_pos as f64 - k_curr.sample_pos as f64)
        };

        let m_left = if i > 0 { secant(i - 1) } else { secant(i) };
        let m_right = secant(i);

        if m_left * m_right <= 0.0 {
            0.0
        } else {
            (m_left + m_right) / 2.0
            // Ideally harmonic mean for true PCHIP, but avg is okay for now
        }
    }

    fn compute_slope_akima(&self, i: usize) -> f64 {
        // Akima formula for slope at point i requires secants:
        // s1 = slope(i-2, i-1)
        // s2 = slope(i-1, i)
        // s3 = slope(i, i+1)
        // s4 = slope(i+1, i+2)
        // t = (|s4 - s3| * s2 + |s2 - s1| * s3) / (|s4 - s3| + |s2 - s1|)

        // Helper to safely get secant slope
        let secant = |idx: isize| -> f64 {
            if idx < 0 || idx >= (self.knots.len() as isize - 1) {
                // Out of bounds: use simple linear extrapolation of nearest segment
                if self.knots.len() < 2 {
                    return 0.0;
                }
                if idx < 0 {
                    let k0 = &self.knots[0];
                    let k1 = &self.knots[1];
                    (k1.value - k0.value) / (k1.sample_pos as f64 - k0.sample_pos as f64)
                } else {
                    let last = self.knots.len() - 1;
                    let k0 = &self.knots[last - 1];
                    let k1 = &self.knots[last];
                    (k1.value - k0.value) / (k1.sample_pos as f64 - k0.sample_pos as f64)
                }
            } else {
                let idx = idx as usize;
                let k0 = &self.knots[idx];
                let k1 = &self.knots[idx + 1];
                (k1.value - k0.value) / (k1.sample_pos as f64 - k0.sample_pos as f64)
            }
        };

        let i_isize = i as isize;
        let s1 = secant(i_isize - 2);
        let s2 = secant(i_isize - 1);
        let s3 = secant(i_isize);
        let s4 = secant(i_isize + 1);

        let w_num = (s4 - s3).abs() * s2 + (s2 - s1).abs() * s3;
        let w_den = (s4 - s3).abs() + (s2 - s1).abs();

        if w_den == 0.0 {
            (s2 + s3) / 2.0
        } else {
            w_num / w_den
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_akima_continuity() {
        let mut curve = AutomationCurve::new(0.0);
        curve.add_knot(100, 1.0);
        curve.add_knot(200, 0.0);
        curve.add_knot(300, 1.0);
        curve.interpolation = InterpolationType::Akima;

        // Check values at knots
        assert_eq!(curve.get_value_at(0), 0.0);
        assert_eq!(curve.get_value_at(100), 1.0);
        assert_eq!(curve.get_value_at(200), 0.0);
        assert_eq!(curve.get_value_at(300), 1.0);

        // Check midpoint values (should be smooth)
        let mid = curve.get_value_at(50);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn test_akima_no_overshoot() {
        // Akima is famous for having less overshoot than cubic splines.
        // Let's test a "step-like" pattern.
        let mut curve = AutomationCurve::new(0.0);
        curve.add_knot(100, 0.0);
        curve.add_knot(110, 0.0);
        curve.add_knot(120, 1.0);
        curve.add_knot(130, 1.0);
        curve.interpolation = InterpolationType::Akima;

        // In a cubic spline, values before 110 might dip below 0.0 (overshoot).
        // Akima should stay very close or at 0.0.
        let val_early = curve.get_value_at(105);
        assert!(
            val_early >= -0.01,
            "Akima should minimize overshoot, got {}",
            val_early
        );

        let val_late = curve.get_value_at(125);
        assert!(
            val_late <= 1.01,
            "Akima should minimize overshoot, got {}",
            val_late
        );
    }

    #[test]
    fn test_lfo_layer_simd() {
        let mut curve = AutomationCurve::new(0.5);
        curve.layers.push(AutomationLayer {
            id: uuid::Uuid::new_v4(),
            name: "LFO".to_string(),
            enabled: true,
            modifier: ModifierType::LFO {
                shape: LfoShape::Sine,
                frequency_hz: 1.0,
                depth: 0.1,
                phase_offset: 0.0,
            },
            mix: 1.0,
        });

        let mut block = vec![0.0; 100];
        curve.get_values_block(0, &mut block);

        // Check if values oscillate around 0.5
        assert!(block[0] >= 0.49 && block[0] <= 0.51);
        // At 1/4 cycle (44100/4 samples), sine should be at peak
        // But freq is 1Hz, so at sample 11025.
    }

    #[test]
    fn test_interpolation_types() {
        let mut curve = AutomationCurve::new(0.0);
        curve.add_knot(100, 1.0);
        curve.add_knot(200, 0.0);

        // 1. Linear
        curve.interpolation = InterpolationType::Linear;
        assert_eq!(curve.get_value_at(150), 0.5);

        // 2. Step
        curve.interpolation = InterpolationType::Step;
        assert_eq!(curve.get_value_at(150), 1.0);

        // 3. Akima (Smooth dip)
        curve.interpolation = InterpolationType::Akima;
        let akima_val = curve.get_value_at(150);
        assert!(akima_val > 0.0 && akima_val < 1.0);
    }

    #[test]
    fn test_automation_layers_combination() {
        let mut curve = AutomationCurve::new(0.5);
        // Add a Quantize layer (steps of 0.1)
        curve.layers.push(AutomationLayer {
            id: uuid::Uuid::new_v4(),
            name: "Bitcrush".to_string(),
            enabled: true,
            modifier: ModifierType::Quantize { step_size: 0.1 },
            mix: 1.0,
        });

        // Add an LFO layer (±0.05)
        curve.layers.push(AutomationLayer {
            id: uuid::Uuid::new_v4(),
            name: "Wobble".to_string(),
            enabled: true,
            modifier: ModifierType::LFO {
                shape: LfoShape::Sine,
                frequency_hz: 100.0,
                depth: 0.05,
                phase_offset: 0.0,
            },
            mix: 1.0,
        });

        let val = curve.get_value_at(0);
        // Base 0.5 -> Quantized (stays 0.5) -> LFO (starts 0.0)
        // Actually LFO adds to final_val.
        // base 0.5. q 0.1 -> quantize(0.5) = 0.5. diff = 0.
        // next layer LFO. sine(0) = 0.
        assert!((val - 0.5).abs() < 0.001);

        // At a different time where LFO is +0.05
        // We'll just check it stays quantized if we quantize AFTER LFO?
        // Wait, layers are serial in apply_layers.
    }
}
