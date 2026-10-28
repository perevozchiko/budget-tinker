// Keep the approved artwork intact, adding macOS optical padding on a transparent canvas.
import AppKit
let arguments = CommandLine.arguments
guard arguments.count == 3 else { fatalError("Usage: icon-padding.swift INPUT.png OUTPUT.png") }
let source = NSImage(contentsOfFile: arguments[1])!
let canvas = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 1024, pixelsHigh: 1024,
                             bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
                             isPlanar: false, colorSpaceName: .deviceRGB,
                             bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: canvas)
NSColor.clear.setFill()
NSRect(x: 0, y: 0, width: 1024, height: 1024).fill(using: .copy)
NSGraphicsContext.current?.imageInterpolation = .high
let bounds = NSRect(x: 100, y: 100, width: 824, height: 824)
NSBezierPath(roundedRect: bounds, xRadius: 166, yRadius: 166).addClip()
source.draw(in: bounds,
            from: .zero, operation: .sourceOver, fraction: 1)
NSGraphicsContext.restoreGraphicsState()
try canvas.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: arguments[2]))
