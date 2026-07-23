# Mingtily Windows build wrapper

Write-Host ""
Write-Host "========================================"
Write-Host "   Mingtily GPU Build"
Write-Host "========================================"
Write-Host ""

& ".\build-gpu.bat" $args
exit $LASTEXITCODE
