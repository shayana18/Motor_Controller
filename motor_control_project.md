# Motor Controller in Embedded Rust — Project Plan

Oct 6, 2026 · revised Oct 13, 2026 · @Shayan

> **Hard dates:** boards ordered by **Friday, November 13**. Board brought up, validated and running the motor under stable closed-loop control by **Thursday, December 10**, the last session before finals; Dec 22 is the absolute latest. From late December into January I'm travelling with the board and **no lab equipment**, so anything that needs a scope, multimeter or bench supply has to be finished before then.

## Overview

A custom three-phase motor controller board, designed by me, driving a direct-drive motor on a small bench test rig, with firmware in embedded Rust. The rig is chosen to exercise every part of FOC with almost no mechanical work; the controller can be attached to a deadbolt or another load later. The project has two equal halves: **designing the board** (electrical fundamentals, component selection, schematic, layout, bring-up) and **writing its firmware** in embedded Rust. The ML and AI safety track is a separate project with its own plan.

Learning goals, in order:

1. **Hardware and PCB design**: electrical fundamentals, choosing the motor, MCU and gate driver, schematic and layout, bring-up and validation of my own board.
2. **Embedded Rust**: Rustlings for the language, then bare-metal firmware with my own peripheral drivers (no Embassy, no HAL), learned by building it, not as separate study.
3. **Motor control that transfers to work**: three-phase PM motors and field-oriented control (FOC), the same techniques used in robot actuators and EV traction drives.

## Schedule

- **Motor Controller Project** sessions: Tuesday, Thursday and Friday, 8:30–10:30. That is 28 sessions (about 56 hours) from October 8 to December 10.
- **Optional Sunday** block, 10:00–1:00, for catch-up. During the PCB phase (M2), the Sundays on Oct 25, Nov 1 and Nov 8 are planned PCB work, and Nov 15 is the last buffer for the board order.
- **Finals, December 11–22:** no planned sessions. The Sundays on Dec 13 and 20 are emergency-only, for a bring-up measurement that still needs the bench before travel.
- **Travel, late December to January:** firmware refinements only, using the board's own sensors. No lab equipment.
- During midterms, drop to two sessions that week and let the Sunday block absorb the difference.
- End every session by writing the next concrete step, so the next session starts fast.
- Use AI as a tutor, not the author. The hologram lamp is where speed matters; this is where the learning happens.

### Where the time goes (before finals)

| Milestone | Dates | Sessions | Hours | Was |
| --- | --- | --- | --- | --- |
| M1 Foundations: Rustlings, motor control, project fundamentals | Oct 8–22 | 7 | ~14 | 11 sessions, ~22 h, about half embedded Rust |
| M2 PCB design: electrical fundamentals, chip selection, layout | Oct 23–Nov 13 | 10 + 3 Sundays | ~29 | 6 sessions, ~12 h |
| M3 Firmware on the dev kit, while boards ship | Nov 17–27 | 6 | ~12 | 6 sessions, ~12 h |
| M4 Bring-up, validation and motor control on the board | Dec 1–10 | 5 | ~10 | 5 sessions, ~10 h |
| M5 Refinements while travelling, no lab equipment | late Dec–Jan | — | ~12+ | winter break port |

## Scope

- This timeline covers the motor controller board and its firmware only, on a bench test rig: a direct-drive gimbal-style motor with a built-in magnetic encoder and a 3D-printed inertia disc that doubles as a knob. A second motor on the dev kit can be coupled shaft-to-shaft as a programmable load (a mini dynamometer).
- **Motor:** a three-phase permanent-magnet motor (sold as "BLDC"), driven first with six-step commutation and then with FOC. See "Motor type" below for why.
- **Board:** STM32G4, a three-phase inverter (gate driver plus discrete MOSFETs), low-side current sensing, a position sensor input, hardware overcurrent trip, NTC, USB-C, and SWD. Every part is chosen in M2, with a written reason.
- **Firmware:** the FOC current loop runs in a PWM-synced interrupt; velocity and position loops run in a lower-priority timer interrupt; a `no_std` control crate is tested on the Mac. Parameter identification, thermal derating and persistent config come after bring-up.
- **Main test:** the motor holds its position against up to 0.2 N·m of twist (0.35 N·m in bursts) on the disc by hand, then returns cleanly when you let go.
- **Protection test:** the hardware overcurrent trip is exercised for real (see "Testing the overcurrent protection").
- **Demos that prove the controller:** torque mode (the disc resists your hand with a set torque), virtual spring and detents (haptic knob), velocity hold under a disturbance, a position step with a clean response, and, with the dyno, speed holding against a commanded load torque and regenerating energy back to the supply.
- Commands come from a button on the board or from the Mac.
- Future expansions: the Linux side (Raspberry Pi with your own Buildroot image), face recognition, learned jam detection, a rev B board at higher voltage.

