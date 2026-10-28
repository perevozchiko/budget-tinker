// Validate page count, A4 bounds and every text cell in the actual PDFs.
// Usage: swift scripts/verify-report-pdfs.swift /output/from/examples/report_pdf
// Also rasterizes each first page to PNG for visual inspection.
import AppKit
import PDFKit
let root=URL(fileURLWithPath:CommandLine.arguments[1])
func normalized(_ value:String)->String { value.split(whereSeparator:{$0.isWhitespace}).joined(separator:" ") }
for kind in 0..<5 {
    let pdf=PDFDocument(url:root.appendingPathComponent("report-\(kind).pdf"))!
    let data=try Data(contentsOf:root.appendingPathComponent("report-\(kind).json"))
    let json=try JSONSerialization.jsonObject(with:data) as! [String:Any]
    let pages=json["pages"] as! [[[String:Any]]]
    precondition(pdf.pageCount==pages.count)
    var cells=0
    for (index,items) in pages.enumerated() {
        let page=pdf.page(at:index)!
        let text=normalized(page.string ?? "")
        for item in items where item["kind"] as? String == "text" {
            let expected=normalized(item["text"] as! String)
            precondition(text.contains(expected),"Missing text in report \(kind) page \(index+1): \(expected)\n\(text)")
            cells += 1
        }
        precondition(abs(page.bounds(for:.mediaBox).width-842)<1)
        if index==0 {
            let image=page.thumbnail(of: NSSize(width:1263,height:893),for:.mediaBox)
            let bitmap=NSBitmapImageRep(data:image.tiffRepresentation!)!
            try bitmap.representation(using:.png,properties:[:])!.write(to:root.appendingPathComponent("report-\(kind).png"))
        }
    }
    print("PASS report \(kind): \(pages.count) pages, \(cells) text cells extracted; A4 landscape")
}
