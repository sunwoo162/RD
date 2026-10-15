// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";

type Repo = { id: string; url: string; path: string; branch: string; mode: "manual" | "auto" | "observe"; enabled: boolean };
const state = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: state.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: state.open }));

let repos: Repo[] = [];
let autostart = false;
let storageRoot: string | null = "C:\\Projects\\RepoDock";
let interval = 15;
const example: Repo = { id: "demo-1", url: "https://github.com/example/demo.git", path: "C:\\Projects\\demo", branch: "main", mode: "manual", enabled: true };

beforeEach(() => {
  repos = [];
  autostart = false;
  storageRoot = "C:\\Projects\\RepoDock";
  interval = 15;
  state.open.mockReset();
  state.invoke.mockReset();
  vi.stubGlobal("confirm", vi.fn(() => true));
  state.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
    if (command === "list_repositories") return repos.map(r => ({ ...r }));
    if (command === "get_preferences") return { default_root: storageRoot, interval_minutes: interval };
    if (command === "open_repository_folder") return undefined;
    if (command !== "run_action") throw new Error("Unexpected command: " + command);
    if (args?.action === "autostart") {
      if (args.mode === "enable") autostart = true;
      if (args.mode === "disable") autostart = false;
      return autostart ? "로그인 자동 실행: 등록됨" : "로그인 자동 실행: 등록 안 됨";
    }
    const id = String(args?.id ?? "");
    const repo = repos.find(r => r.id === id);
    switch (args?.action) {
      case "root":
        storageRoot = String(args.folder);
        return "저장 위치 변경됨";
      case "interval":
        interval = Number(args.mode);
        return "확인 주기 변경";
      case "add_auto":
        if (!storageRoot) throw new Error("먼저 저장 폴더를 선택하세요");
        repos.push({ id: "added-1", url: String(args.url), path: storageRoot + "\\added", branch: "main", mode: "auto", enabled: true });
        return "프로젝트 연결 완료";
      case "check": return "최신 상태";
      case "sync": return "로컬 변경 사항 보호: 자동 업데이트 중단";
      case "pause": if (!repo) throw new Error("Unknown repo"); repo.enabled = false; return "동기화 중지";
      case "resume": if (!repo) throw new Error("Unknown repo"); repo.enabled = true; return "동기화 재개";
      case "mode": if (!repo) throw new Error("Unknown repo"); repo.mode = args.mode as Repo["mode"]; return "모드 변경";
      case "remove": repos = repos.filter(r => r.id !== id); return "관리 목록에서 해제";
      default: throw new Error("Unexpected action: " + args?.action);
    }
  });
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe("RepoDock v1.1 simplified UI", () => {
  it("asks for workspace only once and remembers the selected Documents folder", async () => {
    storageRoot = null;
    state.open.mockResolvedValue("C:\\Users\\tester\\Documents");
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByRole("heading", { name: "프로젝트 저장 위치" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "저장 폴더 선택" }));
    await waitFor(() => expect(storageRoot).toBe("C:\\Users\\tester\\Documents\\RepoDock"));
    expect(screen.queryByRole("heading", { name: "프로젝트 저장 위치" })).toBeNull();
    expect(state.invoke.mock.calls.some(([command, args]) =>
      command === "run_action" && args.action === "root" && args.folder === storageRoot)).toBe(true);
  });

  it("requires only a GitHub URL for auto-clone; missing URL does not trigger clone", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("첫 번째 프로젝트를 연결하세요");
    await user.click(screen.getByRole("button", { name: /프로젝트 추가/ }));
    expect(screen.getByRole("heading", { name: "프로젝트 연결" })).toBeTruthy();
    expect(screen.queryByPlaceholderText("C:\\Users\\...\\Documents\\Projects\\repo")).toBeNull();
    await user.click(screen.getByRole("button", { name: "프로젝트 연결" }));
    expect(await screen.findByRole("status")).toHaveProperty("textContent", expect.stringContaining("GitHub 주소"));
    expect(state.invoke.mock.calls.filter(([command, args]) => command === "run_action" && args?.action === "add_auto")).toHaveLength(0);
    await user.type(screen.getByPlaceholderText("https://github.com/owner/repo.git"), "https://github.com/example/added.git");
    await user.click(screen.getByRole("button", { name: "프로젝트 연결" }));
    expect(await screen.findByText("added", { selector: ".repo-name" })).toBeTruthy();
    const call = state.invoke.mock.calls.find(([command, args]) => command === "run_action" && args?.action === "add_auto");
    expect(call?.[1]).toMatchObject({ url: "https://github.com/example/added.git" });
    expect(screen.getByText("자동 업데이트", { selector: ".state" })).toBeTruthy();
  });

  it("handles check, protected sync, pause, resume and removing a managed repo", async () => {
    repos = [{ ...example }];
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("demo", { selector: ".repo-name" });
    await user.click(screen.getByRole("button", { name: /폴더 열기/ }));
    expect(state.invoke).toHaveBeenCalledWith("open_repository_folder", { id: "demo-1" });
    await user.click(screen.getByRole("button", { name: /변경 확인/ }));
    expect(await screen.findByText("최신 상태")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /안전한 업데이트/ }));
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("로컬 변경 사항이 보호"));
    await user.selectOptions(screen.getByRole("combobox", { name: "demo 동기화 모드" }), "auto");
    await waitFor(() => expect(screen.getByText("자동 업데이트", { selector: ".state" })).toBeTruthy());
    await user.click(screen.getByRole("button", { name: /Auto sync/ }));
    await user.click(screen.getByRole("button", { name: "중지" }));
    await waitFor(() => expect(screen.getByText("조건에 맞는 저장소가 없어요")).toBeTruthy());
    await user.click(screen.getByRole("button", { name: /Paused/ }));
    expect(await screen.findByText("일시 정지", { selector: ".state" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "재개" }));
    await waitFor(() => expect(screen.getByText("조건에 맞는 저장소가 없어요")).toBeTruthy());
    await user.click(screen.getByRole("button", { name: /Overview/ }));
    await user.click(screen.getByTitle("관리 목록에서 제거"));
    await waitFor(() => expect(screen.getByText("첫 번째 프로젝트를 연결하세요")).toBeTruthy());
    expect(window.confirm).toHaveBeenCalled();
  });

  it("changes default folder and 15-minute interval, without enabling login startup by default", async () => {
    const user = userEvent.setup();
    state.open.mockResolvedValue("D:\\OpenSource");
    render(<App />);
    await screen.findByText("첫 번째 프로젝트를 연결하세요");
    await user.click(screen.getByRole("button", { name: /Preferences/ }));
    expect(await screen.findByText("로그인 자동 실행: 등록 안 됨")).toBeTruthy();
    expect(autostart).toBe(false);
    await user.click(screen.getByText("위치 변경"));
    await waitFor(() => expect(storageRoot).toBe("D:\\OpenSource\\RepoDock"));
    await user.selectOptions(screen.getByRole("combobox", { name: "자동 업데이트 확인 주기" }), "30");
    await waitFor(() => expect(interval).toBe(30));
    await user.click(screen.getByRole("button", { name: /자동 실행 켜기/ }));
    await waitFor(() => expect(autostart).toBe(true));
    await user.click(screen.getByRole("button", { name: "자동 실행 끄기" }));
    await waitFor(() => expect(autostart).toBe(false));
  });
});