## Motor type: PMSM vs BLDC

**Recommendation: buy a three-phase PM motor sold as "BLDC" with sinusoidal-ish back-EMF (most gimbal and robot-joint motors), and drive it with FOC. That gives you the work-relevant PMSM skills on cheap, easy-to-source hardware.**

- **They're the same machine.** Both have permanent magnets on the rotor and three phase windings on the stator. The labels describe the back-EMF shape and how you drive it. A "BLDC" has trapezoidal back-EMF and is commutated in six steps from hall sensors. A "PMSM" has sinusoidal back-EMF and is driven with sinusoidal currents using FOC.
- **The board is identical.** Both need three half-bridges, a gate driver, phase current sensing and a position sensor. Choosing between them doesn't change the PCB. It changes the firmware and the motor you buy, so the board can be designed for both.
- **What transfers to work is FOC.** Robot joint actuators and EV traction motors are PMSMs (or interior-PM variants) running FOC: Clarke/Park transforms, dq current loops, SVPWM, field weakening, encoder or sensorless position. Six-step BLDC control is a good first milestone, but it isn't where the industry is.
- **Plan:** get the motor spinning with six-step first (quick win, and it validates the inverter and sensors), then move to FOC on the same hardware.
- **Rig:** direct drive, no gearbox. Gearing hides the motor's torque ripple and cogging behind friction, which is exactly what FOC tuning needs to see; direct drive also makes torque control and haptics feel crisp. A deadbolt or robot joint can be added later with a reduction.

## Motor specs

**Decided (Oct 14): DFRobot FIT1036** (4015, 61 KV, built-in AS5600 encoder). The specs below are what it was chosen against; redo the checks with its measured resistance when it arrives.

You pick the motor; these are the specs it should meet. They come from four requirements: **hold 0.2 N·m indefinitely** against your hand (about 6.7 N at the rim of the 30 mm-radius disc), **reach 0.35 N·m in short bursts** for snappy returns, **come with a built-in position encoder**, and **cost about $30–60**. The motor also has to be able to draw more current than the hardware overcurrent trip, so the protection can be tested at its real threshold. Run at 24 V.

| Spec | Target | Where it comes from |
| --- | --- | --- |
| Built-in encoder | Absolute magnetic, ≥ 12-bit; SPI preferred, I²C acceptable | Position readings for FOC and the position loop. SPI reads faster, which matters at FOC loop rates |
| Kv | about 50–75 rpm/V (Kt ≈ 0.13–0.19 N·m/A) | Peak 0.35 N·m within the 2.2 A max operating current (check 1) |
| Motor constant Km | ≥ 0.07 N·m/√W | Holding 0.2 N·m at ≤ 8 W of copper loss (check 2) |
| Rated continuous current | ≥ the hold current (~1.3 A) | The datasheet's own thermal limit |
| Phase-to-phase resistance | ≤ about 7 Ω | Stall current at 24 V must exceed the 3.2 A trip (check 3) |
| Rated voltage | 24 V | Matches the bus |
| Shaft | Hollow, with mounting holes | Mounting the disc and routing the encoder cable |

Cheap gimbal-style motors sold with an encoder (about 40–50 mm across) usually land in these ranges. Check every number on the datasheet of the motor you pick, and watch for listings that don't say whether resistance is phase-to-phase or per phase.

### Checks (redo with your motor's datasheet values)

The example uses a motor in the middle of the ranges: Kv ≈ 60 (Kt ≈ 0.16 N·m/A) and 6 Ω phase-to-phase (R_phase = 3 Ω), so Km ≈ 0.16 / √(1.5 × 3) ≈ 0.075 N·m/√W.

