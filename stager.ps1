# Enabling execution of PS1 scripts: https://stackoverflow.com/questions/4037939/powershell-says-execution-of-scripts-is-disabled-on-this-system
# Set-ExecutionPolicy RemoteSigned
# Set-ExecutionPolicy Restricted

if (-NOT ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Warning "You do not have Administrator rights to run this script! Please re-run this script as Administrator!"
    Break
}

# Create a variable containing some sample data encoded in base64 format
$base64Data = "RosOwb3N0IG5vdCBhbGxvdyBmb3J0aCwgdG8gcmVhbGx5IGZvdWwgdGhpbmdzIHVwIHlvdSBuZWVkIGEgY29tcHV0ZXIuCiAgICAtLSBQYXVsIFIuIEVocmxpY2g="

# Decode the base64 data into a byte array
$byteArray = [Convert]::FromBase64String($base64Data)

# Specify the path where the binary file will be created
$filePath = Join-Path -Path $PSScriptRoot -ChildPath "myBinaryFile.exe"

# Write the decoded byte array to the binary file
[System.IO.File]::WriteAllBytes($filePath, $byteArray)

# Define the properties for the new service
$serviceName = "Agent"
$serviceDisplayName = "Hermes Agent"
$serviceDescription = "This is the Hermes service agent."
$binaryPath = (Get-Item -Path $filePath).FullName
$startupType = "Automatic" # or "Manual", "Disabled"
$serviceAccount = "LocalSystem" # or "NetworkService", "LocalService"
# $credential = Get-Credential -Message "Enter credentials for the service account:"
# $password = ConvertTo-SecureString -String $credential.Password -AsPlainText -Force
# $accountCredentials = New-Object System.Management.Automation.PSCredential ($credential.UserName, $password)

# Register the new service using sc.exe
# sc.exe create $serviceName binPath= "$binaryPath" displayname= "$serviceDisplayName" description= "$serviceDescription" start=$startupType obj="$serviceAccount.$($accountCredentials.GetNetworkIdentity().Value)" password= "$($accountCredentials.GetNetworkCredential().Password)"
New-Service -Name $serviceName -DisplayName $serviceDisplayName -BinaryPathName 'C:\WINDOWS\System32\agent.exe' -Description $serviceDescription -StartupType $startupType -ServiceAccount $serviceAccount

# Start the newly created service
Start-Service -Name $serviceName
