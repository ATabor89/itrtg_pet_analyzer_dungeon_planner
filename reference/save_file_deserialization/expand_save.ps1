# Expands an ITRTG save file into an indented key:value tree.
# Usage: .\expand_save.ps1 [-SavePath <path>] [-OutPath <path>]
param(
    [string]$SavePath = "$PSScriptRoot\ManualSave_2026-06-09.txt",
    [string]$OutPath  = "$PSScriptRoot\save_expanded.txt"
)

$ErrorActionPreference = 'Stop'

# --- Layer 0: strip 2 junk chars, base64 -> [4-byte len][gzip] -> base64 -> plaintext
function Get-SavePlaintext([string]$path) {
    $raw = (Get-Content $path -Raw).Trim()
    $bytes = [Convert]::FromBase64String($raw.Substring(2))
    $ms  = [System.IO.MemoryStream]::new($bytes, 4, ($bytes.Length - 4))
    $gz  = [System.IO.Compression.GZipStream]::new($ms, [System.IO.Compression.CompressionMode]::Decompress)
    $out = [System.IO.MemoryStream]::new()
    $gz.CopyTo($out); $gz.Close()
    $inner = [System.Text.Encoding]::ASCII.GetString($out.ToArray())
    return [System.Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($inner))
}

# A struct string looks like "a:val;b:val;..." -- split on ';' delimiters that
# are followed by "<key>:" or end-of-string. Values themselves never contain ';'
# (they are numbers, plain strings, or base64), so a plain split is safe enough;
# we re-join any fragment that doesn't look like a new "key:" start.
function Split-Fields([string]$s) {
    $parts = $s.Split(';')
    $fields = [System.Collections.Generic.List[string]]::new()
    foreach ($p in $parts) {
        if ($p -eq '') { continue }
        if ($p -cmatch '^[a-zA-Z0-9]{1,4}:' ) {
            $fields.Add($p)
        } elseif ($p -cmatch '^[a-zA-Z0-9]{1,4}$') {
            # bare key with empty value, e.g. "h;" in "g:0;h;i:..."
            $fields.Add($p + ':')
        } elseif ($fields.Count -gt 0) {
            # continuation of previous value that contained a ';'
            $fields[$fields.Count-1] = $fields[$fields.Count-1] + ';' + $p
        } else {
            $fields.Add($p)
        }
    }
    return $fields
}

function Test-LooksLikeStruct([string]$s) {
    return $s -cmatch '^[a-zA-Z0-9]{1,4}:.*;' -or $s -cmatch '^[a-zA-Z0-9]{1,4}:[^;]*$'
}

function Try-DecodeBase64([string]$s) {
    if ($s.Length -lt 8 -or ($s.Length % 4) -ne 0) { return $null }
    if ($s -notmatch '^[A-Za-z0-9+/]+={0,2}$') { return $null }
    try {
        $b = [Convert]::FromBase64String($s)
        $t = [System.Text.Encoding]::UTF8.GetString($b)
        # reject control chars / replacement chars, allow non-ASCII text (e.g. Piñata)
        if ($t -match '[\x00-\x08\x0B\x0C\x0E-\x1F�]') { return $null }
        return $t
    } catch { return $null }
}

$sw = [System.IO.StreamWriter]::new($OutPath, $false, [System.Text.Encoding]::UTF8)

function Write-Node([string]$text, [int]$depth, [string]$label) {
    $ind = '  ' * $depth
    if ($depth -gt 14) { $sw.WriteLine("$ind$label = <max depth> $text"); return }

    # '&'-joined list?
    if ($text.Contains('&')) {
        $elems = $text.Split('&')
        $allB64 = $true
        foreach ($e in $elems) { if ($null -eq (Try-DecodeBase64 $e)) { $allB64 = $false; break } }
        if ($allB64 -and $elems.Count -gt 1) {
            $sw.WriteLine("$ind$label = <list of $($elems.Count)>")
            for ($i = 0; $i -lt $elems.Count; $i++) {
                Write-Node (Try-DecodeBase64 $elems[$i]) ($depth+1) "[$i]"
            }
            return
        }
    }

    # single base64 blob?
    $dec = Try-DecodeBase64 $text
    if ($null -ne $dec -and (Test-LooksLikeStruct $dec)) {
        Write-Node $dec $depth $label
        return
    }
    if ($null -ne $dec -and $dec.Contains('&')) {
        Write-Node $dec $depth $label
        return
    }

    # struct?
    if (Test-LooksLikeStruct $text) {
        $fields = Split-Fields $text
        if ($fields.Count -gt 1 -or $text.EndsWith(';')) {
            $sw.WriteLine("$ind$label =")
            foreach ($f in $fields) {
                $idx = $f.IndexOf(':')
                $k = $f.Substring(0, $idx)
                $v = $f.Substring($idx + 1)
                Write-Node $v ($depth+1) $k
            }
            return
        }
    }

    # leaf
    if ($null -ne $dec) {
        $sw.WriteLine("$ind$label = $text   <b64: $dec>")
    } else {
        $sw.WriteLine("$ind$label = $text")
    }
}

$plain = Get-SavePlaintext $SavePath
Write-Node $plain 0 'root'
$sw.Close()
Write-Host "Wrote $OutPath"