1. **Currents:** hold = 0.2 / 0.16 ≈ **1.3 A**; peak = 0.35 / 0.16 ≈ **2.2 A** (this is the max operating current).
2. **Heat at the hold:** P = 1.5 · I² · R_phase = 1.5 × 1.3² × 3 ≈ **7.6 W**, or equivalently (T / Km)². The 0.35 N·m bursts are about 22 W, so seconds only.
3. **Can it trip the protection?** Stall current at 24 V = (24 / √3) / R_phase = 13.9 / 3 ≈ **4.6 A**, above the 3.2 A trip. ✅ A 15 Ω gimbal motor (about 1.8 A at stall) couldn't trip it.
4. **Temperature:** ΔT = P · R_th. Use the datasheet thermal resistance if it's given; otherwise trust the rated current. Resistance rises about 0.39% per °C, so recheck the loss hot.
5. **Current ripple:** high-resistance motors have millihenries of inductance. For example, at 2.6 mH per phase, ΔI ≈ 24 / (4 × 2.6 mH × 30 kHz) ≈ 0.08 A, which is negligible.
6. **Speed:** no-load ≈ Kv × 24 ≈ 1400 rpm, plenty.

### Current limits this sets for the board

Each limit = the one below it + the worst normal excursion between them.

| Level | Value | Derivation |
| --- | --- | --- |
| Hold current | 1.3 A | 0.2 N·m ÷ Kt |
| Max operating current | 2.2 A | 0.35 N·m ÷ Kt |
| Firmware current limit | 2.5 A | Small cap above max operating current |
| Hardware overcurrent trip | 3.2 A | 2.5 A + ripple/2 (~0.05 A) + loop overshoot (~15%, 0.4 A) + sense error (~0.2 A) |
| Must stay below | Motor stall current, ~4.6 A | Otherwise the trip can't be tested at full threshold |
| Board rating | 10 A peak, 24 V | Deliberately oversized: survives faults with margin and leaves room for a bigger motor later |

At stall the supply draws only the losses (under 10 W, well under 1 A at 24 V).

### Testing the overcurrent protection

- **Trip path at low current (repeatable, safe):** set the STM32G4 comparator's internal DAC threshold to about 1 A, command a current step above it, and confirm on the scope that PWM shuts off within microseconds and the firmware latches and reports the fault.
- **Full threshold (on the bench in M4):** clamp the disc, use a test build that lifts the 2.5 A firmware limit, and step the command past 3.2 A. The motor's ~4.6 A stall current means the trip, not the winding, is what stops it. Check it fires near 3.2 A and faster than the firmware path.
- **While travelling:** only the low-threshold version and the firmware fault reporting, since there's no scope.

## System architecture

&#91;embedded content: system architecture · Mac host, custom board, bench rig\]

The STM32 owns the motor loops and safety logic. The Mac sends commands, logs telemetry for step responses and validation, and flashes the firmware.

## Milestones

### M1 — Foundations: Rustlings, motor control, project fundamentals (Oct 8–22, 7 sessions, ~14 h)

Three threads run side by side. Motor control fundamentals are largely done already, so the motor-control slots now cover the three-phase gap and the project's own requirements.

| Week of | Tuesday | Thursday | Friday |
| --- | --- | --- | --- |
| Oct 5 | — | ✅ Setup (rustup, probe-rs, KiCad, LTspice), start Rustlings | ✅ Motor model: back-EMF, torque constant, winding resistance, stall current |
| Oct 12 | Rustlings: ownership, structs, enums, error handling | **Project fundamentals:** set the board's power envelope (default: up to 24 V bus, about 10 A peak phase current, so it can later drive a small robot-joint motor), and pick a motor that meets the "Motor specs" section above | Rustlings: traits, generics |
| Oct 19 | **Motor type:** three-phase gap check (commutation, Clarke/Park, SVPWM overview); choose the motor and encoder against the requirements; order motor, encoder and dev kit | Rustlings: iterators, closures, smart pointers; finish with a system block diagram v0 | *M2 starts* |

