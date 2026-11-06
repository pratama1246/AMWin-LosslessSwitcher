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

function Render-RateAlignLogo {
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
    $graphite = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(32, 38, 46), 48)
    $signal = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(0, 107, 120), 48)
    foreach ($pen in @($signal, $graphite)) {
        $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
        $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
        $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    }
    $upper = [System.Drawing.PointF[]]@([System.Drawing.PointF]::new(112,132), [System.Drawing.PointF]::new(256,132), [System.Drawing.PointF]::new(256,216), [System.Drawing.PointF]::new(400,216))
    $lower = [System.Drawing.PointF[]]@([System.Drawing.PointF]::new(112,380), [System.Drawing.PointF]::new(256,380), [System.Drawing.PointF]::new(256,296), [System.Drawing.PointF]::new(400,296))
    $g.DrawLines($signal, $upper)
    $g.DrawLines($graphite, $lower)
    $signal.Dispose()
    $graphite.Dispose()
    $tile.Dispose()
    $white.Dispose()
    $teal.Dispose()
    $g.Dispose()
    return $bmp
}

$sizes = @(16, 24, 32, 48, 64, 128, 256)
$images = @()

foreach ($s in $sizes) {
    $bmp = Render-RateAlignLogo $s
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

Write-Output "Successfully generated RateAlign assets\logo.png and assets\app.ico!"
