# Firmware TODOs

Shared list for every thread. Add new items under the right section, tick items off with `[x]`, and keep the "Why" short. Pin and register names are for the STM32G431CBT6 (LQFP48), RM0440 Rev 9.

## Startup (DECIDED Nov 18)
- [ ] Own startup code from scratch, following the Embedonomicon ch. 1-5 (no cortex-m-rt): linker script, vector table, reset handler. Prove it in QEMU + GDB, then port to the G431 (memory map from RM0440, vector table size, FPU on via CPACR). About 1 week of evenings, then drivers.

## Clock
- [x] SYSCLK 170 MHz (HSI16 /4 x85 /2), set in .ioc Nov 14.

## TIM1 PWM (motor)
- [ ] Pick the PWM frequency (20 to 30 kHz, center-aligned) and set PSC/ARR.
- [x] Pins: CH1 PA8 / CH1N PB13 (phase A), CH2 PA9 / CH2N PB14 (phase B), CH3 PA10 / CH3N PB15 (phase C). CH2N moved PA12 -> PB14 in the .ioc and schematic (Nov 16); PA12 is now free.
- [ ] Dead time: the DRV8300 inserts about 200 ns itself, so TIM1 dead time can start at 0. Add more if scope shows overlap.
- [ ] Cap the duty at about 95% so the bootstrap caps refresh. Pre-charge them by turning on all low sides for about 20 us before starting PWM.
- [ ] Idle states: OIS1-3 and OIS1N-3N = 0 (all gates low when stopped or tripped).

## Hardware overcurrent trip (3.2 A, comparators to TIM1 break)
Why: a shoot-through rises about 1 A/ns. Software sampling once per PWM period is far too slow, and it stops at a breakpoint or crash. This path needs no CPU.

Mapping, as configured in cubemx/motor_controller.ioc:

| Phase | Op-amp + (sense node) | Comparator + (separate pin, same net) | Comparator - | Break bit |
|---|---|---|---|---|
| A | PA1 | COMP3 on PA0 | DAC3 CH1 | BKCMP3E |
| B | PA7 (shared pin, Nov 16) | COMP2 on PA7 (same pin) | DAC1 CH2 | BKCMP2E |
| C | PB0 | COMP1 on PB1 | DAC1 CH1 | BKCMP1E |

- [ ] DACs: DAC1 CH1, DAC1 CH2, DAC3 CH1 in "connected to on-chip peripherals" mode. One channel per comparator so each can be calibrated on its own.
- [ ] COMPs: polarity non-inverted (output 1 = over current), hysteresis none, output internal only.
- [ ] TIM1 break (BRK): enable, polarity high, sources COMP1/2/3 (TIM1_AF1 BKCMP1E/2E/3E), BKIN pin source OFF, AOE = 0 (stays latched), OSSI = 1. Write BDTR in one go.
- [x] BKIN pin source disabled in .ioc (SourceBRKDigInput = DISABLE); PC13 stays reserved as TIM1_BKIN but is not a break source. COMP1/2/3 break sources enabled, OSSI/OSSR enabled (Nov 14).
- [ ] After code generation, check MX_TIM1_Init has BreakState = TIM_BREAK_ENABLE, BreakPolarity = HIGH, AutomaticOutput = DISABLE (these are CubeMX defaults, so not written in the .ioc).
- [ ] Optional: BIE in TIM1_DIER, plus the TIM1 break interrupt in NVIC, to log trips (log which COMP via its VALUE bit).
- [ ] Startup calibration, per comparator, motor off, before setting MOE:
  - Sense node at 0 A = 103 mV = DAC code about 128 (3.3 V / 4096 = 0.806 mV per code).
  - Sweep the DAC upward from code 118 to 140, waiting about 5 us per step. The first code where COMP VALUE goes 0 is the flip point.
  - offset = flip - 128 (expect within about +/-15; otherwise fault).
  - Trip code = 205 + offset (205 = 165 mV = 103 mV + 3.2 A x 19.4 mV/A).
  - This cancels comparator offset (-9 to +3 mV), DAC error and divider tolerance. Residual is about +/-1 code, about +/-0.04 A.
- [ ] CubeMX Output Configuration: Automatic Output State = Disable, OSSI = Enable, OSSR = Enable, Lock = Off, Dead Time = 0; CH1-3 and CH1N-3N Idle State = Reset. BRK2 disabled.
- [ ] Re-arm only on purpose: clear BIF, then set MOE.
- [ ] Bring-up: test by temporarily setting the trip to about 1 A (code about 152). Scope whether the 100 pF filter is enough; if not, add blanking (BLANKSEL = TIM1 OC5) over the low-side turn-on edge.

