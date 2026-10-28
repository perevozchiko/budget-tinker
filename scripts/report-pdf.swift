// Native vector renderer for the shared report-print display list.
// Compiled at build time and embedded in the application; no runtime Swift tools needed.
import AppKit
import CoreText
import PDFKit

struct Item: Decodable {
    let kind: String
    let x: CGFloat
    let y: CGFloat
    let w: CGFloat
    let h: CGFloat
    let size: CGFloat
    let text: String
    let color: [CGFloat]
    let points: [[CGFloat]]
}
struct Document: Decodable { let pages: [[Item]] }

func run() throws {
    let args = CommandLine.arguments
    guard args.count == 4 else { throw NSError(domain: "Expected input.json output.pdf export|print", code: 1) }
    let document = try JSONDecoder().decode(Document.self, from: Data(contentsOf: URL(fileURLWithPath: args[1])))
    var box = CGRect(x: 0, y: 0, width: 842, height: 595)
    guard let context = CGContext(URL(fileURLWithPath: args[2]) as CFURL, mediaBox: &box, nil) else {
        throw NSError(domain: "Cannot create PDF", code: 2)
    }
    for page in document.pages {
        context.beginPDFPage(nil)
        for item in page {
            let color = CGColor(colorSpace: CGColorSpace(name: CGColorSpace.sRGB)!, components: item.color + [1])!
            context.saveGState()
            context.setFillColor(color)
            context.setStrokeColor(color)
            switch item.kind {
            case "text":
                context.clip(to: CGRect(x: item.x, y: 595 - item.y - item.h, width: item.w, height: item.h))
                let font = CTFontCreateWithName("Helvetica" as CFString, item.size, nil)
                let line = CTLineCreateWithAttributedString(NSAttributedString(string: item.text, attributes: [
                    NSAttributedString.Key(kCTFontAttributeName as String): font,
                    NSAttributedString.Key(kCTForegroundColorAttributeName as String): color
                ]))
                context.textPosition = CGPoint(x: item.x, y: 595 - item.y - item.size)
                CTLineDraw(line, context)
            case "rect":
                context.fill(CGRect(x: item.x, y: 595 - item.y - item.h, width: item.w, height: item.h))
            default:
                if let first = item.points.first {
                    context.move(to: CGPoint(x: first[0], y: 595 - first[1]))
                    for point in item.points.dropFirst() { context.addLine(to: CGPoint(x: point[0], y: 595 - point[1])) }
                    if item.kind == "polygon" { context.closePath(); context.fillPath() }
                    else { context.setLineWidth(1.5); context.strokePath() }
                }
            }
            context.restoreGState()
        }
        context.endPDFPage()
    }
    context.closePDF()
    if args[3] == "print" {
        defer { try? FileManager.default.removeItem(atPath: args[2]) }
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        app.finishLaunching()
        let info = NSPrintInfo.shared
        info.orientation = .landscape
        guard let pdf = PDFDocument(url: URL(fileURLWithPath: args[2])),
              let operation = pdf.printOperation(for: info, scalingMode: .pageScaleToFit, autoRotate: true) else {
            throw NSError(domain: "Cannot prepare PDF printing", code: 3)
        }
        app.activate(ignoringOtherApps: true)
        operation.run()
    }
}
do { try run() } catch { fputs("\(error)\n", stderr); exit(1) }
