import { Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";
import { TaskDock } from "./TaskDock";
import { ToastHost } from "./ToastHost";
import { TopBar } from "./TopBar";
import { AssetsPage } from "@/modules/asset/AssetsPage";
import { DashboardPage } from "@/modules/dashboard/DashboardPage";
import { AnimationPage } from "@/modules/animation/AnimationPage";
import { EditorPage } from "@/modules/editor/EditorPage";
import { StylesPage } from "@/modules/style/StylesPage";
import { GeneratePage } from "@/modules/generator/GeneratePage";
import { ProjectsPage } from "@/modules/project/ProjectsPage";
import { SettingsPage } from "@/modules/settings/SettingsPage";
import { useUiStore } from "@/stores/uiStore";

/** 视图路由：基于 uiStore 的轻量切换（单窗口工作台，无需路由库） */
function ActiveView() {
  const view = useUiStore((s) => s.view);
  switch (view) {
    case "dashboard":
      return <DashboardPage />;
    case "generate":
      return <GeneratePage />;
    case "assets":
      return <AssetsPage />;
    case "animation":
      return <AnimationPage />;
    case "editor":
      return <EditorPage />;
    case "projects":
      return <ProjectsPage />;
    case "styles":
      return <StylesPage />;
    case "settings":
      return <SettingsPage />;
    default:
      return <DashboardPage />;
  }
}

export function AppShell() {
  return (
    <div className="flex h-full flex-col">
      <TopBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <main className="min-w-0 flex-1 overflow-hidden">
          <ActiveView />
        </main>
      </div>
      <StatusBar />
      <TaskDock />
      <ToastHost />
    </div>
  );
}
