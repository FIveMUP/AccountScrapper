while ($true) {
    $process = Get-Process | Where-Object {$_.Name -eq "accountscrapper"} # Reemplaza "nombre_del_programa" con el nombre del proceso de tu programa (sin `.exe`).

    if (-not $process) {
        Start-Process "C:\Users\crist\Desktop\FiveMUP\Dev&Tests\AccountScrapper\accountscrapper.exe" # Reemplaza con la ruta completa a tu .exe
        Write-Output "$(Get-Date) - Programa reiniciado"
        Start-Sleep -Seconds 3 # Espera 10 segundos antes de revisar nuevamente.
    }
    else {
        Write-Output "$(Get-Date) - El programa está en funcionamiento"
        Start-Sleep -Seconds 3 # Espera 10 segundos antes de revisar nuevamente.
    }
}