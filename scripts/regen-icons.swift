#!/usr/bin/env swift
// 应用图标全量再生成器（DeepSeek 黑鲸鱼设计）
//
// 设计源：src-tauri/icons/app-icon.svg —— 浅灰圆角矩形底（线性渐变
// #F5F6F8→#DCDEE2，rx=220）+ 黑色鲸鱼（#141414，居中）。
// 此前安装包图标是按 src-tauri/icons/icon-source.png（深蓝 "D" 设计）
// 生成的，黑鲸鱼只用于壳页标题栏；本脚本把图标源切回鲸鱼设计。
//
// 流程：
// 1. 解析 app-icon.svg 并栅格化 1024x1024 → 覆盖 icon-source.png
//    （此后 `tauri icon` 等任何从 icon-source.png 再生成的流程都基于鲸鱼）
// 2. 生成各平台 PNG：icon.png(512)/32x32/64x64/128x128/128x128@2x/
//    Square30..310Logo/StoreLogo（尺寸与原先一致）
// 3. macOS：Apple 图标网格（内容 824x824 居中 + 四周 100px 透明边距）
//    → icon-mac-1024.png → iconutil 打包 icon.icns
// 4. Windows：sips 从 256px 生成 icon.ico
//
// 用法（仓库根目录）：swift scripts/regen-icons.swift
// 依赖：macOS 自带 swift / iconutil / sips，无第三方库。

import Foundation
import CoreGraphics
import ImageIO

let repoRoot = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
let iconsDir = repoRoot.appendingPathComponent("src-tauri/icons")
let svgPath = iconsDir.appendingPathComponent("app-icon.svg")
let sourcePng = iconsDir.appendingPathComponent("icon-source.png")
let macPng = iconsDir.appendingPathComponent("icon-mac-1024.png")
let icnsPath = iconsDir.appendingPathComponent("icon.icns")
let canvas = 1024

func fail(_ msg: String) -> Never {
    FileHandle.standardError.write(("FAIL: " + msg + "\n").data(using: .utf8)!)
    exit(1)
}

extension String {
    /// 返回所有匹配的第 1 个捕获组
    func matches(pattern: String) -> [String] {
        guard let re = try? NSRegularExpression(pattern: pattern) else {
            fail("非法正则: \(pattern)")
        }
        return re.matches(in: self, range: NSRange(startIndex..., in: self)).map {
            (self as NSString).substring(with: $0.range(at: 1))
        }
    }
    /// 返回首个匹配的第 1 个捕获组
    func capture(_ pattern: String) -> String? {
        guard let re = try? NSRegularExpression(pattern: pattern) else {
            fail("非法正则: \(pattern)")
        }
        guard let m = re.firstMatch(in: self, range: NSRange(startIndex..., in: self)) else { return nil }
        return (self as NSString).substring(with: m.range(at: 1))
    }
    /// 返回所有匹配的完整文本（无捕获组时用）
    func values(pattern: String) -> [String] {
        guard let re = try? NSRegularExpression(pattern: pattern) else {
            fail("非法正则: \(pattern)")
        }
        return re.matches(in: self, range: NSRange(startIndex..., in: self)).map {
            (self as NSString).substring(with: $0.range)
        }
    }
}

// ---------- 1) 解析 SVG ----------
let svg = try! String(contentsOf: svgPath, encoding: .utf8)

