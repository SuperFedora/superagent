#!/usr/bin/env bash
# Copyright 2026 SuperFedora Contributors
# SPDX-License-Identifier: Apache-2.0
# ==============================================================================
# quake-ai.sh — SuperDock [`] AI Harness Manager (Native LLM Harness + Pi for Ollama)
# ==============================================================================
# Usage:
#   quake-ai.sh [toggle|show|hide|restart|next-agent|set-agent <id>|list-agents]
#   quake-ai.sh [set-ollama <ip:port> [model]|add-agent <id> <label> <cmd>|remove-agent <id>]
#   quake-ai.sh [prompt-ollama|prompt-add|configure|status]
# ==============================================================================
set -euo pipefail

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/.minimax-code/bin:$HOME/.minimax/bin:$HOME/.grok/bin:$HOME/.npm-global/bin:/usr/local/bin:/usr/bin:/bin:$PATH"
SELF="${BASH_SOURCE[0]}"; SF_OLLAMA="$HOME/.local/bin/sf-ollama"
[[ -x "$SF_OLLAMA" ]] || SF_OLLAMA="$(command -v sf-ollama 2>/dev/null || echo "$HOME/.local/bin/sf-ollama")"
STATE_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/superfedora"; FAST_FILE="${XDG_RUNTIME_DIR:-/tmp}/quake-ai-${HYPRLAND_INSTANCE_SIGNATURE:-0}.env"
STATE_FILE="$STATE_DIR/quake-ai.env"; AGENTS_FILE="$STATE_DIR/ai-agents.conf"
[[ -d "$STATE_DIR" ]] || mkdir -p "$STATE_DIR"

AI_AGENT="agy"; OLLAMA_HOST="127.0.0.1:11434"; OLLAMA_MODEL=""
# shellcheck disable=SC1090
[[ -f "$STATE_FILE" ]] && source "$STATE_FILE"
AI_AGENT="${AI_AGENT:-agy}"; OLLAMA_HOST="${OLLAMA_HOST:-127.0.0.1:11434}"

detect_ollama_model() { local h="${1:-$OLLAMA_HOST}" pref="${2:-}" m=""; [[ -x "$SF_OLLAMA" ]] && m="$("$SF_OLLAMA" best-model "$h" "$pref" 2>/dev/null || true)"; printf '%s' "${m:-${pref:-llama3.2}}"; }
OLLAMA_MODEL="${OLLAMA_MODEL:-llama3.2}"
[[ -f "$AGENTS_FILE" ]] || printf '# SuperFedora Custom AI Agents (id|label|command)\n' > "$AGENTS_FILE"
save_state() { printf 'AI_AGENT=%q\nOLLAMA_HOST=%q\nOLLAMA_MODEL=%q\n' "$AI_AGENT" "$OLLAMA_HOST" "$OLLAMA_MODEL" > "$STATE_FILE"; }

