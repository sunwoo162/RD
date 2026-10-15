import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Activity, ArrowDownToLine, ArrowRight, Check, CheckCircle2, ChevronDown,
  Clock3, CloudDownload, Command, ExternalLink, Folder, FolderOpen, GitBranch,
  Github, HardDrive, LayoutGrid, LockKeyhole, Pause, Play, Plus, RefreshCw,
  Search, Settings2, ShieldCheck, ShieldAlert, Trash2, X
} from "lucide-react";

type Mode = "manual" | "auto" | "observe";
type Repo = { id: string; url: string; path: string; branch: string; mode: Mode; enabled: boolean };
type ActionName = "add" | "add_auto" | "root" | "interval" | "check" | "sync" | "pause" | "resume" | "remove" | "mode" | "autostart";
type Preferences = { default_root: string | null; interval_minutes: number };
type Notice = { tone: "success" | "error" | "info"; message: string };
type Filter = "all" | "auto" | "paused";

function getRepoName(repo: Repo) {
  return repo.url.replace(/\.git$/, "").split("/").filter(Boolean).pop() || "Repository";
}
function shortPath(path: string) {
  return path.replace(/\\/g, "/").replace(/^.*?\/(Documents|문서)\//i, "$1/");
}
function labelMode(mode: Mode) {
  return mode === "auto" ? "자동 업데이트" : mode === "manual" ? "보호 모드" : "관찰 모드";
}
function branchType(url: string) {
  return url.startsWith("git@") ? "SSH" : "HTTPS";
}

export default function App() {
  const [repos, setRepos] = useState<Repo[]>([]);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [showAdd, setShowAdd] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [url, setUrl] = useState("");
  const [storageRoot, setStorageRoot] = useState<string | null>(null);
  const [syncInterval, setSyncInterval] = useState(15);
  const [showSetup, setShowSetup] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [results, setResults] = useState<Record<string, string>>({});
  const [events, setEvents] = useState<{ action: string; message: string; at: string }[]>([]);
  const [autostart, setAutostart] = useState<string>("알 수 없음");

  async function reload() {
    const [all, preferences] = await Promise.all([
      invoke<Repo[]>("list_repositories"),
      invoke<Preferences>("get_preferences"),
    ]);
    setRepos(all);
    setStorageRoot(preferences.default_root ?? null);
    setSyncInterval(preferences.interval_minutes);
    return preferences;
  }
  function refresh() {
    reload().then(prefs => { if (!prefs.default_root) setShowSetup(true); }).catch(() => setNotice({ tone: "error", message: "RD Agent 설정을 읽지 못했습니다." }));
  }
  useEffect(() => {
    refresh();
  }, []);

  const shown = useMemo(() => repos.filter(repo => {
    const matches = (getRepoName(repo) + " " + repo.url + " " + repo.branch).toLowerCase().includes(query.toLowerCase());
    return matches && (filter === "all" || (filter === "auto" && repo.enabled && repo.mode === "auto") ||
      (filter === "paused" && !repo.enabled));
  }), [repos, query, filter]);

  async function run(action: ActionName, id?: string, extra?: { url?: string; folder?: string; branch?: string; mode?: string }) {
    setBusy(id || action);
    setNotice(null);
    try {
      const message = await invoke<string>("run_action", { action, id, ...extra });
      if (id) setResults(current => ({ ...current, [id]: message || "완료" }));
      setEvents(old => [{ action, message: message || "작업 완료", at: new Date().toLocaleTimeString("ko-KR") }, ...old].slice(0, 20));
      // A successful Git inspection may still report a protected/blocked update.
      const updated = action === "sync" && message.includes("커밋 반영 완료");
      const informational = action === "check" || (action === "sync" && !updated);
      setNotice({
        tone: informational ? "info" : "success",
        message: action === "add" || action === "add_auto" ? "프로젝트가 연결됐어요. 안전한 자동 업데이트가 활성화됐습니다."
          : action === "check" ? "변경 확인 결과를 확인하세요."
          : action === "sync" && !updated ? "동기화 상태를 확인하세요. 로컬 변경 사항이 보호됐을 수 있습니다."
          : "작업을 완료했습니다.",
      });
      await reload();
      return true;
    } catch (err) {
      setNotice({ tone: "error", message: String(err) });
      return false;
    } finally {
      setBusy(null);
    }
  }

  async function add() {
    if (!url.trim()) {
      setNotice({ tone: "error", message: "GitHub 주소를 입력해주세요." });
      return;
    }
    if (!storageRoot) {
      setShowAdd(false);
      setShowSetup(true);
      return;
    }
    const ok = await run("add_auto", undefined, { url: url.trim() });
    if (ok) { setShowAdd(false); setUrl(""); }
  }
  async function pickStorageRoot() {
    const selected = await open({ directory: true, multiple: false, title: "프로젝트를 보관할 상위 폴더 선택" });
    if (typeof selected !== "string") return;
    // The selected directory is preserved; RD creates its own child folder.
    const root = selected.replace(/[\\/]+$/, "") + "\\RepoDock";
    const ok = await run("root", undefined, { folder: root });
    if (ok) { setShowSetup(false); await reload(); }
  }
  function openAdd() {
    if (!storageRoot) setShowSetup(true);
    else setShowAdd(true);
  }
  async function openFolder(id: string) {
    try { await invoke<void>("open_repository_folder", { id }); }
    catch (error) { setNotice({ tone: "error", message: String(error) }); }
  }
  async function showStartup() {
    setShowSettings(true);
    try { setAutostart(await invoke<string>("run_action", { action: "autostart", mode: "status" })); }
    catch { setAutostart("자동 시작 설정을 읽지 못했습니다."); }
  }
  const autoCount = repos.filter(repo => repo.enabled && repo.mode === "auto").length;
  const pausedCount = repos.filter(repo => !repo.enabled).length;

  return <div className="shell">
    <aside className="sidebar">
      <div className="brand"><div className="brand-icon">RD<span>.</span></div><div className="brand-text">RepoDock<small>WORKSPACE MANAGER</small></div></div>
      <div className="nav-label">WORKSPACE</div>
      <nav className="nav">
        <button className={filter === "all" ? "nav-item active" : "nav-item"} onClick={() => setFilter("all")}><LayoutGrid size={17}/> Overview <span>{repos.length}</span></button>
        <button className={filter === "auto" ? "nav-item active" : "nav-item"} onClick={() => setFilter("auto")}><RefreshCw size={17}/> Auto sync <span>{autoCount}</span></button>
        <button className={filter === "paused" ? "nav-item active" : "nav-item"} onClick={() => setFilter("paused")}><Pause size={17}/> Paused <span>{pausedCount}</span></button>
      </nav>
      <div className="nav-label secondary-nav">SYSTEM</div>
      <nav className="nav"><button className="nav-item" onClick={showStartup}><Settings2 size={17}/> Preferences</button></nav>
      <div className="sidebar-bottom">
        <div className="protection"><ShieldCheck size={18} color="#89dec1"/><div><strong>Sync Guard enabled</strong><small>작업 중인 코드를 보호합니다</small></div></div>
        <div className="agent-label"><span className="pulse"/> Agent and UI are separate <span className="version">v1.1 Preview</span></div>
      </div>
    </aside>
    <main className="content">
      <header className="topbar">
        <div className="crumb">Workspace <span>/</span> <strong>Overview</strong></div>
        <div className="top-actions"><span className="local-only"><HardDrive size={14}/> LOCAL FIRST</span><button className="icon-button" title="프로젝트 새로고침" onClick={refresh}><RefreshCw size={17}/></button><div className="avatar">RD</div></div>
      </header>
      <div className="viewport">
        <div className="intro"><div><div className="eyebrow"><span className="eyebrow-line"/> REPOSITORY CONTROL CENTER</div><h1>Your code, <em>already here.</em></h1><p>GitHub와 로컬 프로젝트를 안전하게 연결하세요. 필요한 프로젝트만, 원하는 방식으로.</p></div><button className="primary" onClick={openAdd}><Plus size={18}/> 프로젝트 추가</button></div>
        <section className="metrics">
          <div className="metric"><div className="metric-caption"><Folder size={16}/> MANAGED REPOSITORIES</div><div className="metric-value">{repos.length.toString().padStart(2, "0")}</div><div className="metric-note">등록 제한 없음</div></div>
          <div className="metric"><div className="metric-caption"><RefreshCw size={16}/> AUTOMATIC SYNC</div><div className="metric-value">{autoCount.toString().padStart(2, "0")}</div><div className="metric-note good"><span className="green-dot"/> 사용자 승인한 저장소</div></div>
          <div className="metric"><div className="metric-caption"><ShieldAlert size={16}/> PROTECTED / PAUSED</div><div className="metric-value">{pausedCount.toString().padStart(2, "0")}</div><div className="metric-note">자동 업데이트 중지</div></div>
        </section>
        <section className="repository-section">
          <div className="section-header"><div><h2>Repositories <span>{shown.length}</span></h2><p>한 번 연결하면 자동으로 최신 상태를 유지해요. 로컬 수정 파일은 건드리지 않아요.</p></div><div className="search-wrap"><Search size={16}/><input aria-label="프로젝트 검색" placeholder="Search repositories..." value={query} onChange={e => setQuery(e.target.value)}/><kbd>⌕</kbd></div></div>
          {shown.length ? <div className="repo-list">{shown.map(repo => {
            const name = getRepoName(repo);
            const working = busy === repo.id;
            return <article className="repo-card" key={repo.id}>
              <div className="repo-head"><div className="repo-emblem"><Github size={23}/></div><div className="repo-identity"><div className="repo-name">{name}<span className="source-tag">{branchType(repo.url)}</span></div><div className="repo-url">{repo.url}</div></div><div className={repo.enabled ? "state enabled" : "state disabled"}><span className="state-dot"/>{repo.enabled ? labelMode(repo.mode) : "일시 정지"}</div></div>
              <div className="repo-details"><span><GitBranch size={15}/>{repo.branch}</span><span className="path" title={repo.path}><FolderOpen size={15}/>{shortPath(repo.path)}</span></div>
              {results[repo.id] && <div className="sync-result"><Activity size={15}/><pre>{results[repo.id]}</pre></div>}
              <div className="repo-footer"><div className="repo-operations">
                <button className="secondary-button" onClick={() => openFolder(repo.id)}><FolderOpen size={15}/> 폴더 열기</button>
                <button disabled={working || !!busy} className="secondary-button" onClick={() => run("check", repo.id)}><RefreshCw size={15} className={working ? "spin" : ""}/> 변경 확인</button>
                <button disabled={working || !!busy || !repo.enabled || repo.mode === "observe"} className="secondary-button" onClick={() => run("sync", repo.id)}><ArrowDownToLine size={15}/> 안전한 업데이트</button>
                <button disabled={!!busy} className="simple-button" onClick={() => run(repo.enabled ? "pause" : "resume", repo.id)}>{repo.enabled ? <Pause size={15}/> : <Play size={15}/>} {repo.enabled ? "중지" : "재개"}</button>
              </div><div className="right-actions">
                <label className="select-label">동기화 <ChevronDown size={13}/><select aria-label={name + " 동기화 모드"} disabled={!!busy} value={repo.mode} onChange={e => run("mode", repo.id, { mode: e.target.value })}><option value="manual">보호 모드</option><option value="auto">자동 업데이트</option><option value="observe">관찰 모드</option></select></label>
                <button className="delete-button" disabled={!!busy} title="관리 목록에서 제거" onClick={() => { if (window.confirm(name + "을 RD 관리 목록에서 제외할까요? 로컬 파일은 그대로 유지됩니다.")) run("remove", repo.id); }}><Trash2 size={15}/></button>
              </div></div>
            </article>;
          })}</div> : <div className="empty"><div className="empty-glyph"><CloudDownload size={30}/></div><h3>{repos.length ? "조건에 맞는 저장소가 없어요" : "첫 번째 프로젝트를 연결하세요"}</h3><p>{repos.length ? "검색어나 필터를 변경해 보세요." : "GitHub 주소 하나만 붙여넣으면 저장 폴더 생성부터 업데이트까지 RD가 관리해요."}</p><button className="secondary-button" onClick={() => { if (repos.length) { setQuery(""); setFilter("all"); } else setShowAdd(true); }}>{repos.length ? "전체 보기" : "저장소 연결하기"} <ArrowRight size={15}/></button></div>}
        </section>
        <div className="lower-grid"><section className="tip-panel"><div className="panel-icon"><LockKeyhole size={19}/></div><div><h3>Safe by design</h3><p>RD는 코드를 강제 초기화하거나 자동으로 Push하지 않습니다. 로컬 변경이 있거나 브랜치가 분기되면 업데이트를 중단합니다.</p></div></section><section className="history-panel"><div className="panel-title"><Clock3 size={16}/> Session activity</div>{events.length ? <div className="events">{events.slice(0, 3).map((e, i) => <div key={i}><span>{e.action}</span><time>{e.at}</time></div>)}</div> : <p>이 화면에서 실행한 작업이 여기에 표시됩니다.</p>}</section></div>
        <footer><span>RD · Your code, already here.</span><span>Portable · On-demand management UI</span></footer>
      </div>
    </main>
    {notice && <div role="status" className={"toast " + notice.tone}>{notice.tone === "success" ? <CheckCircle2 size={17}/> : <ShieldAlert size={17}/>}<span>{notice.message}</span><button onClick={() => setNotice(null)} title="닫기"><X size={15}/></button></div>}
    {showAdd && <div className="overlay" onMouseDown={e => { if (e.target === e.currentTarget) setShowAdd(false); }}>
      <div className="modal"><div className="modal-top"><div><div className="eyebrow">ONE-CLICK CONNECTION</div><h2>프로젝트 연결</h2></div><button className="icon-button" title="닫기" onClick={() => setShowAdd(false)}><X size={19}/></button></div>
        <p className="modal-lead">GitHub 주소만 입력하세요. 폴더와 브랜치는 RD가 자동으로 준비해요.</p>
        <label className="form-field">GitHub 프로젝트 주소<input autoFocus placeholder="https://github.com/owner/repo.git" value={url} onChange={e => setUrl(e.target.value)} onKeyDown={e => { if (e.key === "Enter" && !busy) void add(); }}/></label>
        <div className="workspace-hint"><FolderOpen size={17}/><span><strong>저장 위치</strong><small>{storageRoot}</small></span><button className="simple-button" onClick={() => { setShowAdd(false); void showStartup(); }}>변경</button></div>
        <div className="mode-information"><ShieldCheck size={19}/><span><strong>안전한 자동 업데이트 · {syncInterval}분 간격</strong><br/>작업 중인 파일은 수정하지 않고 새 커밋만 반영합니다.</span></div>
        <div className="modal-footer"><button className="secondary-button" onClick={() => setShowAdd(false)}>취소</button><button disabled={!!busy} className="primary" onClick={add}>{busy ? <RefreshCw className="spin" size={17}/> : <Plus size={17}/>} 프로젝트 연결</button></div>
      </div>
    </div>}
    {showSetup && <div className="overlay"><div className="modal small"><div className="modal-top"><div><div className="eyebrow">WELCOME TO REPODOCK</div><h2>프로젝트 저장 위치</h2></div><button className="icon-button" title="나중에" onClick={() => setShowSetup(false)}><X size={19}/></button></div><p className="modal-lead">최초 한 번만 선택하세요. 선택한 폴더에 <strong>RepoDock</strong> 폴더를 만들고 프로젝트를 자동으로 분류해요.</p><div className="mode-information"><FolderOpen size={19}/><span>예: 문서 선택 → 문서 / RepoDock / 프로젝트명<br/>기존 폴더를 덮어쓰거나 옮기지 않습니다.</span></div><div className="modal-footer"><button className="secondary-button" onClick={() => setShowSetup(false)}>나중에</button><button disabled={!!busy} className="primary" onClick={pickStorageRoot}><Folder size={17}/> 저장 폴더 선택</button></div></div></div>}
    {showSettings && <div className="overlay" onMouseDown={e => { if (e.target === e.currentTarget) setShowSettings(false); }}><div className="modal small"><div className="modal-top"><div><div className="eyebrow">PREFERENCES</div><h2>환경 설정</h2></div><button className="icon-button" onClick={() => setShowSettings(false)} title="닫기"><X size={19}/></button></div>
      <p className="modal-lead">한 번 설정하면 이후 프로젝트마다 반복할 필요 없어요.</p>
      <label className="form-field">기본 저장 폴더<div className="workspace-hint"><FolderOpen size={18}/><span><small>{storageRoot || "미설정"}</small></span><button className="secondary-button" disabled={!!busy} onClick={pickStorageRoot}>위치 변경</button></div></label>
      <label className="form-field">자동 업데이트 확인 주기<select className="setting-select" aria-label="자동 업데이트 확인 주기" disabled={!!busy} value={syncInterval} onChange={async e => { const value = e.target.value; await run("interval", undefined, { mode: value }); }}>
        <option value="5">5분</option><option value="15">15분 (권장)</option><option value="30">30분</option><option value="60">1시간</option>
      </select></label>
      <div className="startup-state"><Settings2 size={19}/><span>{autostart}</span></div>
      <p className="modal-lead">로그인 시 가벼운 Tray만 실행되고 관리 화면은 필요할 때 열립니다.</p>
      <div className="modal-footer"><button disabled={!!busy} className="secondary-button" onClick={async () => { await run("autostart", undefined, { mode: "disable" }); await showStartup(); }}>자동 실행 끄기</button><button disabled={!!busy} className="primary" onClick={async () => { await run("autostart", undefined, { mode: "enable" }); await showStartup(); }}><Check size={17}/> 자동 실행 켜기</button></div>
    </div></div>}
  </div>;
}