- **Done when:** Rustlings complete through error handling, traits, generics and iterators; a one-page requirements sheet with the board's voltage and current envelope; motor and encoder chosen with a written reason; motor and dev kit ordered by **October 20**.
- **Moved out of M1:** the Embedded Rust Book sessions. They're learned just in time in M3, while writing the drivers.
- **Resources:** [Rustlings](https://github.com/rust-lang/rustlings), [The Rust Programming Language](https://doc.rust-lang.org/book/), Brian Douglas's control lectures (YouTube), TI and ST application notes introducing BLDC/PMSM and FOC.

### M2 — PCB design (Oct 23–Nov 13, 10 sessions plus 3 Sundays, ~29 h)

Four phases, each ending with something written down. Every phase uses LTspice or a quick calculation to check the numbers. The order date is fixed by the December 10 bring-up target: an assembled 4-layer board takes roughly one to two weeks from order to door with express shipping, so a Nov 13 order lands around Nov 23–30, in time for bring-up on Dec 1.

**Phase A — Electrical fundamentals (Oct 23–29, 3 sessions + Sun Oct 25)**

- Ohm's law and KVL/KCL refresher through real circuits: voltage dividers, RC filters, decoupling.
- MOSFETs as switches: Rds(on), gate charge, switching and conduction losses, body diode, safe operating area.
- Half-bridges: dead time, shoot-through, bootstrap supplies, ringing and the power-loop inductance.
- Current sensing: shunts, op-amp and differential amplifiers, anti-aliasing filters, sampling in sync with PWM.
- Power supplies: buck converter operation, LDO vs buck, reverse-polarity protection.
- Thermal: power dissipation to temperature rise, copper area as a heatsink.
- **Done when:** an LTspice sim of one half-bridge with dead time and of the current-sense front end, plus a loss and temperature estimate for the MOSFETs at the M1 peak current.

**Phase B — Choosing the parts (Oct 30–Nov 3, 2 sessions + Sun Nov 1)**

- **MCU:** compare the STM32G431 and STM32G474 (and an RP2350 as the counterexample). The things that matter for motor control: an advanced timer with complementary outputs and hardware dead time, a break input, fast ADCs triggered by the timer, on-chip op-amps and comparators, CORDIC for FOC math, and a clear reference manual and Rust PAC (register-access crate), since you're writing the drivers yourself. **Decided (Oct 14): STM32G431RBT6** (LQFP64, 128 KB flash), the same chip as the Nucleo-G431RB, so dev-kit firmware ports directly.
- **Gate driver and power stage:** compare an integrated-FET driver (e.g. TI DRV8316: simpler layout, fewer decisions) with a gate driver plus discrete MOSFETs (e.g. TI DRV8323, which has built-in current-sense amplifiers). Default: **gate driver plus discrete MOSFETs**, because sizing FETs, gate resistors and bootstrap caps and laying out the power loop is the electrical learning you want, and it's how EV and robot inverters are built. Because the board has to work first time for the Dec 10 target, make this call firmly at the end of Phase B (Nov 3): if the discrete design isn't clearly understood by then, switch to the integrated part. A working board you can travel with beats a more educational one that needs a rev B.
- **The rest:** MOSFETs (voltage margin, Rds(on), gate charge, package), shunt values and power rating, the position sensor interface (hall, quadrature or SPI magnetic encoder), the buck regulator, USB-C protection.
- For each part: read the datasheet's reference design and layout guidance, check it's in stock and assemblable at the board house.
- **Done when:** block diagram v1, BOM v0 with a one-line reason per major part, and a power budget per rail.

**Phase C — PCB design fundamentals and schematic (Nov 5–8, 2 sessions + Sun Nov 8)**

- Stackups and why a 4-layer board with a solid ground plane is worth it here; return paths; separating the high-current power loop from analog sensing; Kelvin connections on the shunts; trace width for current; creepage and clearance at your voltage.
- KiCad schematic: hierarchical sheets for power, MCU, inverter, sensing and connectors; ERC; footprint assignment against the board house's parts library.
- **Schematic checklist:** 24 V-max input with reverse-polarity protection, buck to 3.3 V, STM32G431, USB-C (CC resistors, ESD) feeding a USB-to-UART bridge chip so the Mac talks to the MCU's UART and no USB driver is needed, three-phase inverter sized from M1's current, phase current sensing with filters, current-sense output to a G4 comparator for the hardware trip, NTC beside the FETs, position sensor connector, button, LED, buzzer, SWD header, a spare GPIO on a test pad for loop timing, and test points on every rail and key signal.
- **Done when:** the schematic passes ERC and a self-review against the reference designs.

**Phase D — Layout, review, order (Nov 10–13, 3 sessions)**

- Placement first (power stage, then sensing, then MCU), the power loop as tight as possible, then routing, ground pour, stitching vias, silkscreen and test points.
- DRC, a written checklist review, the board house's DFM check, and a 3D view check of connectors and the motor wiring.
- **Design for bring-up:** test points and a series 0 Ω link or jumper on each rail so they can be powered one at a time, an LED on each rail, gate-drive signals on probe-able pads, and a phase-current or fault readout over USB so the board can report its own health later without a meter.
- **Done when:** boards are ordered with assembly and express shipping by **Friday, November 13** (Sunday, Nov 15 at the very latest), together with a written bring-up plan listing the expected value for every rail and signal. Order at least 3 assembled boards, so a damaged board during bring-up isn't the end of the travel plan.
- **Resources:** Phil's Lab KiCad and component-selection videos, Rick Hartley's talks on grounding and return paths, the gate driver and MCU datasheets' layout guidelines, TI's "MOSFET power losses" and bootstrap application notes, *The Art of Electronics* (reference, not cover to cover).

### M3 — Firmware on the dev kit (Nov 17–27, 6 sessions, ~12 h, while boards ship)

The goal is firmware that's ready to port, so the December sessions go into the board, not into writing code from scratch.

- **Embedded Rust, just in time:** the first session is the Embedded Rust Book essentials (`no_std`, `cortex-m-rt`, the PAC, interrupts and NVIC priorities) and a register-level blinky with `defmt` logging over probe-rs on the Nucleo.
- **Your own drivers, only what the motor needs, in this order:** clocks and GPIO; the advanced timer (complementary PWM, dead time, break input, center-aligned trigger); ADC injected conversions triggered by the timer; the encoder bus (I²C for the AS5600, or SPI if you switch encoders); the comparator and DAC for the overcurrent trip; UART to the Mac through the ST-Link's virtual COM port. Skip the MCU's native USB: a USB driver from scratch is a project on its own. On the custom board, a USB-to-UART bridge chip gives the same connection to the Mac through a UART driver you write anyway.
- **Schedule risk:** writing drivers instead of using a HAL costs time in M3. The timer and ADC drivers are the critical path to spinning the motor; write those first and keep the rest minimal until the Dec 10 target is met.
- **Structure:** a Cargo workspace with a `no_std` `control` crate (Clarke/Park, SVPWM, PI with anti-windup, motion profile, state machine) with no hardware dependencies, unit-tested on the Mac with `cargo test`; a `protocol` crate (`serde` + `postcard`); the `firmware` crate with your drivers and a board-support module per board, so the dev kit and the custom board share everything else.
- **Order of work:** six-step commutation from the position sensor, then current sensing synced to PWM, then a closed-loop speed or position loop, then the FOC current loop in a high-priority interrupt triggered by the PWM timer if time allows.
- **Done when:** the control crate's tests pass on the Mac; the motor runs under stable closed-loop control on the dev kit; the current loop's rate and jitter are measured on the scope with a GPIO toggle. FOC on hardware can finish while travelling, since it needs only the board.

### M4 — Bring-up, validation and motor control on the board (Dec 1–10, 5 sessions, ~10 h)

Everything that needs the bench happens here. The finals Sundays (Dec 13, 20) are emergency-only.

| Session | Work |
| --- | --- |
| Tue Dec 1 | Power up at a current limit with the motor disconnected; verify each rail and its ripple; SWD, and the USB-to-UART bridge showing up on the Mac |
| Thu Dec 3 | Gate-drive waveforms and dead time on the scope with no motor; current-sense offset and calibration against a known load; NTC reading |
| Fri Dec 4 | Port the M3 firmware via the board-support module; spin the motor open-loop at low current |
| Tue Dec 8 | Closed-loop control on the board; hardware overcurrent trip verified on the scope as faster than the firmware path; ripple and current-sense noise with the motor running; MOSFET temperature at peak current |
| Thu Dec 10 | Soak test: 30 minutes of position steps and velocity holds with no resets or faults; test on the power supply you'll travel with; travel checklist |

- **Done when (Dec 10, Dec 22 at the latest):** every row of the bring-up plan has a measured value beside the expected one; the motor is under stable closed-loop position and velocity control and holds them when you push on the disc; the hardware trip is verified; a written list of rev B changes exists.
- **If the boards arrive late or a board is dead:** keep the bench sessions for whatever board works, and pack the dev kit as the travel fallback so the firmware work in M5 still happens.
- **Travel checklist:** the board on the supply you'll take (12–24 V adapter or a USB-C PD trigger with an inline fuse), the motor with its disc (and the dyno motor if it fits), the debug probe (the Nucleo's ST-Link), USB cables, a spare assembled board, the dev kit as backup, and firmware that comes up in a safe state if anything goes wrong. The Analog Discovery 3 fits in a laptop bag if you decide to take one instrument after all.

