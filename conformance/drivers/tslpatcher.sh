#!/usr/bin/env bash
# Install a mod with its own TSLPatcher, under Wine in a container, with no window anyone looks at.
#
#   conformance/drivers/tslpatcher.sh TSLPATCHDATA GAME OPTION [OUT]
#
#   TSLPATCHDATA  the mod's tslpatchdata/ directory; TSLPatcher's .exe is its sibling (one .exe
#                 beside it, or name it in $TSLPATCHER_EXE)
#   GAME          the game directory to patch, in place (take a copy first)
#   OPTION        0-based index into namespaces.ini's [Namespaces] list; ignored for a mod
#                 without one
#   OUT           where the log and the screenshots go (default: a directory under $TMPDIR)
#
# The mod directory is copied to OUT/mod first, because TSLPatcher writes beside its .exe (a
# backup/ folder and installlog.rtf); the copy is what runs. Afterwards OUT holds installlog.rtf,
# the backup/ folder, the screenshots taken at each step, and driver.log with the windows seen.
# Exit status is TSLPatcher's finish box having appeared (0), or the step that never came (1).
#
# The image is modshop's Wine image plus xdotool, openbox and ImageMagick
# (conformance/drivers/wine/Containerfile); $TSLPATCHER_IMAGE overrides its name. The game tree is
# mounted at /game, which TSLPatcher sees as Z:\game, and the folder is chosen in its Browse for
# Folder tree by keyboard: select "/", expand it, type "game", Enter. See
# docs/research/tslpatcher-oracle.md for what was measured.
set -euo pipefail

data=${1:?tslpatchdata directory}
game=${2:?game directory}
option=${3:?option number}
out=${4:-$(mktemp -d "${TMPDIR:-/tmp}/tslpatch.XXXXXX")}
image=${TSLPATCHER_IMAGE:-rakata-oracle-wine}

data=$(realpath "$data")
game=$(realpath "$game")
mkdir -p "$out"
out=$(realpath "$out")
moddir=$(dirname "$data")

if [ -n "${TSLPATCHER_EXE:-}" ]; then
  exe=$(basename "$TSLPATCHER_EXE")
else
  mapfile -t exes < <(find "$moddir" -maxdepth 1 -iname '*.exe' ! -iname 'nwnnsscomp.exe' -printf '%f\n')
  if [ "${#exes[@]}" -ne 1 ]; then
    echo "tslpatcher: expected one .exe beside $data, found ${#exes[@]}; set TSLPATCHER_EXE" >&2
    exit 2
  fi
  exe=${exes[0]}
fi

rm -rf "$out/mod"
cp -a "$moddir" "$out/mod"
rm -rf "$out/mod/backup" "$out/mod/installlog.rtf"

# What runs inside the container. Window names are the only state it reads; coordinates are
# relative to the window they click in, so the screen size does not matter.
cat >"$out/drive.sh" <<'EOF'
set -u
exe="$1"; opt="$2"
export DISPLAY=:77
Xvfb :77 -screen 0 1280x1024x24 -nolisten tcp >/out/xvfb.log 2>&1 &
sleep 1
openbox >/out/openbox.log 2>&1 &
sleep 1
shot() { import -window root "/out/$1.png" 2>/dev/null; }
wins() { # visible named windows: id<TAB>name
  for w in $(xdotool search --onlyvisible --name '.' 2>/dev/null); do
    n=$(xdotool getwindowname "$w" 2>/dev/null)
    case "$n" in Openbox|TSLPatcher|"") ;; *) printf '%s\t%s\n' "$w" "$n" ;; esac
  done
}
find_win() { wins | awk -F'\t' -v re="$1" '$2 ~ re {print $1; exit}'; }
wait_win() { # $1 regex, $2 seconds
  local w="" i
  for i in $(seq 1 $(( $2 * 2 ))); do
    w=$(find_win "$1"); [ -n "$w" ] && { echo "$w"; return 0; }
    sleep 0.5
  done
  return 1
}
other_win() { wins | awk -F'\t' '$2 !~ /Select installation type/ {print $1; exit}'; }

cd /mod
wine "/mod/$exe" >/out/wine.out 2>&1 &
wpid=$!

# 1. the namespace dialog, if the mod has one: the combo has focus; Down N times, then Select
for i in $(seq 1 40); do
  ns=$(find_win 'Select installation type'); [ -n "$ns" ] && break
  main=$(other_win); [ -n "$main" ] && break
  sleep 0.5
done
if [ -n "${ns:-}" ]; then
  sleep 0.5
  xdotool windowactivate --sync "$ns"; sleep 0.3
  for i in $(seq 1 "$opt"); do xdotool key Down; sleep 0.2; done
  shot 1-namespace
  xdotool key alt+s
  sleep 1