sync_pi_ollama() {
    local host="${1:-$OLLAMA_HOST}" model="${2:-$OLLAMA_MODEL}" probe=""
    [[ -x "$SF_OLLAMA" ]] && probe="$("$SF_OLLAMA" probe "$host" "$model" 2>/dev/null || true)"
    python3 -c 'import json,os,sys; path,host,cur,probe=sys.argv[1:5]; os.makedirs(os.path.dirname(path),exist_ok=True); data=json.load(open(path)) if os.path.exists(path) else {}; models=[]; seen=set()
for line in probe.splitlines():
 p=line.split("|")
 if len(p)>=2 and p[0]=="OLLAMA_MODEL" and p[1] and p[1] not in seen: seen.add(p[1]); models.append({"id":p[1]})
if cur and cur not in seen: models.insert(0,{"id":cur})
data["$schema"]="https://pi.dev/schemas/models.schema.json"; data.setdefault("providers",{})["ollama"]={"baseUrl":f"http://{host}/v1","api":"openai-completions","apiKey":"ollama","models":models or [{"id":"llama3.2"}]}; json.dump(data,open(path,"w"),indent=2)' "$HOME/.pi/agent/models.json" "$host" "$model" "$probe" 2>/dev/null || true
}
normalize_id() {
    case "${1:-$AI_AGENT}" in
        agy|antigravity*) echo "agy" ;; mcode|minimax*) echo "mcode" ;; grok|grok-cli) echo "grok" ;; vibe|mistral*) echo "vibe" ;;
        claude*) echo "claude" ;; chatgpt|chatgtp|codex|openai) echo "chatgpt" ;; ollama|local|pi) echo "ollama" ;; *) echo "${1:-agy}" ;;
    esac
}
all_agents() {
    local chat_bin="codex" cid clabel ccmd cbin; command -v chatgpt >/dev/null 2>&1 && chat_bin="chatgpt"
    printf 'agy|core|Antigravity CLI (agy)|agy|agy\nmcode|core|MiniMax Code (mcode)|mcode|mcode\ngrok|core|xAI Grok CLI (grok)|grok|grok\nvibe|core|Mistral Vibe (vibe)|vibe|vibe\n'
    printf 'claude|standard|Claude Code (claude)|claude|claude\nchatgpt|standard|ChatGPT / Codex (%s)|%s|%s\nollama|local|Ollama via Pi (%s // %s)|pi --provider ollama --model %s|pi\n' "$chat_bin" "$chat_bin" "$chat_bin" "$OLLAMA_HOST" "$OLLAMA_MODEL" "$OLLAMA_MODEL"
    if [[ -f "$AGENTS_FILE" ]]; then
        while IFS='|' read -r cid clabel ccmd; do
            [[ -z "${cid// /}" || "$cid" == \#* ]] && continue
            case "$cid" in agy|mcode|grok|vibe|claude|chatgpt|ollama) continue ;; esac
            clabel="${clabel//|/ -}"; ccmd="${ccmd//|/ }"; cbin="${ccmd%% *}"; printf '%s|custom|%s|%s|%s\n' "$cid" "${clabel:-$cid}" "${ccmd:-$cid}" "${cbin:-$cid}"
        done < "$AGENTS_FILE"
    fi
}
resolve_agent() {
    local target cid clabel ccmd cbin found=0; target="$(normalize_id "${1:-$AI_AGENT}")"
    while IFS='|' read -r cid _ clabel ccmd cbin; do [[ "$cid" == "$target" ]] && { AI_AGENT="$cid"; AI_LABEL="$clabel"; AI_CMD="$ccmd"; AI_BIN="$cbin"; found=1; break; }; done < <(all_agents)
    [[ $found -eq 0 ]] && { AI_AGENT="agy"; AI_LABEL="Antigravity CLI (agy)"; AI_CMD="agy"; AI_BIN="agy"; }; save_state
}
list_agents() {
    local probe_out="" ol_online="0" ol_best="" cid ccat clabel ccmd cbin inst act
    if [[ -x "$SF_OLLAMA" ]]; then
        probe_out="$("$SF_OLLAMA" probe "$OLLAMA_HOST" "$OLLAMA_MODEL" 2>/dev/null || true)"
        IFS='|' read -r _ OLLAMA_HOST ol_online ol_best _ <<<"${probe_out%%$'\n'*}"
        [[ -n "$ol_best" && "$ol_best" != "$OLLAMA_MODEL" ]] && OLLAMA_MODEL="$ol_best"
    fi
    resolve_agent "$AI_AGENT"; printf 'CFG|%s|%s|%s|%s\n' "$AI_AGENT" "$AI_LABEL" "$OLLAMA_HOST" "$OLLAMA_MODEL"
    while IFS='|' read -r cid ccat clabel ccmd cbin; do
        inst=0; command -v "$cbin" >/dev/null 2>&1 && inst=1; [[ "$cid" == "ollama" && "$ol_online" == "1" ]] && inst=1
        act=0; [[ "$cid" == "$AI_AGENT" ]] && act=1; printf 'AGENT|%s|%s|%s|%s|%d|%d\n' "$cid" "$ccat" "$clabel" "$ccmd" "$inst" "$act"
    done < <(all_agents)
    [[ -n "$probe_out" ]] && printf '%s\n' "$probe_out"
}
set_ollama() {
    local host="${1:-$OLLAMA_HOST}" model="${2:-}"; host="${host#http://}"; host="${host#https://}"; host="${host%/}"; host="${host%%/*}"
    [[ -n "$host" ]] && { [[ "$host" != *:* ]] && host="$host:11434"; OLLAMA_HOST="$host"; }
    OLLAMA_MODEL="$(detect_ollama_model "$OLLAMA_HOST" "${model%% *}")"; save_state; sync_pi_ollama "$OLLAMA_HOST" "$OLLAMA_MODEL"
}
add_custom_agent() {
    local id="${1:-}" label="${2:-}" cmd="${3:-}" tmp="$AGENTS_FILE.tmp"; id="$(echo "$id" | tr '[:upper:] ' '[:lower:]-' | tr -cd 'a-z0-9_-')"; label="${label//|/ -}"; cmd="${cmd//|/ }"
    [[ -n "$id" && -n "$cmd" ]] || { echo "Usage: quake-ai.sh add-agent <id> <label> <command>" >&2; exit 2; }
    case "$id" in agy|mcode|grok|vibe|claude|chatgpt|ollama) echo "Cannot override built-in agent '$id'" >&2; exit 2 ;; esac
    [[ -z "$label" ]] && label="$id ($cmd)"; if [[ -f "$AGENTS_FILE" ]]; then grep -Ev "^${id}\\|" "$AGENTS_FILE" > "$tmp" || true; mv "$tmp" "$AGENTS_FILE"; fi
    printf '%s|%s|%s\n' "$id" "$label" "$cmd" >> "$AGENTS_FILE"
}
remove_custom_agent() {
    local id="${1:-}" tmp="$AGENTS_FILE.tmp"; [[ -n "$id" && -f "$AGENTS_FILE" ]] || return 0
    grep -Ev "^${id}\\|" "$AGENTS_FILE" > "$tmp" || true; mv "$tmp" "$AGENTS_FILE"; [[ "$AI_AGENT" == "$id" ]] && { AI_AGENT="agy"; save_state; } || true
}

