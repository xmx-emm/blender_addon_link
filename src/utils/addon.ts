import {appDataDir, dirname, join} from "@tauri-apps/api/path";

export function parseVersion(v: string): [number, number] {
    const parts = v.split(".");
    return [parseInt(parts[0] ?? "0", 10) || 0, parseInt(parts[1] ?? "0", 10) || 0];
}

export function versionGte(v: string, major: number, minor: number): boolean {
    const [a, b] = parseVersion(v);
    return a > major || (a === major && b >= minor);
}

export function compareVersion(a: string, b: string): number {
    const [a1, a2] = parseVersion(a);
    const [b1, b2] = parseVersion(b);
    return a1 - b1 || a2 - b2;
}

/** 4.2 起才有 Extensions 系统 */
export function addonIsExtension(version: string): boolean {
    return versionGte(version, 4, 2);
}

/** Blender 用户目录根：%APPDATA%\Blender Foundation\Blender\<版本> */
export async function getBlenderVersionFolder(version: string): Promise<string> {
    return await join(
        await dirname(await dirname(await appDataDir())),
        "Roaming",
        "Blender Foundation",
        "Blender",
        version,
    );
}

/** 插件安装目录：扩展走 extensions\user_default，传统走 scripts\addons */
export async function getAddonLinkFolder(version: string, is_extension: boolean): Promise<string> {
    const base = await getBlenderVersionFolder(version);
    if (is_extension && addonIsExtension(version)) {
        return await join(base, "extensions", "user_default");
    }
    return await join(base, "scripts", "addons");
}

/**
 * 主安装路径 +（4.2+）另一形态路径，用于双重安装检测。
 * primary 按插件形态；alternate 为另一位置（addons ↔ extensions）。
 */
export async function getAddonInstallLocations(
    version: string,
    addonName: string,
    is_extension: boolean,
): Promise<{primary: string; alternate: string | null}> {
    const primaryFolder = await getAddonLinkFolder(version, is_extension);
    const primary = await join(primaryFolder, addonName);
    if (!addonIsExtension(version)) {
        return {primary, alternate: null};
    }
    const altFolder = await getAddonLinkFolder(version, !is_extension);
    const alternate = await join(altFolder, addonName);
    if (alternate.toLowerCase() === primary.toLowerCase()) {
        return {primary, alternate: null};
    }
    return {primary, alternate};
}

export function formatBytes(n: number): string {
    if (!Number.isFinite(n) || n < 0) return "-";
    if (n < 1024) return `${n} B`;
    const units = ["KB", "MB", "GB", "TB"];
    let v = n / 1024;
    let i = 0;
    while (v >= 1024 && i < units.length - 1) {
        v /= 1024;
        i++;
    }
    return `${v >= 100 ? v.toFixed(0) : v.toFixed(1)} ${units[i]}`;
}

export function formatSeconds(s: number): string {
    if (!Number.isFinite(s)) return "-";
    if (s < 60) return `${s.toFixed(s < 10 ? 2 : 1)} 秒`;
    const m = Math.floor(s / 60);
    const sec = Math.round(s % 60);
    if (m < 60) return `${m} 分 ${sec} 秒`;
    const h = Math.floor(m / 60);
    return `${h} 小时 ${m % 60} 分`;
}

/** RGBA 原始像素（自下而上）转 data URL，供 <img> 显示 */
export function thumbToDataUrl(width: number, height: number, rgbaBase64: string): string | null {
    try {
        const bin = atob(rgbaBase64);
        const src = new Uint8ClampedArray(bin.length);
        for (let i = 0; i < bin.length; i++) src[i] = bin.charCodeAt(i);
        const flipped = new Uint8ClampedArray(src.length);
        const row = width * 4;
        for (let y = 0; y < height; y++) {
            flipped.set(src.subarray(y * row, (y + 1) * row), (height - 1 - y) * row);
        }
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext("2d");
        if (!ctx) return null;
        ctx.putImageData(new ImageData(flipped, width, height), 0, 0);
        return canvas.toDataURL("image/png");
    } catch {
        return null;
    }
}

/** 分类颜色（图表与图例共用） */
export function categoryColor(code: string): string {
    const map: Record<string, string> = {
        IM: "#E87D0D", // Blender 橙留给通常最大的图像
        ME: "#26A69A",
        OB: "#42A5F5",
        MA: "#AB47BC",
        NT: "#66BB6A",
        SC: "#FFCA28",
        AC: "#EC407A",
        TE: "#8D6E63",
        GR: "#78909C",
        DATA: "#9E9E9E",
        DNA1: "#607D8B",
        SO: "#5C6BC0",
        VO: "#00ACC1",
        PT: "#9CCC65",
        CU: "#FF7043",
        GP: "#D4E157",
        GD: "#D4E157",
        AR: "#7E57C2",
        VF: "#BDBDBD",
    };
    if (map[code]) return map[code];
    // 其他类别按 code 稳定取色
    const palette = ["#4DB6AC", "#7986CB", "#F06292", "#A1887F", "#90A4AE", "#4FC3F7", "#AED581", "#FFB74D"];
    let h = 0;
    for (const c of code) h = (h * 31 + c.charCodeAt(0)) | 0;
    return palette[Math.abs(h) % palette.length];
}
