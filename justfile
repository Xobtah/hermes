build PROFILE="dev" TARGET="x86_64-pc-windows-gnu":
    cargo b -p agent --target {{TARGET}} --profile {{PROFILE}} && \
    cargo b -p packer --features windows-service --target {{TARGET}} --profile {{PROFILE}} && \
    cargo b -p stager --target {{TARGET}} --profile {{PROFILE}} && \
    cargo b -p dropper --features windows-service --target {{TARGET}} --profile {{PROFILE}}

debug PROFILE="dev" TARGET="x86_64-pc-windows-gnu":
    cargo b -p agent --target {{TARGET}} --profile {{PROFILE}} && \
    cargo b -p packer --target {{TARGET}} --profile {{PROFILE}} && \
    cargo b -p stager --target {{TARGET}} --profile {{PROFILE}} && \
    cargo b -p dropper --target {{TARGET}} --profile {{PROFILE}}

drop PROFILE="debug":
    cp target/x86_64-pc-windows-gnu/{{PROFILE}}/dropper.exe ~/Desktop
    cp target/x86_64-pc-windows-gnu/{{PROFILE}}/panacea.exe ~/Desktop