compute_geometry() {
    local mon_json full_w full_h hdr_h="${1:-40}"; [[ "$hdr_h" =~ ^[0-9]+$ ]] || hdr_h=40
    mon_json="$(hyprctl monitors -j 2>/dev/null || echo '[]')"
    read -r MON_NAME MON_X MON_Y MON_W MON_H SPEC_OPEN < <(jq -r '(.[] | select(.focused == true)) // .[0] // {name:"default", x:0, y:0, width:1920, height:1080, scale:1} | "\(.name // "default") \(.x) \(.y) \((.width / (.scale // 1)) | round) \((.height / (.scale // 1)) | round) \(if .specialWorkspace.name == "special:ai" then 1 else 0 end)"' <<<"$mon_json")
    MON_NAME="${MON_NAME:-default}"; MON_X="${MON_X:-0}"; MON_Y="${MON_Y:-0}"; MON_W="${MON_W:-1920}"; MON_H="${MON_H:-1080}"; SPEC_OPEN="${SPEC_OPEN:-0}"
    full_w=$(( (MON_W * 62 + 50) / 100 )); full_h=$(( (MON_H * 43 + 50) / 100 ))
    (( full_w > 1080 )) && full_w=1080; (( full_w < 960 )) && full_w=960; (( full_w > MON_W - 40 )) && full_w=$(( MON_W - 40 ))
    (( full_h > 460 )) && full_h=460; (( full_h < 410 )) && full_h=410; (( full_h > MON_H - 64 )) && full_h=$(( MON_H - 64 ))
    Q_FULL_H="$full_h"; Q_BASE_Y=$(( MON_Y + 42 ))
    Q_X=$(( MON_X + (MON_W - full_w) / 2 + 12 )); Q_Y=$(( Q_BASE_Y + hdr_h + 6 ))
    Q_W=$(( full_w - 24 )); Q_H=$(( full_h - hdr_h - 20 ))
}