fi
# a message box before the main window (a missing info.rtf is a "Warning") takes Enter
for i in $(seq 1 40); do
  b=$(find_win '^(Information|Warning|Error)$')
  if [ -n "$b" ]; then echo "box before main: $(xdotool getwindowname "$b")"; shot "2-box-$i"; xdotool windowactivate --sync "$b"; xdotool key Return; sleep 0.5; continue; fi
  main=$(other_win); [ -n "$main" ] && break
  sleep 0.5
done
[ -n "${main:-}" ] || { echo "no main window"; wins; shot fail; exit 1; }
echo "main: $(xdotool getwindowname "$main")"
sleep 1
shot 2-main

# 2. Install Mod, bottom right of the main window; dialogs before the folder browser (a
#    ConfirmMessage) take Enter
xdotool windowactivate --sync "$main"; sleep 0.3
eval "$(xdotool getwindowgeometry --shell "$main")"
xdotool mousemove --window "$main" $((WIDTH - 62)) $((HEIGHT - 34)) click 1
br=""
for i in $(seq 1 60); do
  br=$(find_win 'Browse for Folder'); [ -n "$br" ] && break
  d=$(wins | awk -F'\t' -v m="$main" '$1 != m {print $1; exit}')
  if [ -n "$d" ]; then echo "dialog before browse: $(xdotool getwindowname "$d")"; shot 3-confirm; xdotool windowactivate --sync "$d"; xdotool key Return; fi
  sleep 0.5
done
[ -n "$br" ] || { echo "no Browse for Folder dialog"; wins; shot fail; exit 1; }
sleep 1
shot 4-browse

# 3. the folder: the tree lists Desktop, then "/" as its first child; select "/", expand it,
#    jump to "game" with the tree's type-ahead, OK
xdotool windowactivate --sync "$br"; sleep 0.3
xdotool mousemove --window "$br" 67 90 click 1; sleep 0.5
xdotool key Right; sleep 1
xdotool type --delay 40 game; sleep 1
shot 5-folder
xdotool key Return

# 4. the run: wait for the finish box, "Information" when all went well, "Warning" when the log
#    counts warnings, "Error" otherwise; its title is the verdict. Any other box takes Enter.
start=$(date +%s)
fin=""
while [ $(( $(date +%s) - start )) -lt "${TSLPATCH_TIMEOUT:-600}" ]; do
  fin=$(find_win '^(Information|Warning|Error)$'); [ -n "$fin" ] && break
  # a box that appeared after the check above is the finish box, for the next pass to find
  d=$(wins | awk -F'\t' -v m="$main" '$1 != m && $2 !~ /Browse for Folder/ && $2 !~ /^(Information|Warning|Error)$/ {print $1; exit}')
  if [ -n "$d" ]; then echo "dialog during install: $(xdotool getwindowname "$d")"; shot "6-dialog-$(date +%s)"; xdotool windowactivate --sync "$d"; xdotool key Return; fi
  kill -0 $wpid 2>/dev/null || { echo "TSLPatcher exited early"; shot fail; exit 1; }
  sleep 0.5
done
[ -n "$fin" ] || { echo "no finish box"; wins; shot fail; exit 1; }
echo "finished after $(( $(date +%s) - start ))s: $(xdotool getwindowname "$fin")"
shot 7-finished
xdotool windowactivate --sync "$fin"; xdotool key Return; sleep 0.5

# 5. Exit, bottom left; the log file is written on exit
xdotool windowactivate --sync "$main"; sleep 0.3
xdotool mousemove --window "$main" 59 $((HEIGHT - 34)) click 1
for i in $(seq 1 40); do kill -0 $wpid 2>/dev/null || break; sleep 0.5; done
kill -0 $wpid 2>/dev/null && { echo "still running after Exit; killing"; kill $wpid; }
wineserver --wait
ls -la /mod >/out/mod-after.txt
[ -f /mod/installlog.rtf ] && cp /mod/installlog.rtf /out/installlog.rtf
[ -f /mod/installlog.txt ] && cp /mod/installlog.txt /out/installlog.txt  # PlaintextLog=1
[ -d /mod/backup ] && cp -a /mod/backup /out/backup
exit 0
EOF

rm -f "$out/driver.log"
start=$(date +%s.%N)
podman run --rm --network none --security-opt label=disable \
  -v "$game:/game" -v "$out/mod:/mod" -v "$out:/out" \
  -e WINEDEBUG=-all -e "TSLPATCH_TIMEOUT=${TSLPATCH_TIMEOUT:-600}" \
  "$image" bash /out/drive.sh "$exe" "$option" 2>&1 \
  | grep -v 'X connection to\|XGetInputFocus' | tee "$out/driver.log"
status=${PIPESTATUS[0]}
end=$(date +%s.%N)
printf 'tslpatcher: %s option %s onto %s: status %s in %.1fs; log and screenshots in %s\n' \
  "$exe" "$option" "$game" "$status" "$(echo "$end - $start" | bc)" "$out"
exit "$status"