### M5 — Refinements while travelling (late Dec–Jan, ~12 h, no lab equipment)

Everything here uses only the board's own sensors (phase current, bus voltage, NTC, position sensor) and logging over USB serial, so it needs a laptop and the board, nothing else.

- **FOC on the board:** dq current loops, then velocity and position, then motion profiles and the haptic modes (spring, detents, damping). Optional: field weakening to run above base speed, and sensorless operation as a check against the encoder.
- **Parameter identification using the board itself:** phase resistance from a DC current at locked rotor, inductance from the current rise after a voltage step, back-EMF constant from spinning at a known speed. Compute current-loop gains from R and L instead of hand-tuning.
- **Firmware fault handling:** latch and report hardware trips over USB serial; stall detection with back-off and retry.
- **Thermal derating** from the NTC, and **persistent config** (calibration and gains in flash with a version and CRC, e.g. `sequential-storage`).
- **Host tool:** commands, live gain changes, telemetry to CSV, so the plots replace the scope.
- **Done when:** FOC gives smooth torque at low speed where six-step visibly cogs; the haptic knob modes work; the current-loop step response is logged and matches the bandwidth predicted from R and L; identified parameters are tabled against the motor's datasheet; config survives a power cycle.

### Stretch — Bootloader and updates (January, optional, ~8 h)