has_quake_client() { hyprctl clients -j 2>/dev/null | grep -q '"class": "quake-ai"'; }
is_special_ai_open() { hyprctl monitors -j 2>/dev/null | grep -q '"special:ai"'; }
save_fast() { printf 'Q_PID=%q\nQ_X=%q\nQ_BASE_Y=%q\nQ_W=%q\nQ_FULL_H=%q\nQ_HDR_H=%q\nQ_OPEN=%q\n' "${Q_PID:-}" "${Q_X:-432}" "${Q_BASE_Y:-42}" "${Q_W:-1056}" "${Q_FULL_H:-460}" "${Q_HDR_H:-40}" "${1:-0}" > "$FAST_FILE"; }

spawn_quake() {
    [[ "$AI_AGENT" == "ollama" ]] && { OLLAMA_MODEL="$(detect_ollama_model "$OLLAMA_HOST" "$OLLAMA_MODEL")"; sync_pi_ollama "$OLLAMA_HOST" "$OLLAMA_MODEL"; }
    resolve_agent "$AI_AGENT"
    local runner="$STATE_DIR/quake-ai-runner.sh" safe_title="${AI_LABEL//[\'\"]/}"
    {
        printf '#!/usr/bin/env bash\nexport PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/.minimax-code/bin:$HOME/.minimax/bin:$HOME/.grok/bin:$HOME/.npm-global/bin:/usr/local/bin:/usr/bin:/bin:$PATH"\n'
        printf 'export AI_AGENT=%q AI_LABEL=%q AI_BIN=%q AI_CMD=%q OLLAMA_HOST=%q OLLAMA_MODEL=%q SF_OLLAMA=%q SELF=%q\n' "$AI_AGENT" "$AI_LABEL" "$AI_BIN" "$AI_CMD" "$OLLAMA_HOST" "$OLLAMA_MODEL" "$SF_OLLAMA" "$SELF"
        cat <<'EOF'
clear
if [[ "$AI_AGENT" == "ollama" ]]; then
    if command -v pi >/dev/null 2>&1; then pi --provider ollama --model "$OLLAMA_MODEL"
    elif command -v ollama >/dev/null 2>&1; then OLLAMA_HOST="$OLLAMA_HOST" ollama run "$OLLAMA_MODEL"
    elif [[ -x "$SF_OLLAMA" ]]; then "$SF_OLLAMA" chat "$OLLAMA_HOST" "$OLLAMA_MODEL"
    else printf '\033[1;33m[!]\033[0m Cannot connect to Ollama at %s (%s)\n' "$OLLAMA_HOST" "$OLLAMA_MODEL"; fi
elif command -v "$AI_BIN" >/dev/null 2>&1; then eval "$AI_CMD"
else printf '\033[1;33m[!]\033[0m %s (%s) is not installed yet. Configure via Top Bar AI menu or: sf ai configure\n' "$AI_LABEL" "$AI_BIN"; fi
"${SHELL:-/bin/bash}"
"$SELF" hide >/dev/null 2>&1 || true
EOF
    } > "$runner"; chmod +x "$runner"
    hyprctl dispatch exec "[workspace special:ai silent; float; size $Q_W $Q_H; move $Q_X $Q_Y] kitty --class quake-ai -o 'map escape launch --type=background $SELF hide' -o 'map shift+escape send_text all \x1b' -o 'map alt+grave launch --type=background qs ipc call superdeck open ai' -o 'map alt+1 launch --type=background qs ipc call superdeck open 1' -o 'map alt+2 launch --type=background qs ipc call superdeck open 2' -o 'map alt+3 launch --type=background qs ipc call superdeck open 3' -o 'map alt+4 launch --type=background qs ipc call superdeck open 4' -o 'map alt+5 launch --type=background qs ipc call superdeck open 5' -o 'map alt+6 launch --type=background qs ipc call superdeck open 6' -o 'map alt+7 launch --type=background qs ipc call superdeck open 7' -o 'map alt+8 launch --type=background qs ipc call superdeck open 8' -o 'map alt+9 launch --type=background qs ipc call superdeck open 9' -o 'map alt+0 launch --type=background qs ipc call superdeck open 0' --title 'Quake AI — $safe_title' -e $runner" >/dev/null
    for _ in {1..30}; do has_quake_client && break; sleep 0.04; done
}

