# Pre-dev setup: build bat-agent so it's available alongside bat-shell during dev
Write-Host "Building bat-agent..."
cargo build -p bat-agent
if ($LASTEXITCODE -ne 0) { exit 1 }
Write-Host "bat-agent ready."
# Start the UI dev server (Tauri needs this to stay running)
Set-Location crates/bat-shell/ui
npm run dev
