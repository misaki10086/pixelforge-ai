import { ComingSoon } from "@/components/ui/Panel";

export function EditorPage() {
  return (
    <ComingSoon
      feature="像素编辑器"
      items={[
        "工具集：选择 / 移动 / 画笔 / 橡皮 / 铅笔 / 填充 / 吸管 / 缩放 / 平移（第二、三阶段）",
        "像素网格与棋盘格显示，像素级缩放平移旋转（第三阶段）",
        "AI 编辑：「保持服装不变，把武器改成火焰剑」式局部修改（第三阶段）",
        "画笔 / 擦除 / 选区蒙版，配合支持蒙版的 Provider（第三阶段）",
        "AI 修改生成新版本，完整保留版本历史（第三阶段）",
        "图像处理管线：像素化 / 调色板缩减 / 抖动 / 描边（第二阶段）",
      ]}
    />
  );
}
