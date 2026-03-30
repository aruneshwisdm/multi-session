# How jc Works — A Plain English Guide

## The Problem

You're a developer using Claude Code. You have 3 projects. Each project has tasks you want Claude to work on in parallel. But you only have one pair of eyes.

Without jc, you're constantly switching between terminals:
- "Did Claude finish in terminal 2?"
- "Oh no, Claude asked for permission 10 minutes ago in terminal 4 and I didn't notice"
- "Which task was I supposed to check next?"

jc fixes this. It's one window that manages everything and **tells you what needs your attention right now**.

---

## Core Concepts

### Projects

A project is a directory on your machine — your codebase.

```
jc ~/work/backend      # registers backend as a project
jc ~/work/frontend     # registers frontend as a project
```

jc keeps all your projects in one window. Switch between them with **Ctrl+O**.

### Sessions

A session is one Claude Code conversation. Each project can have multiple sessions running in parallel.

```
Project: backend
├── Session: "fix-auth"          ← Claude is working on authentication
├── Session: "write-tests"       ← Claude is writing tests
└── Session: "update-docs"       ← Claude is updating documentation
```

Create a new session with **Ctrl+T**. Switch between sessions with **Ctrl+1** through **Ctrl+9**.

### Panes

The window is split into panes (1, 2, or 3). Each pane shows something different:

- **Claude Terminal** — where Claude is working
- **General Terminal** — a regular shell for you to run commands
- **Code Viewer** — view source files
- **TODO Editor** — your task list
- **Git Diff** — review changes Claude made

Toggle the layout with **Ctrl+J**.

---

## The Attention System

This is jc's most important feature. Instead of you checking every session manually, jc watches everything and tells you: **"Hey, this needs your attention."**

These are called **problems**. jc ranks them by urgency into 4 levels:

### 🔴 Emergency (most urgent)

**What:** Claude is stuck and can't continue without you.

**Examples:**
- Claude is asking for permission to edit a file
- Claude hit an error and stopped unexpectedly

**Why it matters:** Claude is blocked. Every minute you don't respond is wasted time.

**Scope:** jc checks ALL sessions across ALL projects. Even if you're working in project A, it will pull you to project B if Claude is stuck there.

### 🟡 Review Needed

**What:** There's something you should look at before more work happens.

**Examples:**
- Claude made git changes you haven't reviewed yet
- Your TODO.md has issues (a session heading without a Claude UUID)

**Why it matters:** Letting unreviewed work pile up leads to bigger problems later.

**Scope:** Current project only.

### 🔵 Action Suggested

**What:** Something you wrote in your TODO.md suggests you need to do something.

**Examples:**
- You wrote a "wait:" section (waiting for a code review, waiting for a deploy) but haven't sent it yet
- A task's checkboxes are all done but the session is still running

**Why it matters:** These are reminders you set for yourself.

**Note:** These are hidden when Claude is busy or when there are 🟡 Review items. jc doesn't nag you about low-priority stuff when there's urgent work.

**Scope:** Current project only.

### ⚪ Idle Check

**What:** Claude finished working and is sitting idle.

**Examples:**
- Claude completed a task and is waiting at the prompt
- Claude stopped on its own after finishing

**Why it matters:** Go see what Claude did! Maybe it's done and you can start the next task.

**Scope:** Current project only.

---

## How Ctrl+; Works

Press **Ctrl+;** and jc jumps you to the most urgent problem:

```
Press Ctrl+;
  → Any 🔴 Emergency?  → Jump to that session (even in another project)
  → Any 🟡 Review?     → Jump to the diff/file that needs review
  → Any 🔵 Action?     → Jump to your TODO.md
  → Any ⚪ Idle?        → Jump to the idle Claude session
  → Nothing?           → Land on TODO.md to plan next work
```

Press **Ctrl+;** again to cycle to the next problem. When all problems at one level are handled, it moves to the next level.

If jc jumped you away from your current project to handle a 🔴 Emergency, it remembers where you were and takes you back once the emergencies are cleared.

**That's it.** One key, always takes you to the right place.

---

## TODO.md Workflow

jc uses a `TODO.md` file in your project root as a live task board.

### Basic format

```markdown
# fix-auth
> uuid: abc-123-def

- [x] Find the broken middleware
- [ ] Write regression test
- [ ] Update docs

# refactor-api
> uuid: xyz-789

- [ ] Extract shared types
- [ ] Update error handling
```

### What each part means

**`# fix-auth`** — This is a session name. jc creates a Claude session labeled "fix-auth".

**`> uuid: abc-123-def`** — This links to a specific Claude conversation. When jc starts this session, it runs `claude --resume abc-123-def` so Claude picks up where it left off.

**`- [x]` / `- [ ]`** — Standard checkboxes. Track progress on subtasks.

### Special sections

**`## wait: reason`** — You're waiting on something external.

```markdown
# fix-auth
> uuid: abc-123-def

- [x] Write the fix
- [ ] Deploy to staging

## wait: need @alice to approve the PR
```

This creates an 🔵 Action problem reminding you to follow up.

**`## disabled`** — Pause this session. Claude won't be started for it.

```markdown
# refactor-api

## disabled
Blocked until fix-auth lands.
```

### The cycle

```
1. You write TODO.md with your tasks as headings
2. jc reads it, starts Claude sessions for each heading
3. Claude works in parallel on all active sessions
4. Things happen:
   - Claude finishes → ⚪ Idle problem appears
   - Claude needs permission → 🔴 Emergency problem appears
   - You haven't reviewed the diff → 🟡 Review problem appears
5. You press Ctrl+; → jc takes you to the right place
6. You handle it, check off items in TODO.md
7. Repeat until done
```

---

## Multi-Project in Practice

```
You're working on 3 projects:

  backend     — 2 sessions running (fix-auth, write-tests)
  frontend    — 1 session running (refactor-components)
  infra       — 1 session running (update-terraform)

You're currently looking at backend/fix-auth.

Meanwhile:
  - frontend/refactor-components hits a permission prompt → 🔴 Emergency
  - infra/update-terraform finishes → ⚪ Idle

You press Ctrl+;
  → jc jumps to frontend (🔴 is most urgent)
  → You approve the permission
  → Press Ctrl+; again
  → No more 🔴, jc takes you back to backend (where you were)
  → Press Ctrl+; again
  → 🟡 Review: backend has unreviewed git changes
  → You review the diff, mark it reviewed
  → Press Ctrl+; again
  → ⚪ Idle: infra session finished
  → You go check what Claude did in infra
  → Press Ctrl+; again
  → Nothing left! Lands on TODO.md. Plan your next task.
```

---

## Quick Reference

| Key | What it does |
|-----|-------------|
| Ctrl+; | **Go to next problem** (the most important key) |
| Ctrl+T | New Claude session |
| Ctrl+W | Close current session |
| Ctrl+1..9 | Switch to session 1-9 |
| Ctrl+O | Switch project |
| Ctrl+P | Open file |
| Ctrl+Shift+P | Command palette |
| Ctrl+D | View git diff |
| Ctrl+J | Toggle pane layout (1/2/3 panes) |
| Ctrl+/ | Move focus between panes |
| Ctrl+S | Save |
| Ctrl+Q | Quit |

---

## Summary

**jc = one window + multiple projects + multiple Claude sessions + automatic attention management**

You focus on the work. jc tells you where to look next.
