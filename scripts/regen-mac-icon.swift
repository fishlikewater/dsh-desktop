#!/usr/bin/env swift
// macOS 应用图标再生成器（Dock 图标大小修复）
//
// 背景：src-tauri/icons/icon-source.png 是 1024x1024 满铺方图（内容包围盒 =
// 整个画布、四边无透明边距）。macOS 的 Dock / 启动台图标按 1024 画布 +
// 内容约 824x824 居中（四周 ~100px 透明边距）的网格规范绘制，满铺图标会
// 显得比其他应用图标"大一号"。
//
// 本脚本把 icon-source.png 等比缩放进 1024 画布中央 824x824，生成带透明
// 边距的 macOS 图标并重建 icons/icon.icns（Windows 图标不受影响，
// Windows 图标本身就是满铺规范）。
//
// 用法（在仓库根目录）：
//   swift scripts/regen-mac-icon.swift
//
// 依赖：macOS 自带 swift/sips/iconutil，无需第三方库。
// 之后如有设计调整（改图形/圆角），更新 icons/icon-source.png 后重跑即可。

import Foundation
import CoreGraphics
import ImageIO

// 仓库根目录：本脚本位于 <root>/scripts/
let repoRoot = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
let srcPath = repoRoot.appendingPathComponent("src-tauri/icons/icon-source.png")
let outPngPath = repoRoot.appendingPathComponent("src-tauri/icons/icon-mac-1024.png")
let outIcnsPath = repoRoot.appendingPathComponent("src-tauri/icons/icon.icns")

let canvas = 1024
let content = 824   // Apple 图标网格：内容区 824x824 居中
let offset = (canvas - content) / 2

func fail(_ msg: String) -> Never {
    FileHandle.standardError.write(("FAIL: " + msg + "\n").data(using: .utf8)!)
    exit(1)
}

// 1) 加载源图
guard let srcCG = CGImageSourceCreateImageAtIndex(
    CGImageSourceCreateWithURL(srcPath as CFURL, nil)!,
    0, nil
) else { fail("无法加载 \(srcPath.path)") }

// 2) 绘制到 1024x1024 透明画布中央 824x824
guard let ctx = CGContext(
    data: nil, width: canvas, height: canvas, bitsPerComponent: 8,
    bytesPerRow: canvas * 4, space: CGColorSpaceCreateDeviceRGB(),
    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
) else { fail("无法创建绘图上下文") }
let rect = CGRect(x: offset, y: offset, width: content, height: content)
ctx.interpolationQuality = .high
ctx.draw(srcCG, in: rect)
guard let padded = ctx.makeImage() else { fail("绘制失败") }

// 3) 输出 1024 带边距 PNG（供核验 / 复用）
guard let dest = CGImageDestinationCreateWithURL(
    outPngPath as CFURL, "public.png" as CFString, 1, nil
) else { fail("无法创建输出 PNG") }
CGImageDestinationAddImage(dest, padded, nil)
guard CGImageDestinationFinalize(dest) else { fail("写出 \(outPngPath.lastPathComponent) 失败") }
print("OK: 已生成 \(outPngPath.relativePath)（1024x1024，内容 824x824 居中）")

// 4) 生成 iconset（sips 缩放）并打包 icns
let fm = FileManager.default
let iconset = FileManager.default.temporaryDirectory
    .appendingPathComponent("dsh-mac-icon-\(UUID().uuidString).iconset")
try? fm.createDirectory(at: iconset, withIntermediateDirectories: true)
defer { try? fm.removeItem(at: iconset) }

let sizes: [(name: String, px: Int)] = [
    ("icon_16x16.png", 16), ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32), ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128), ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256), ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512), ("icon_512x512@2x.png", 1024),
]
for (name, px) in sizes {
    let out = iconset.appendingPathComponent(name)
    let p = Process()
    p.executableURL = URL(fileURLWithPath: "/usr/bin/sips")
    p.arguments = ["-z", "\(px)", "\(px)", outPngPath.path, "--out", out.path]
    try? p.run(); p.waitUntilExit()
    guard p.terminationStatus == 0, fm.fileExists(atPath: out.path) else {
        fail("sips 生成 \(name) 失败")
    }
}

let iconutil = Process()
iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
iconutil.arguments = ["-c", "icns", iconset.path, "-o", outIcnsPath.path]
try? iconutil.run(); iconutil.waitUntilExit()
guard iconutil.terminationStatus == 0 else { fail("iconutil 打包 \(outIcnsPath.lastPathComponent) 失败") }
print("OK: 已重建 \(outIcnsPath.relativePath)")
