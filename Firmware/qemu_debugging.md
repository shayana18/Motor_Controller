# Debugging the firmware in QEMU (Docker + GDB)

How to run the startup code in an emulated Cortex-M4 and step through it with GDB, before any hardware exists.

## Why it is set up this way

- **Docker:** this Mac is Intel, and Homebrew's `qemu` build doesn't support it. So QEMU and GDB run inside an Ubuntu container. Cargo still builds on the Mac, and the container sees the same files through a mounted folder.
- **QEMU machine `olimex-stm32-h405`:** QEMU has no STM32G431 model. This board is an STM32F405, which has the same Cortex-M4F core and puts flash at `0x0800_0000` and RAM at `0x2000_0000`, like the G431. So the vector table, reset handler, `.data`/`.bss` setup and FPU code behave the same way. The peripherals (and their addresses) are different, so G431 peripheral drivers can't be tested here.
- **Your linker script fits inside the F405:** it uses FLASH 128K at `0x08000000` and RAM 32K at `0x20000000`, which is the G431's size. The F405 has more of both, so nothing overflows.
- **Two processes:** QEMU is the "board" and also runs a GDB server (the GDB remote protocol over TCP port 3333). GDB is the client that connects to it. Later, probe-rs will be the GDB server for the real board, and the GDB side stays the same.

## Paths used below

| What | Path on the Mac | Path in the container |
|---|---|---|
| Firmware folder (mounted) | `~/Desktop/motor_control_project/firmware` | `/work` |
| Binary crate | `firmware/app` | `/work/app` |
| Debug ELF | `firmware/app/target/thumbv7em-none-eabihf/debug/app` | `/work/app/target/thumbv7em-none-eabihf/debug/app` |

The target comes from `app/.cargo/config.toml` (`target = "thumbv7em-none-eabihf"`), so a plain `cargo build` in `app/` writes to that path. Use the **debug** build for GDB: release builds are optimised, so lines and variables jump around or disappear.

## Step 1: Build on the Mac

```sh
cd ~/Desktop/motor_control_project/firmware/app
cargo build
```

Check that the ELF looks right before starting QEMU:

```sh
cargo objdump --bin app -- -h
```

`.vector_table` should be at `08000000` and `.text` right after it.

## Step 2: Create the container (first time only)

Make sure Docker Desktop is running, then:

```sh
cd ~/Desktop/motor_control_project/firmware
docker run -it --name qemu-dbg -v "$PWD":/work -w /work ubuntu:24.04 bash
```

- `--name qemu-dbg` gives the container a fixed name, so the second pane can find it with `docker exec` without looking up an ID.
- `-v "$PWD":/work` mounts the firmware folder at `/work`. A rebuild on the Mac shows up in the container at once.
- `-w /work` starts the shell in that folder.
- No `--rm`, so the container (and the tools installed in it) survives after you exit. Delete it with `docker rm qemu-dbg` if you ever want a fresh one.

Inside the container, install the two tools:

```sh
apt update && apt install -y qemu-system-arm gdb-multiarch
qemu-system-arm --version
```

`gdb-multiarch` is Ubuntu's GDB built for every architecture, ARM included. It does the same job as `arm-none-eabi-gdb`.

### Every time after that

```sh
docker start -ai qemu-dbg
```

`start` restarts the stopped container, `-a` attaches your terminal, `-i` makes it interactive. The tools are still installed. If you used `--rm` instead, the container is deleted on exit and you have to redo the `apt install` every time.

## Step 3: Start QEMU (pane 1)

```sh
qemu-system-arm -machine olimex-stm32-h405 -nographic \
  -S -gdb tcp::3333 \
  -kernel app/target/thumbv7em-none-eabihf/debug/app
```

- `-machine olimex-stm32-h405`: the emulated board (STM32F405, Cortex-M4F).
- `-nographic`: no window. Serial output and the QEMU monitor share this terminal.
- `-S`: freezes the CPU at reset, before the first instruction. Without it the code would run before GDB connects and you'd miss the reset handler.
- `-gdb tcp::3333`: starts the GDB server on port 3333. (`-s` is the shortcut for `-gdb tcp::1234`. If you use it, connect to `:1234` instead.)
- `-kernel <elf>`: loads the ELF's sections at their addresses (the vector table into flash at `0x08000000`).

The pane then looks frozen. That's expected: the CPU is held until GDB tells it to run.

**Quit QEMU:** press `Ctrl-A`, then `X`. (`Ctrl-A C` toggles the QEMU monitor, where `info registers` and `system_reset` also work.)

## Step 4: Start GDB (pane 2)

Open a second terminal pane on the Mac and attach to the same container:

