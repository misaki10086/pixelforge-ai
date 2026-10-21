import { useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { cn } from "@/utils/cn";

interface AssetImageProps {
  path: string | null | undefined;
  alt: string;
  className?: string;
  pixelated?: boolean;
}

/** 展示本地资产图片：asset protocol + 棋盘格透明底 */
export function AssetImage({ path, alt, className, pixelated = true }: AssetImageProps) {
  const [failed, setFailed] = useState(false);
  if (!path || failed) {
    return (
      <div
        className={cn(
          "flex items-center justify-center border border-line bg-input text-inkfaint",
          className
        )}
        title="无预览图"
      >
        <span className="text-[10px]">N/A</span>
      </div>
    );
  }
  return (
    <div className={cn("checkerboard overflow-hidden border border-line", className)}>
      <img
        src={convertFileSrc(path)}
        alt={alt}
        draggable={false}
        onError={() => setFailed(true)}
        className={cn("h-full w-full object-contain", pixelated && "pixelated")}
      />
    </div>
  );
}
