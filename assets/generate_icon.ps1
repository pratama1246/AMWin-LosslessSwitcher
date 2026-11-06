Add-Type -AssemblyName System.Drawing

function New-RoundedRectangle {
    param(
        [System.Drawing.RectangleF]$rect,
        [float]$radius
    )
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $diameter = [float]($radius * 2.0)
    $arc = New-Object System.Drawing.RectangleF($rect.X, $rect.Y, $diameter, $diameter)
    
    $path.AddArc($arc, [float]180.0, [float]90.0)
    $arc.X = [float]($rect.Right - $diameter)
    $path.AddArc($arc, [float]270.0, [float]90.0)
    $arc.Y = [float]($rect.Bottom - $diameter)
    $path.AddArc($arc, [float]0.0, [float]90.0)
    $arc.X = [float]$rect.Left
    $path.AddArc($arc, [float]90.0, [float]90.0)
    $path.CloseFigure()
    return $path
}

function Render-RateMatchLogo {
    param([int]$size)
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.ScaleTransform(($size / 512.0), ($size / 512.0))
    $tile = New-RoundedRectangle ([System.Drawing.RectangleF]::new(12, 12, 488, 488)) 108
    $white = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
    $teal = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(0, 107, 120))
    $g.FillPath($white, $tile)
    $mark = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $mark.AddLine(386, 112, 226, 112)
    $mark.AddBezier(226,112,204,112,188,120,176,140)
    $mark.AddLine(176,140,106,246)
    $mark.AddBezier(106,246,94,268,110,290,142,290)
    $mark.AddLine(142,290,226,290)
    $mark.AddLine(226,290,266,230)
    $mark.AddLine(266,230,174,230)
    $mark.AddLine(174,230,222,166)
    $mark.AddLine(222,166,350,166)
    $mark.CloseFigure()
    $g.FillPath($teal, $mark)
    $rotation = [System.Drawing.Drawing2D.Matrix]::new()
    $rotation.RotateAt(180, [System.Drawing.PointF]::new(256,256))
    $mark.Transform($rotation)
    $g.FillPath($teal, $mark)
    $rotation.Dispose()
    $mark.Dispose()
    $tile.Dispose()
    $white.Dispose()
    $teal.Dispose()
    $g.Dispose()
    return $bmp
}

$sizes = @(16, 24, 32, 48, 64, 128, 256)
$images = @()

foreach ($s in $sizes) {
    $bmp = Render-RateMatchLogo $s
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $images += [PSCustomObject]@{
        Size = $s
        Bytes = $ms.ToArray()
    }
    if ($s -eq 256) {
        $bmp.Save((Join-Path $PSScriptRoot "logo.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    }
    $bmp.Dispose()
    $ms.Dispose()
}

# Write multi-frame ICO file
$icoPath = (Join-Path $PSScriptRoot "app.ico")
$fs = [System.IO.File]::Open($icoPath, [System.IO.FileMode]::Create)
$bw = New-Object System.IO.BinaryWriter($fs)

$bw.Write([uint16]0)
$bw.Write([uint16]1)
$bw.Write([uint16]$images.Count)

$offset = 6 + (16 * $images.Count)

foreach ($img in $images) {
    $w = if ($img.Size -eq 256) { 0 } else { $img.Size }
    $h = if ($img.Size -eq 256) { 0 } else { $img.Size }
    $bw.Write([byte]$w)
    $bw.Write([byte]$h)
    $bw.Write([byte]0)
    $bw.Write([byte]0)
    $bw.Write([uint16]1)
    $bw.Write([uint16]32)
    $bw.Write([uint32]$img.Bytes.Length)
    $bw.Write([uint32]$offset)
    $offset += $img.Bytes.Length
}

foreach ($img in $images) {
    $bw.Write($img.Bytes)
}

$bw.Flush()
$bw.Close()
$fs.Close()

Write-Output "Successfully generated RateMatch assets\logo.png and assets\app.ico!"