show_quake() {
    local hdr_h="${1:-40}" batch=""
    exec 9>"$STATE_DIR/quake-ai.lock"; flock -w 2 9 || true
    # shellcheck disable=SC1090
    [[ -f "$FAST_FILE" ]] && source "$FAST_FILE"
    compute_geometry "$hdr_h"
    if [[ -n "${Q_PID:-}" && -d "/proc/$Q_PID" ]] && tr '\0' ' ' < "/proc/$Q_PID/cmdline" 2>/dev/null | grep -q 'alt+1'; then
        Q_HDR_H="$hdr_h"
        (( SPEC_OPEN == 1 )) || batch="dispatch togglespecialworkspace ai; "
        hyprctl --batch "${batch}dispatch resizewindowpixel exact $Q_W $Q_H,class:^(quake-ai)$; dispatch movewindowpixel exact $Q_X $Q_Y,class:^(quake-ai)$; dispatch focuswindow class:^(quake-ai)$" >/dev/null
        save_fast 1; return 0
    fi
    if has_quake_client; then
        Q_PID="$(hyprctl clients -j 2>/dev/null | jq -r '.[] | select(.class=="quake-ai") | .pid' | head -n1)"
        if [[ -n "$Q_PID" ]] && ! tr '\0' ' ' < "/proc/$Q_PID/cmdline" 2>/dev/null | grep -q 'alt+1'; then
            hyprctl dispatch closewindow "class:^(quake-ai)$" >/dev/null 2>&1 || true
            for _ in {1..20}; do has_quake_client || break; sleep 0.04; done
        fi
    fi
    has_quake_client || spawn_quake
    Q_PID="$(hyprctl clients -j 2>/dev/null | jq -r '.[] | select(.class=="quake-ai") | .pid' | head -n1)"; Q_HDR_H="$hdr_h"
    is_special_ai_open || batch="dispatch togglespecialworkspace ai; "
    hyprctl --batch "${batch}dispatch resizewindowpixel exact $Q_W $Q_H,class:^(quake-ai)$; dispatch movewindowpixel exact $Q_X $Q_Y,class:^(quake-ai)$; dispatch focuswindow class:^(quake-ai)$" >/dev/null
    save_fast 1
}

hide_quake() {
    exec 9>"$STATE_DIR/quake-ai.lock"; flock -w 2 9 || true
    # shellcheck disable=SC1090
    [[ -f "$FAST_FILE" ]] && source "$FAST_FILE"
    if is_special_ai_open; then hyprctl dispatch togglespecialworkspace ai >/dev/null || true; fi
    [[ -n "${Q_PID:-}" && -d "/proc/$Q_PID" ]] && save_fast 0
    [[ "${1:-}" == "keep-deck" ]] || qs ipc call superdeck close >/dev/null 2>&1 || true
}
relaunch_quake() { rm -f "$FAST_FILE"; hyprctl dispatch closewindow "class:^(quake-ai)$" >/dev/null 2>&1 || true; for _ in {1..25}; do has_quake_client || break; sleep 0.04; done; show_quake "${1:-40}"; }

prompt_ollama() {
    local host model choices
    host="$(printf '%s\n127.0.0.1:11434\nlocalhost:11434\n' "$OLLAMA_HOST" | awk '!seen[$0]++' | fuzzel --dmenu --prompt 'Ollama Server (host:port) > ' --width 48)" || return 0
    [[ -n "${host// /}" ]] || return 0; set_ollama "$host" ""
    choices="$("$SF_OLLAMA" fuzzel-models "$OLLAMA_HOST" "$OLLAMA_MODEL" 2>/dev/null || true)"
    model="$(printf '%s\n' "$choices" | fuzzel --dmenu --prompt "Model ($OLLAMA_HOST) > " --width 56)" || return 0
    [[ -z "${model// /}" || "$model" == "[!]"* ]] && return 0
    set_ollama "$OLLAMA_HOST" "${model%% *}"; resolve_agent "ollama"
    if has_quake_client && is_special_ai_open; then relaunch_quake; fi
}

