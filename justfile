build:
    cd guest && lake build
    mkdir -p guest/.lake/packages/dummy/.lake/build/ir/
    mkdir -p guest_build/risc0_ir
    rsync -a --prune-empty-dirs --include '*/' --include '*.c' --exclude '*' guest/.lake/build/ir/ guest/.lake/packages/*/.lake/build/ir/ guest_build/risc0_ir/
    cd guest_build && just build
    cp guest_build/_build/libGuest.a methods/guest/lib/libGuest.a
    cp guest_build/_build/libGuest.a methods/guest-eth2-noinit/lib/libGuest.a
    cp guest_build/_build/libGuest.a methods/guest-eth2-init/lib/libGuest.a
    cargo build --release

clean:
    cd guest && lake clean
    cd guest_build && just clean
    rm -rf guest_build/risc0_ir/
    rm -f methods/guest/lib/libGuest.a
    rm -f methods/guest-eth2-noinit/lib/libGuest.a
    rm -f methods/guest-eth2-init/lib/libGuest.a
    cargo clean

bench-execute:
    cargo run --release --bin benchmark -- --mode execute

bench-prove:
    cargo run --release --bin benchmark -- --mode prove

bench-profile-lean N="1000":
    RISC0_PPROF_OUT=lean_profile.pb RISC0_DEV_MODE=1 cargo run --release --bin benchmark -- --guest lean --inputs {{N}}

bench-profile-rust N="1000":
    RISC0_PPROF_OUT=rust_profile.pb RISC0_DEV_MODE=1 cargo run --release --bin benchmark -- --guest rust --inputs {{N}}

bench-eth2-execute:
    RISC0_DEV_MODE=1 cargo run --release --bin benchmark -- --suite eth2 --mode execute --inputs 10 --guest all

bench-eth2-prove:
    cargo run --release --bin benchmark -- --suite eth2 --mode prove --inputs 10 --guest all

# IR trace pipeline
dump-ir:
    cd guest && lake env lean --run IrDump.lean && mv ir_program.json ../ir_program.json

run-ir-trace INPUT ENTRY="risc0_main_eth2":
    cargo run -p ir-trace -- --ir ir_program.json --input {{INPUT}} --output trace.bin --entry {{ENTRY}}

verify-ir-trace INPUT ENTRY="risc0_main_eth2":
    RISC0_DEV_MODE=1 cargo run --release --bin ir-trace-host -- --ir ir_program.json --input {{INPUT}} --entry {{ENTRY}}

verify-ir-trace-prove INPUT ENTRY="risc0_main_eth2":
    cargo run --release --bin ir-trace-host -- --ir ir_program.json --input {{INPUT}} --entry {{ENTRY}}

# IR Trace benchmark (compare with bench-eth2-execute)
bench-ir-trace VALIDATORS="10,100":
    #!/usr/bin/env bash
    set -euo pipefail
    for n in $(echo {{VALIDATORS}} | tr ',' ' '); do
        echo "=== N=$n validators ==="
        cargo run --release -p ir-trace --bin gen-eth2-input -- $n /tmp/eth2_input_${n}.bin
        cargo run --release -p ir-trace -- \
          --ir ir_program_filtered.json \
          --input /tmp/eth2_input_${n}.bin \
          --entry risc0_main_eth2 --bench --runs 3
        echo
    done

# IR trace zkVM cycle count (execute mode, no proving)
bench-ir-trace-cycles VALIDATORS="10":
    #!/usr/bin/env bash
    set -euo pipefail
    for n in $(echo {{VALIDATORS}} | tr ',' ' '); do
        echo "=== N=$n validators ==="
        cargo run --release -p ir-trace --bin gen-eth2-input -- $n /tmp/eth2_input_${n}.bin
        cargo run --release -p ir-trace --bin ir-trace -- \
          --ir ir_program_filtered.json \
          --input /tmp/eth2_input_${n}.bin \
          --entry risc0_main_eth2 --output /tmp/trace_${n}.bin
        cargo run --release --bin ir-trace-host -- \
          --ir ir_program_filtered.json \
          --input /tmp/eth2_input_${n}.bin \
          --trace /tmp/trace_${n}.bin --mode execute
        echo
    done