## Current sensing (OPAMP + ADC)
- [x] OPAMP1/2/3 all PGA gain 16 with VINM0 Kelvin (Nov 14).
- [ ] ADC: .ioc has ADC1 = VOPAMP1 and ADC2 = VOPAMP2. VOPAMP3 is ticked on ADC2 (Nov 14), but ADC2 still has only 1 regular conversion, so VOPAMP3 needs its own rank (regular rank 2 or an injected rank) when sampling is set up. With internal op-amp outputs: OPAMP1 to ADC1_IN13, OPAMP2 to ADC2_IN16, OPAMP3 to ADC2_IN18. B and C share ADC2, so simultaneous pairs are A+B or A+C.
- [ ] DECIDED Nov 16: all ADC channels via DMA. DMA only serves REGULAR conversions (injected results sit in JDRx with no DMA), so current sensing uses the regular group, hardware-triggered by TIM1, not injected.
- [ ] ADC1 + ADC2 in dual regular-simultaneous mode, one DMA channel (DMA1, circular, 32-bit word) reading the common data register ADC12_CDR (ADC12_CCR: DUAL = 00110 regular simultaneous, MDMA = 10 for 12-bit so each DMA request moves one 32-bit word = ADC1 result low half + ADC2 result high half, DMACFG = 1 circular). Both ADCs need the same number of ranks and matching sample times per rank.
- [ ] Ranks: ADC1 = [1 VOPAMP1 (I_A), 2 PA2 ADC1_IN3 (VBUS)]; ADC2 = [1 VOPAMP2 (I_B), 2 VOPAMP3 (I_C)]. Result: I_A + I_B together, then VBUS + I_C about 0.2 us later. Buffer = 2 words.
- [ ] Trigger the regular group from TIM1 at the center of the low-side on-time (counter underflow in center-aligned mode, e.g. TRGO2 on OC4REF with CCR4 near ARR, or update event with RCR = 1 so only one of the two updates fires).
- [ ] Scale: output = 1.65 V + 0.31 V/A (20 mOhm x 31/32 x 16). Range about +/-4.8 A.
- [ ] Set OPAINTOEN = 1 in OPAMPx_CSR in the same write as (or before) OPAEN. Why: with OPAINTOEN = 0 the op-amp output drives its VOUT pin. OPAMP3 VOUT is PB1, which sits on the INC+ net (its own + input), so it would latch to a rail; OPAMP1 VOUT is PA2, the VBUS_SENSE pin (RM0440 25.3.1, Nov 17).
- [ ] ADC sample time on the VOPAMP1/2/3 channels must be at least 200 ns (DS12589 TS_OPAMP_VOUT, OPAINTOEN = 1). The .ioc has 2.5 cycles; at a 42.5 MHz ADC clock use 12.5 cycles (294 ns) or more, the same on the paired rank of the other ADC (Nov 17).
- [ ] Firmware current limit 2.5 A (the hardware trip at 3.2 A is the backstop).

## Bus voltage
- [ ] VBUS divider to PA2 = ADC1_IN3 (enabled in .ioc; divider drawn Nov 17: 100k + 5.1k top, 10k bottom, 10 nF, so VBUS = V_PA2 x 11.51). Add PA2 to an ADC1 rank when sampling is set up. Use it for over- and under-voltage cutoff and to limit braking current, since the eFuse/adapter can't absorb energy pushed back from the motor.

## Power-up / protection notes
- [ ] eFuse (TPS26632) is latch-off: after a 4 A overload, power-cycle. Nothing to configure in firmware.
- [ ] DRV8300 inputs have 200 kOhm pull-downs, so the FETs stay off until TIM1 runs.
- [ ] EFUSE_FLT on PB6: GPIO input with pull-up (open-drain, low = fault). PB6 is not in the .ioc yet (Nov 17).
- [ ] Set PWR_CR3.UCPD1_DBDIS = 1 early in startup. Why: PB6 is UCPD1_CC1, and a high level on PA9 (PWM_HSB) switches on a 5.1 kOhm dead-battery pull-down on PB6, which would read as a false eFuse fault (DS12589 pinout note 6). The .ioc's 'Disable Dead Battery Signals' does this in HAL; our own drivers must do it themselves (Nov 17).

## Comms and misc
- [ ] USART3: TX PB10, RX PB11 (via USB-UART bridge). Set the baud rate.
- [ ] I2C1: SCL PA15, SDA PB7, AS5600 encoder.
- [ ] I2C1 timing in the .ioc (0x40B285C2) is 100 kHz standard mode. 400 kHz cuts a 2-byte angle read about 4x; decide when writing the encoder driver (Nov 17).
- [x] Status LED on PA11 (GPIO output, .ioc and schematic agree, Nov 16). Wired source-style: PA11 -> LED -> 150 ohm -> GND, so drive HIGH to turn on. PC13 is no longer in the .ioc.
- [ ] PB8 = BOOT0: keep the hardware pull-down; no GPIO use.