prompt_add_agent() {
    local cmd id label
    cmd="$(printf 'pi\naider\nopencode\ngemini\n' | fuzzel --dmenu --prompt 'Agent Command > ' --width 44)" || return 0
    [[ -n "${cmd// /}" ]] || return 0
    id="$(echo "${cmd%% *}" | tr '[:upper:]' '[:lower:]' | tr -cd 'a-z0-9_-')"
    label="$(printf '%s (%s)\n%s\n' "${id^} CLI" "$cmd" "$cmd" | fuzzel --dmenu --prompt 'Agent Label > ' --width 44)" || return 0
    add_custom_agent "$id" "${label:-$id}" "$cmd"; resolve_agent "$id"
    if has_quake_client && is_special_ai_open; then relaunch_quake; fi
}

ask_agent() {
    local q="${*:-}" out=""; [[ -n "${q// /}" ]] || { echo "Please enter a prompt."; return 0; }; resolve_agent "$AI_AGENT"
    [[ "$AI_AGENT" == "ollama" ]] && command -v pi >/dev/null 2>&1 && { sync_pi_ollama "$OLLAMA_HOST" "$OLLAMA_MODEL"; if out="$(timeout 45 pi --provider ollama --model "$OLLAMA_MODEL" -p "$q" </dev/null 2>&1)" && [[ -n "${out// /}" ]]; then printf '%s\n' "$out"; return 0; fi; }
    case "$AI_AGENT" in agy|grok|vibe|claude) if command -v "$AI_BIN" >/dev/null 2>&1 && out="$(timeout 45 "$AI_BIN" -p "$q" </dev/null 2>&1)" && [[ -n "${out// /}" ]]; then printf '%s\n' "$out"; return 0; fi ;; esac
    "$SF_OLLAMA" ask "$OLLAMA_HOST" "$OLLAMA_MODEL" "$q"
}

configure_tui() {
    local probe_out ol_online ol_best ol_st loaded_list all_list cid ccat clabel ccmd cbin st mk act aid oh om nid nlbl ncmd rid mnames=()
    while true; do
        probe_out="$("$SF_OLLAMA" probe "$OLLAMA_HOST" "$OLLAMA_MODEL" 2>/dev/null || true)"
        IFS='|' read -r _ OLLAMA_HOST ol_online ol_best _ <<<"${probe_out%%$'\n'*}"
        [[ -n "$ol_best" && "$ol_best" != "$OLLAMA_MODEL" ]] && OLLAMA_MODEL="$ol_best"
        ol_st="OFFLINE"; [[ "$ol_online" == "1" ]] && ol_st="ONLINE"
        loaded_list="$(awk -F'|' '$1=="OLLAMA_MODEL" && $3=="1"{printf "%s ", $2}' <<<"$probe_out")"
        all_list="$(awk -F'|' '$1=="OLLAMA_MODEL"{printf "%s ", $6}' <<<"$probe_out")"
        resolve_agent "$AI_AGENT"; clear
        printf '=== SUPERFEDORA QUAKE AI // HARNESS & OLLAMA CONFIGURATOR ===\n  Active Harness: %s [%s]\n  Ollama Server : http://%s [%s] (Pi Model: %s)\n  VRAM Loaded   : %s\n  Server Models : %s\n\n' "$AI_LABEL" "$AI_AGENT" "$OLLAMA_HOST" "$ol_st" "$OLLAMA_MODEL" "${loaded_list:-none}" "${all_list:-none}"
        while IFS='|' read -r cid ccat clabel ccmd cbin; do
            st="[--]"; mk="   "; command -v "$cbin" >/dev/null 2>&1 && st="[OK]"
            [[ "$cid" == "ollama" && "$ol_online" == "1" ]] && st="[OK]"; [[ "$cid" == "$AI_AGENT" ]] && mk="[*]"
            printf '  %s %-10s %-10s %-34s (%s) %s\n' "$mk" "$cid" "[$ccat]" "$clabel" "$ccmd" "$st"
        done < <(all_agents)
        printf '\n  1) Select active harness  2) Set Ollama Server & Pick Pi Model  3) Add custom agent  4) Remove custom agent  5) Launch Quake AI  q) Quit\n'
        read -r -p "  Select action [1-5/q]: " act
        case "$act" in
            1) read -r -p "  Agent ID: " aid; [[ -n "$aid" ]] && resolve_agent "$aid" ;;
            2) read -r -p "  Ollama Host/URL [$OLLAMA_HOST]: " oh; set_ollama "${oh:-$OLLAMA_HOST}" ""; "$SF_OLLAMA" table "$OLLAMA_HOST" "$OLLAMA_MODEL"
               mapfile -t mnames < <("$SF_OLLAMA" probe "$OLLAMA_HOST" "$OLLAMA_MODEL" | awk -F'|' '$1=="OLLAMA_MODEL"{print $2}')
               read -r -p "  Select model (#, name, or Enter for $OLLAMA_MODEL): " om
               [[ "$om" =~ ^[0-9]+$ ]] && (( om >= 1 && om <= ${#mnames[@]} )) && om="${mnames[$((om - 1))]}"
               set_ollama "$OLLAMA_HOST" "${om:-$OLLAMA_MODEL}"; resolve_agent "ollama" ;;
            3) read -r -p "  ID: " nid; read -r -p "  Label: " nlbl; read -r -p "  Command: " ncmd; [[ -n "$nid" && -n "$ncmd" ]] && add_custom_agent "$nid" "$nlbl" "$ncmd" ;;
            4) read -r -p "  Custom ID to remove: " rid; [[ -n "$rid" ]] && remove_custom_agent "$rid" ;;
            5) relaunch_quake; break ;;
            q|Q|"") break ;;
        esac
    done
}

