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
let startup = false;

const sample: Repo = {
  id: "demo-1", url: "https://github.com/example/demo.git", path: "C:\\Projects\\demo",
  branch: "main", mode: "manual", enabled: true,
};

beforeEach(() => {
  repos = [];
  startup = false;
  vi.stubGlobal("confirm", vi.fn(() => true));
  state.open.mockReset();
  state.invoke.mockReset();
  state.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
    if (command === "list_repositories") return repos.map(r => ({ ...r }));
    if (command !== "run_action") throw new Error("Unexpected command: " + command);
    if (args?.action === "autostart") {
      if (args.mode === "enable") startup = true;
      if (args.mode === "disable") startup = false;
      return startup ? "로그인 자동 실행: 등록됨" : "로그인 자동 실행: 등록 안 됨";
    }
    const id = String(args?.id ?? "");
    const repo = repos.find(r => r.id === id);
    switch (args?.action) {
      case "add":
        repos.push({
          id: "added-1", url: String(args.url), path: String(args.folder),
          branch: String(args.branch || "main"), mode: "manual", enabled: true,
        });
        return "등록 완료";
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

describe("RepoDock Manager UI", () => {
  it("opens the add form and validates required fields without invoking clone", async () => {
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByText("첫 번째 프로젝트를 연결하세요")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /프로젝트 추가/ }));
    expect(screen.getByRole("heading", { name: "프로젝트 연결" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /등록 및 Clone/ }));
    expect(await screen.findByRole("status")).toHaveProperty("textContent", expect.stringContaining("모두 입력"));
    expect(state.invoke.mock.calls.filter(([command, args]) => command === "run_action" && args?.action === "add")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "취소" }));
    expect(screen.queryByRole("heading", { name: "프로젝트 연결" })).toBeNull();
  });

  it("clones through the form, checks and protects local edits, then unregisters without deleting work", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("첫 번째 프로젝트를 연결하세요");
    await user.click(screen.getByRole("button", { name: /프로젝트 추가/ }));
    await user.type(screen.getByPlaceholderText("https://github.com/owner/repo.git"), "https://github.com/example/added.git");
    await user.type(screen.getByPlaceholderText("C:\\Users\\...\\Documents\\Projects\\repo"), "C:\\Projects\\added");
    await user.click(screen.getByRole("button", { name: /등록 및 Clone/ }));
    expect(await screen.findByText("added", { selector: ".repo-name" })).toBeTruthy();
    const add = state.invoke.mock.calls.find(([command, args]) => command === "run_action" && args?.action === "add");
    expect(add?.[1]).toMatchObject({ action: "add", url: "https://github.com/example/added.git", folder: "C:\\Projects\\added", branch: "" });

    await user.click(screen.getByRole("button", { name: /변경 확인/ }));
    expect(await screen.findByText("최신 상태")).toBeTruthy();
    expect(screen.getByRole("status").textContent).toContain("변경 확인 결과");

    await user.click(screen.getByRole("button", { name: /안전한 업데이트/ }));
    await waitFor(() => expect(screen.getByRole("status").textContent).toContain("로컬 변경 사항이 보호"));
    expect(screen.getByRole("status").className).toContain("info");

    await user.click(screen.getByTitle("관리 목록에서 제거"));
    await waitFor(() => expect(screen.getByText("첫 번째 프로젝트를 연결하세요")).toBeTruthy());
    expect(window.confirm).toHaveBeenCalled();
    const remove = state.invoke.mock.calls.find(([command, args]) => command === "run_action" && args?.action === "remove");
    expect(remove?.[1]).toMatchObject({ id: "added-1" });
  });

  it("filters and updates mode and pause state through the controls", async () => {
    repos = [{ ...sample }];
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("demo", { selector: ".repo-name" });
    await user.selectOptions(screen.getByRole("combobox", { name: "demo 동기화 모드" }), "auto");
    await waitFor(() => expect(screen.getByText("자동 업데이트", { selector: ".state" })).toBeTruthy());
    await user.click(screen.getByRole("button", { name: /Auto sync/ }));
    expect(screen.getByText("demo", { selector: ".repo-name" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "중지" }));
    // Pausing removes the repository from the auto-sync filtered list.
    await waitFor(() => expect(screen.getByText("조건에 맞는 저장소가 없어요")).toBeTruthy());
    await user.click(screen.getByRole("button", { name: /Paused/ }));
    expect(await screen.findByText("일시 정지", { selector: ".state" })).toBeTruthy();
    expect(screen.getByText("demo", { selector: ".repo-name" })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "재개" }));
    await waitFor(() => expect(screen.getByText("조건에 맞는 저장소가 없어요")).toBeTruthy());
    await user.click(screen.getByRole("button", { name: /Overview/ }));
    await user.type(screen.getByRole("textbox", { name: "프로젝트 검색" }), "no-match");
    expect(screen.getByText("조건에 맞는 저장소가 없어요")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: /전체 보기/ }));
    expect(screen.getByText("demo", { selector: ".repo-name" })).toBeTruthy();
  });

  it("reads startup state and only enables autostart after an explicit click", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("첫 번째 프로젝트를 연결하세요");
    await user.click(screen.getByRole("button", { name: /Preferences/ }));
    expect(await screen.findByText("로그인 자동 실행: 등록 안 됨")).toBeTruthy();
    expect(startup).toBe(false);
    await user.click(screen.getByRole("button", { name: /자동 실행 켜기/ }));
    await waitFor(() => expect(screen.getByText("로그인 자동 실행: 등록됨")).toBeTruthy());
    expect(startup).toBe(true);
    await user.click(screen.getByRole("button", { name: "자동 실행 끄기" }));
    await waitFor(() => expect(screen.getByText("로그인 자동 실행: 등록 안 됨")).toBeTruthy());
    expect(startup).toBe(false);
  });
});
