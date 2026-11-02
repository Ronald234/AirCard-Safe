param([Parameter(Mandatory)][string]$Path)
$ErrorActionPreference = 'Stop'
[byte[]]$bytes = [IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $Path))
if ($bytes.Length -lt 256 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) { throw 'Not a Windows MZ executable' }
$pe = [BitConverter]::ToInt32($bytes, 0x3c)
if ($pe -lt 64 -or $pe -gt $bytes.Length - 256) { throw 'Invalid PE offset' }
if ([BitConverter]::ToUInt32($bytes,$pe) -ne 0x4550) { throw 'Missing PE signature' }
if ([BitConverter]::ToUInt16($bytes,$pe+4) -ne 0x8664) { throw 'Not AMD64 Windows code' }
$characteristics = [BitConverter]::ToUInt16($bytes,$pe+22)
if (($characteristics -band 2) -eq 0 -or ($characteristics -band 0x2000) -ne 0) { throw 'Not an executable application' }
if ([BitConverter]::ToUInt16($bytes,$pe+24) -ne 0x20b) { throw 'Not PE32+' }
$subsystem = [BitConverter]::ToUInt16($bytes,$pe+24+68)
if ($subsystem -ne 2) { throw 'Expected Windows GUI subsystem' }
[pscustomobject]@{Format='PE32+';Machine='AMD64';Subsystem='Windows GUI';Length=$bytes.Length;SHA256=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}