case "${1:-toggle}" in
    toggle)        if has_quake_client && is_special_ai_open; then hide_quake; else show_quake "${2:-40}"; fi ;;
    show|open)     show_quake "${2:-40}" ;;
    hide|close)    hide_quake "${2:-}" ;;
    restart|relaunch) relaunch_quake "${2:-40}" ;;
    select-agent)  resolve_agent "${2:-agy}"; qs ipc call status refresh >/dev/null 2>&1 || true; if has_quake_client; then relaunch_quake; fi ;;
    set-agent)     resolve_agent "${2:-agy}"; qs ipc call status refresh >/dev/null 2>&1 || true; relaunch_quake ;;
    next-agent)    mapfile -t IDS < <(all_agents | cut -d'|' -f1); NEXT="${IDS[0]:-agy}"; for i in "${!IDS[@]}"; do [[ "${IDS[$i]}" == "$AI_AGENT" ]] && { NEXT="${IDS[$(( (i + 1) % ${#IDS[@]} ))]}"; break; }; done
                   resolve_agent "$NEXT"; qs ipc call status refresh >/dev/null 2>&1 || true; if has_quake_client; then relaunch_quake; fi ;;
    ask|query)     shift; ask_agent "$@" ;;
    list-agents)   list_agents ;;
    set-ollama)    set_ollama "${2:-}" "${3:-}" ;;
    add-agent)     add_custom_agent "${2:-}" "${3:-}" "${4:-}" ;;
    remove-agent)  remove_custom_agent "${2:-}" ;;
    prompt-ollama) prompt_ollama ;;
    prompt-add)    prompt_add_agent ;;
    configure|tui) configure_tui ;;
    status)        resolve_agent "$AI_AGENT"; compute_geometry "${2:-40}"; printf 'Agent: %s (%s) | Ollama: %s (%s) | Geometry: %dx%d+%d+%d\n' "$AI_LABEL" "$AI_CMD" "$OLLAMA_HOST" "$OLLAMA_MODEL" "$Q_W" "$Q_H" "$Q_X" "$Q_Y" ;;
    *)             sed -n '7,10p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
