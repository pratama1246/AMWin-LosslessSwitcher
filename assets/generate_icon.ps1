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

function Render-LosslessLogo {
    param([int]$size)
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    [float]$scale = [float]$size / 512.0

    # Solid Squircle Container (Flat Dark Charcoal)
    [float]$margin = 16.0 * $scale
    [float]$w = 480.0 * $scale
    [float]$radius = 112.0 * $scale
    $rect = New-Object System.Drawing.RectangleF($margin, $margin, $w, $w)
    $squirclePath = New-RoundedRectangle $rect $radius

    $bgBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 14, 17, 24))
    $g.FillPath($bgBrush, $squirclePath)

    # Solid Flat Border
    [float]$strokeWidth = [Math]::Max(1.0, 4.0 * $scale)
    $borderPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 30, 36, 51), $strokeWidth)
    $g.DrawPath($borderPen, $squirclePath)

    # Flat Solid Brushes (No gradients, no blur, no shadows)
    $cyanBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 0, 240, 255))
    $pinkBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 255, 45, 85))

    $pills = @(
        # L bars (Solid Cyan)
        @{ x = 116.0; y = 130.0; w = 28.0; h = 230.0; brush = $cyanBrush },
        @{ x = 156.0; y = 170.0; w = 28.0; h = 190.0; brush = $cyanBrush },
        @{ x = 196.0; y = 270.0; w = 28.0; h = 90.0;  brush = $cyanBrush },
        @{ x = 236.0; y = 324.0; w = 36.0; h = 36.0;  brush = $cyanBrush },
        # S bars (Solid Pink)
        @{ x = 284.0; y = 130.0; w = 100.0; h = 28.0; brush = $pinkBrush },
        @{ x = 284.0; y = 166.0; w = 28.0;  h = 48.0; brush = $pinkBrush },
        @{ x = 274.0; y = 238.0; w = 102.0; h = 28.0; brush = $pinkBrush },
        @{ x = 356.0; y = 274.0; w = 28.0;  h = 52.0; brush = $pinkBrush },
        @{ x = 260.0; y = 332.0; w = 124.0; h = 28.0; brush = $pinkBrush }
    )

    foreach ($p in $pills) {
        [float]$px = [float]$p.x * $scale
        [float]$py = [float]$p.y * $scale
        [float]$pw = [float]$p.w * $scale
        [float]$ph = [float]$p.h * $scale
        $pRect = New-Object System.Drawing.RectangleF($px, $py, $pw, $ph)
        [float]$pRad = [Math]::Min($pw, $ph) / 2.0
        $pPath = New-RoundedRectangle $pRect $pRad
        $g.FillPath($p.brush, $pPath)
    }

    # Flat Status Dots (for >= 48px)
    if ($size -ge 48) {
        $dotWhite = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 255, 255, 255))
        $g.FillEllipse($cyanBrush, [float](388.0 * $scale), [float](94.0 * $scale), [float](12.0 * $scale), [float](12.0 * $scale))
        $g.FillEllipse($dotWhite,  [float](410.0 * $scale), [float](94.0 * $scale), [float](12.0 * $scale), [float](12.0 * $scale))
        $g.FillEllipse($pinkBrush, [float](432.0 * $scale), [float](94.0 * $scale), [float](12.0 * $scale), [float](12.0 * $scale))
    }

    $g.Dispose()
    return $bmp
}

$sizes = @(16, 24, 32, 48, 64, 128, 256)
$images = @()

foreach ($s in $sizes) {
    $bmp = Render-LosslessLogo $s
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $images += [PSCustomObject]@{
        Size = $s
        Bytes = $ms.ToArray()
    }
    if ($s -eq 256) {
        $bmp.Save("D:\Project-Web\lossless-switcher-x64\assets\logo.png", [System.Drawing.Imaging.ImageFormat]::Png)
    }
}

# Write multi-frame ICO file
$icoPath = "D:\Project-Web\lossless-switcher-x64\assets\app.ico"
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

Write-Output "Successfully generated flat assets\logo.png and assets\app.ico!"