```sh
docker exec -it qemu-dbg bash
```

`exec` starts a second shell inside the already running container, so this pane can reach QEMU on `localhost:3333`. The `-w /work` setting carries over, so you're already in `/work`.

```sh
gdb-multiarch -q app/target/thumbv7em-none-eabihf/debug/app
```

Then, in GDB:

```gdb
target remote :3333
set substitute-path /Users/shykizzy/Desktop/motor_control_project/firmware /work
```

- `target remote :3333` connects to QEMU's GDB server. The ELF you passed gives GDB the symbols and debug info. QEMU has the actual running code.
- `set substitute-path ...` is needed because the debug info stores your Mac's source paths, and the container sees the same files at `/work`. Without it `list` and `layout src` can't find your `.rs` files. (Rust's own `core` sources live in paths that don't exist in the container, so source view inside `core` functions stays empty. That's normal.)

To skip typing those every time, pass them on the command line:

```sh
gdb-multiarch -q app/target/thumbv7em-none-eabihf/debug/app \
  -ex "target remote :3333" \
  -ex "set substitute-path /Users/shykizzy/Desktop/motor_control_project/firmware /work"
```

## Step 5: First checks after connecting

```gdb
info registers sp pc
x/2xw 0x08000000
```

Expected (verified on Nov 19):
- `sp = 0x20008000`: the first word of the vector table, `ORIGIN(RAM) + LENGTH(RAM)` = `0x2000_0000 + 32K`. The core loads it into SP at reset.
- `pc` = the address of `reset_handler`.
- The second word of the vector table is the reset vector. It must be **odd** (the Thumb bit). The core clears bit 0 when it loads PC, which is why `pc` is one less than the stored word. An even value would HardFault on the very first instruction.

Then step through the reset handler with `stepi` and watch RAM being written (see the cheat sheet).

## Edit, rebuild, debug again

1. In GDB: `quit` (answer `y` to detach).
2. In the QEMU pane: `Ctrl-A` then `X`.
3. On the Mac: `cargo build` in `app/`.
4. Start QEMU again (Step 3), then GDB (Step 4). Press the up arrow in each pane to repeat the last command.

QEMU only reads the ELF at start, so restarting it is the simplest way to load a new build.

## Closing down

- `exit` in the `docker exec` pane closes only that shell.
- `exit` in the first pane stops the container. `docker start -ai qemu-dbg` brings it back with the tools still installed.

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| `Connection refused` on `target remote` | QEMU isn't running, it was started without `-gdb tcp::3333`, or GDB is running outside the container. |
| `No such file or directory` for the ELF | Wrong folder: you must be in `/work` (the firmware folder), and you must have built on the Mac first. |
| `No symbol table is loaded` | GDB was started without the ELF. Use `file app/target/thumbv7em-none-eabihf/debug/app`. |
| `pc` at an odd-looking address or `0x00000000`, or a HardFault straight away | The reset vector is even (missing Thumb bit), or the vector table isn't at `0x08000000`. Check `x/2xw 0x08000000` and `cargo objdump --bin app -- -h`. |
| `docker: Error ... name "/qemu-dbg" is already in use` | The container already exists. Use `docker start -ai qemu-dbg` instead of `docker run`. |
| `step`/`next` jumps over the whole reset handler | It's written in `global_asm!`, which has no Rust line info. Use `stepi`/`nexti` there. |

---

# GDB cheat sheet

Most commands have a short form, shown in brackets. Pressing Enter on an empty line repeats the last command, which is handy for `stepi`.

## Connect and control

| Command | What it does |
|---|---|
| `target remote :3333` | Connect to the GDB server (QEMU now, probe-rs later). |
| `continue` (`c`) | Let the CPU run until a breakpoint, a fault, or `Ctrl-C`. |
| `Ctrl-C` | Pause a running target. GDB shows where it stopped. |
| `monitor system_reset` | Passes `system_reset` to QEMU's monitor, which resets the emulated chip. GDB may still show the old registers afterwards, so run `maintenance flush register-cache` and then `info registers`. |
| `detach` | Disconnect and leave the target running. |
| `quit` (`q`) | Leave GDB. QEMU keeps running until you quit it with `Ctrl-A X`. |

## Breakpoints

| Command | What it does |
|---|---|
| `break main` (`b`) | Stop when execution reaches the symbol `main`. Your `entry!` macro exports the symbol `main`, so this is a good first stop after the reset handler. |
| `break reset_handler` | Stop at the start of your reset handler. |
| `break *0x08000010` | Stop at an exact address. The `*` means "address", not "symbol". |
| `break main.rs:12` | Stop at a source line. |
| `tbreak main` | A temporary breakpoint: deleted automatically the first time it's hit. |
| `info breakpoints` (`i b`) | List breakpoints with their numbers. |
| `delete 2` (`d 2`) | Delete breakpoint 2. `delete` alone deletes all of them. |
| `disable 2` / `enable 2` | Turn breakpoint 2 off or on without deleting it. |
| `watch MY_STATIC` | Stop whenever that variable is written. Good for catching who changes a value. |

## Stepping

| Command | What it does |
|---|---|
| `step` (`s`) | One **source line**, going into function calls. |
| `next` (`n`) | One source line, running function calls without stopping inside them. |
| `stepi` (`si`) | One **machine instruction**, going into calls. Use it in the reset handler, since `global_asm!` has no source lines. |
| `nexti` (`ni`) | One instruction, but runs a `bl` call through to its return. |
| `finish` | Run until the current function returns. Never returns for `-> !` functions, so don't use it in `main`. |
| `until 25` | Run until line 25 of the current file. Useful to get out of a loop in one go. |

## Registers

| Command | What it does |
|---|---|
| `info registers` (`i r`) | All core registers: r0 to r12, sp, lr, pc, xpsr. |
| `info registers pc sp` | Just those two. |
| `p/x $pc` | Print a register in hex. `/x` = hex, `/d` = decimal, `/t` = binary. |
| `p $sp` | Stack pointer. At reset it should equal the first vector table word. |
| `p/t $xpsr` | xPSR in binary. Bit 24 is the T (Thumb) bit and must be 1. The low 9 bits are the exception number (0 = thread mode, 3 = HardFault). |
| `set $r0 = 5` | Write a register. |

## Memory

`x/<count><format><size> <address>` examines memory. Format `x` = hex, `d` = decimal, `i` = instructions, `s` = string. Size `b` = byte, `h` = 2 bytes, `w` = 4 bytes (a word).

| Command | What it does |
|---|---|
| `x/2xw 0x08000000` | The first two vector table words: initial SP, reset vector. |
| `x/16xw 0x08000000` | The first 16 vector table entries: SP, Reset, NMI, HardFault, MemManage, BusFault, UsageFault, 4 reserved, SVCall, 2 reserved, PendSV, SysTick. Every handler address must be odd (Thumb bit). Reserved entries are 0. |
| `x/4bx &MY_STATIC` | The 4 bytes of a static. Run it before and after the `.bss`/`.data` loops to see them zeroed or filled in. |
| `x/8xw 0x20000000` | The start of RAM, where `.bss` and `.data` live. |
| `x/8xw $sp` | The top of the stack. |
| `p &MY_STATIC` | Print the address of a symbol (shows whether it landed in RAM or flash). |
| `info symbol 0x08000010` | Which symbol an address belongs to. |
| `set {int}0x20000000 = 0x1234` | Write a word to memory. |

## Code

| Command | What it does |
|---|---|
| `disassemble` | The instructions of the current function. An `=>` marks the next one to run. |
| `disassemble reset_handler` | The instructions of a named function. |
| `x/6i $pc` | The next 6 instructions from the PC. |
| `display/i $pc` | Print the next instruction after every step. Pairs well with `stepi`. `undisplay` removes it. |
| `list` (`l`) | Source around the current line. |
| `info line *$pc` | Which source file and line the current address comes from. |

## Where am I

| Command | What it does |
|---|---|
| `backtrace` (`bt`) | The call stack: which function called which. Short in your code, since `reset_handler` jumps to `main` with `bx`, not a call. |
| `frame 1` (`f 1`) | Move to the caller's frame to look at its variables. |
| `info locals` | Local variables in the current frame (debug build only). |
| `p my_var` | Print a variable. `p/x my_var` for hex. |
| `display my_var` | Print it automatically after every step. |

## TUI layouts

Text panes that update as you step:

| Command | What it does |
|---|---|
| `layout asm` | Disassembly pane with the current instruction highlighted. |
| `layout regs` | Adds a register pane. Changed registers are highlighted after each step. |
| `layout src` | Source pane. Needs `set substitute-path` (Step 4). |
| `layout split` | Source and disassembly together. |
| `tui disable` | Back to the plain command line. `Ctrl-X A` toggles it too. |
| `refresh` (`Ctrl-L`) | Redraw if the screen gets garbled. |

## A first session, start to finish

```gdb
target remote :3333
info registers sp pc        # sp = 0x20008000, pc = reset_handler
x/2xw 0x08000000            # vector table: SP, then an odd reset vector
layout regs
display/i $pc
stepi                       # Enter repeats it; watch r0/r1 walk through .bss
break main
continue                    # runs the rest of the reset handler, stops in main
bt
quit
```
