import { ComingSoon } from "@/components/ui/Panel";

export function StylesPage() {
  return (
    <ComingSoon
      feature="风格库"
      items={[
        "风格档案：分辨率 / 调色板 / 描边 / 像素密度 / 光照 / 饱和度 / 对比度（第三阶段）",
        "新建 / 编辑 / 复制 / 删除风格配置（第三阶段）",
        "风格分析器：从已有资产提取主色、调色板、描边与像素密度（第四阶段）",
        "生成资产时绑定风格档案，保证全项目视觉一致（第四阶段）",
        "地块 / 特效 / 场景专用生成器（第三阶段）",
      ]}
    />
  );
}
