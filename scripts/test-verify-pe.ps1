$ErrorActionPreference='Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ('aircard-pe-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
try {
    $file = Join-Path $root 'fixture.bin'
    [byte[]]$valid = New-Object byte[] 512
    $valid[0]=0x4d; $valid[1]=0x5a
    [BitConverter]::GetBytes([int]128).CopyTo($valid,0x3c)
    [BitConverter]::GetBytes([uint32]0x4550).CopyTo($valid,128)
    [BitConverter]::GetBytes([uint16]0x8664).CopyTo($valid,132)
    [BitConverter]::GetBytes([uint16]2).CopyTo($valid,150)
    [BitConverter]::GetBytes([uint16]0x20b).CopyTo($valid,152)
    [BitConverter]::GetBytes([uint16]2).CopyTo($valid,220)
    [IO.File]::WriteAllBytes($file,$valid)
    $result = & "$PSScriptRoot/verify-pe.ps1" -Path $file
    if ($result.Machine -ne 'AMD64') { throw 'Valid fixture was rejected' }
    $count=1
    foreach ($offset in @(0,128,132,150,152,220)) {
        [byte[]]$bad=$valid.Clone(); $bad[$offset]=0
        [IO.File]::WriteAllBytes($file,$bad)
        $rejected=$false
        try { & "$PSScriptRoot/verify-pe.ps1" -Path $file | Out-Null } catch { $rejected=$true }
        if (-not $rejected) { throw "Malformed header accepted at offset $offset" }
        $count++
    }
    [IO.File]::WriteAllBytes($file,[byte[]]@(0x4d,0x5a))
    $rejected=$false
    try { & "$PSScriptRoot/verify-pe.ps1" -Path $file | Out-Null } catch { $rejected=$true }
    if (-not $rejected) { throw 'Truncated header accepted' }
    $count++
    Write-Output "$count PE verifier tests passed. Fixtures are not compiled applications."
} finally {
    # Only files created above, within the unique test directory.
    Remove-Item -LiteralPath (Join-Path $root 'fixture.bin') -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $root -Force
}