let rx = Double(svg.capture(#"rx="([\d.]+)""#) ?? "220")!
let stopColors = svg.matches(pattern: ##"stop-color="#([0-9a-fA-F]{6})""##)
let transform = svg.capture(#"transform="([^"]+)""#) ?? ""
let pathData = svg.capture(##"<path[^>]*\sd="([^"]+)""##) ?? ""

func hexToRGB(_ s: String) -> (CGFloat, CGFloat, CGFloat) {
    let hex = s.hasPrefix("#") ? String(s.dropFirst()) : s
    let v = UInt32(hex, radix: 16) ?? 0
    if hex.count == 6 {
        return (CGFloat((v >> 16) & 0xff) / 255, CGFloat((v >> 8) & 0xff) / 255, CGFloat(v & 0xff) / 255)
    }
    return (0.95, 0.96, 0.97)
}

guard stopColors.count >= 2 else { fail("app-icon.svg 中未找到渐变 stop") }
let (r1, g1, b1) = hexToRGB(stopColors[0])
let (r2, g2, b2) = hexToRGB(stopColors[1])

// 解析 path 命令（仅 M/C/Z，按正则扫描为字母/数字事件流）
func parsePath(_ d: String) -> CGMutablePath {
    let path = CGMutablePath()
    let pattern = try! NSRegularExpression(pattern: "([MCZ])|([-+]?[0-9]*\\.?[0-9]+(?:[eE][-+]?[0-9]+)?)")
    var events: [String] = []
    for m in pattern.matches(in: d, range: NSRange(d.startIndex..., in: d)) {
        events.append((d as NSString).substring(with: m.range))
    }
    var idx = 0
    while idx < events.count {
        let e = events[idx]
        if e == "M" {
            idx += 1
            var args: [Double] = []
            while idx < events.count, Double(events[idx]) != nil {
                args.append(Double(events[idx])!); idx += 1
            }
            guard args.count >= 2 else { fail("M 参数不足") }
            path.move(to: CGPoint(x: args[0], y: args[1]))
            for k in stride(from: 2, to: args.count - 1, by: 2) {
                path.addLine(to: CGPoint(x: args[k], y: args[k + 1]))
            }
        } else if e == "C" {
            idx += 1
            var args: [Double] = []
            while idx < events.count, Double(events[idx]) != nil {
                args.append(Double(events[idx])!); idx += 1
            }
            guard args.count >= 6 else { fail("C 参数不足") }
            path.addCurve(to: CGPoint(x: args[4], y: args[5]),
                          control1: CGPoint(x: args[0], y: args[1]),
                          control2: CGPoint(x: args[2], y: args[3]))
        } else if e == "Z" {
            idx += 1
            path.closeSubpath()
        } else {
            idx += 1
        }
    }
    return path
}

// ---------- 2) 渲染 1024x1024（SVG 坐标 y 向下，先做翻转） ----------
func makeContext(_ px: Int) -> CGContext {
    CGContext(data: nil, width: px, height: px, bitsPerComponent: 8, bytesPerRow: px * 4,
              space: CGColorSpaceCreateDeviceRGB(),
              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
}

let ctx = makeContext(canvas)
ctx.saveGState()
ctx.translateBy(x: 0, y: CGFloat(canvas)); ctx.scaleBy(x: 1, y: -1)

let rr = CGPath(roundedRect: CGRect(x: 0, y: 0, width: canvas, height: canvas),
                cornerWidth: 220, cornerHeight: 220, transform: nil)
ctx.addPath(rr); ctx.clip()

let grad = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                      colors: [CGColor(red: r1, green: g1, blue: b1, alpha: 1),
                               CGColor(red: r2, green: g2, blue: b2, alpha: 1)] as CFArray,
                      locations: [0, 1])!
ctx.drawLinearGradient(grad, start: CGPoint(x: 0, y: 0), end: CGPoint(x: 1024, y: 1024), options: [])

// 应用 <g transform="translate(512 512) scale(15.5) translate(-25 -25)">
for part in transform.split(separator: ")", omittingEmptySubsequences: false) {
    let t = part.trimmingCharacters(in: CharacterSet(charactersIn: " ()"))
    if t.hasPrefix("translate") {
        let values = t.values(pattern: #"[-+]?[0-9]*\.?[0-9]+"#).compactMap { Double($0) }
        ctx.translateBy(x: CGFloat(values.first ?? 0), y: CGFloat(values.count > 1 ? values[1] : 0))
    } else if t.hasPrefix("scale"), let s = t.values(pattern: #"[-+]?[0-9]*\.?[0-9]+"#).first.flatMap({ Double($0) }) {
        ctx.scaleBy(x: CGFloat(s), y: CGFloat(s))
    }
}
ctx.setFillColor(CGColor(red: 20 / 255, green: 20 / 255, blue: 20 / 255, alpha: 1)) // #141414
ctx.addPath(parsePath(pathData))
ctx.fillPath()   // 默认 nonzero（与 SVG fill-rule 一致）
ctx.restoreGState()

guard let base = ctx.makeImage() else { fail("渲染失败") }

func writePNG(_ img: CGImage, to url: URL) {
    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else {
        fail("无法创建 \(url.lastPathComponent)")
    }
    CGImageDestinationAddImage(dest, img, nil)
    guard CGImageDestinationFinalize(dest) else { fail("写出 \(url.lastPathComponent) 失败") }
}

func scaled(_ px: Int) -> CGImage {
    let c = makeContext(px)
    c.interpolationQuality = .high
    c.draw(base, in: CGRect(x: 0, y: 0, width: px, height: px))
    return c.makeImage()!
}

func scaledFrom(_ img: CGImage, _ px: Int) -> CGImage {
    let c = makeContext(px)
    c.interpolationQuality = .high
    c.draw(img, in: CGRect(x: 0, y: 0, width: px, height: px))
    return c.makeImage()!
}

writePNG(base, to: sourcePng)
print("OK: \(sourcePng.lastPathComponent)（鲸鱼设计 1024x1024）")

// ---------- 3) 平台 PNG（尺寸与仓库原有一致） ----------
let sizes: [(String, Int)] = [
    ("icon.png", 512),
    ("32x32.png", 32), ("64x64.png", 64),
    ("128x128.png", 128), ("128x128@2x.png", 256),
    ("Square30x30Logo.png", 30), ("Square44x44Logo.png", 44),
    ("Square71x71Logo.png", 71), ("Square89x89Logo.png", 89),
    ("Square107x107Logo.png", 107), ("Square142x142Logo.png", 142),
    ("Square150x150Logo.png", 150), ("Square284x284Logo.png", 284),
    ("Square310x310Logo.png", 310), ("StoreLogo.png", 50),
]
for (name, px) in sizes {
    writePNG(scaled(px), to: iconsDir.appendingPathComponent(name))
}
print("OK: 平台 PNG 已生成（\(sizes.count) 个）")

// ---------- 4) macOS：HIG 边距 + iconset + icns ----------
let content = 824
let offset = (canvas - content) / 2
let paddedCtx = makeContext(canvas)
paddedCtx.interpolationQuality = .high
paddedCtx.draw(base, in: CGRect(x: offset, y: offset, width: content, height: content))
guard let padded = paddedCtx.makeImage() else { fail("生成 macOS 带边距图标失败") }
writePNG(padded, to: macPng)
print("OK: \(macPng.lastPathComponent)（824x824 居中 + 100px 透明边距）")

let fm = FileManager.default
let iconset = fm.temporaryDirectory.appendingPathComponent("dsh-icons-\(UUID().uuidString).iconset")
try? fm.createDirectory(at: iconset, withIntermediateDirectories: true)
defer { try? fm.removeItem(at: iconset) }
// 用「已加 HIG 边距」的 padded 图生成 iconset（此前误用全幅 base，icns 再次满铺）
for (name, px) in [("icon_16x16.png", 16), ("icon_16x16@2x.png", 32),
                   ("icon_32x32.png", 32), ("icon_32x32@2x.png", 64),
                   ("icon_128x128.png", 128), ("icon_128x128@2x.png", 256),
                   ("icon_256x256.png", 256), ("icon_256x256@2x.png", 512),
                   ("icon_512x512.png", 512), ("icon_512x512@2x.png", 1024)] {
    writePNG(scaledFrom(padded, px), to: iconset.appendingPathComponent(name))
}
let iconutil = Process()
iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
iconutil.arguments = ["-c", "icns", iconset.path, "-o", icnsPath.path]
try? iconutil.run(); iconutil.waitUntilExit()
guard iconutil.terminationStatus == 0 else { fail("iconutil 打包 icns 失败") }
print("OK: \(icnsPath.lastPathComponent)")

// ---------- 5) Windows ICO（sips，从 256px 生成） ----------
let tmp256 = fm.temporaryDirectory.appendingPathComponent("dsh-ico-256-\(UUID().uuidString).png")
writePNG(scaled(256), to: tmp256)
let sips = Process()
sips.executableURL = URL(fileURLWithPath: "/usr/bin/sips")
sips.arguments = ["-s", "format", "ico", tmp256.path, "--out", iconsDir.appendingPathComponent("icon.ico").path]
try? sips.run(); sips.waitUntilExit()
try? fm.removeItem(at: tmp256)
guard sips.terminationStatus == 0 else { fail("sips 生成 icon.ico 失败") }
print("OK: icon.ico")
print("DONE")