- Your own A/B bootloader with firmware updates over UART from the host tool, an integrity check, and rollback if new firmware doesn't confirm itself.
- GitHub Actions CI that builds the firmware and runs the control crate's tests.

## If time slips

Use the Sunday block first. The November 13 order and the December 10 bring-up are the dates that matter; everything else can bend. Cut in this order:

1. Discrete MOSFETs: switch to an integrated-FET driver at the Nov 3 checkpoint to shorten layout and lower bring-up risk.
2. FOC before travel: six-step closed-loop control is enough to call the board working on Dec 10.
3. The stretch milestone.
4. Thermal derating in M5.
5. Parameter identification: use datasheet values for the gains.

Keep the host tool: while travelling, its telemetry plots are the only stand-in for a scope.

## Parts and tools

Order the motor and dev kit by October 20 so M3 isn't waiting on them. Exact part numbers are settled in M1 (motor) and M2 (board).

| Item | Needed by | Notes |
| --- | --- | --- |
| Nucleo-G431RB | M3 | Its onboard ST-Link can also program the custom board over SWD |
| Three-phase driver expansion board | M3 | For the dev-kit prototype, e.g. ST X-NUCLEO-IHM07M1 on the Nucleo, or the all-in-one B-G431B-ESC1 |
| Motor: **DFRobot FIT1036** (decided Oct 14), ×2 | M3 | 4015 gimbal motor, 61 KV, AS5600 encoder built in (I²C), no driver included; $29.90, in stock at Mouser. Measure phase-to-phase resistance on arrival (listing gives 4.8 Ω and 7.2 Ω) |
| Second motor for the dyno | M3 | Optional; the same model, run from the dev kit as the load |
| 3D-printed inertia disc, motor mount, shaft coupler | M3 | Disc: 60 mm diameter (30 mm radius), about 10 mm thick, knurled rim for grip, with a ring of holes at about 25 mm radius for bolting on nuts or washers to change the inertia. In PLA it weighs about 35 g, so J ≈ 1.6 × 10⁻⁵ kg·m². This is the only mechanical work on the rig |
| 12–24 V supply | M3 | Rated above the motor's peak current |
| Analog Discovery 3 | M2 | Scope, logic analyzer and signal generator; works on macOS. Useful for Phase A experiments |
| Current-limited bench supply and multimeter | M4 | For safe first power-up |
| Custom PCBs with assembly | M4 | 4-layer, at least 3 assembled, ordered by Nov 13 with express shipping |
