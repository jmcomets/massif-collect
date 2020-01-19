#!/bin/bash

set -e

log() {
    #echo $*
    true
}

WORK_DIR=$(mktemp -d)
cleanup() {
    log "Deleting $WORK_DIR ..."
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

log "Creating FIFO in $WORK_DIR/pipe"
mkfifo "$WORK_DIR/pipe"

log "Ensuring $WORK_DIR/tty exists"
touch "$WORK_DIR/tty"

tmux split-window -h "tty > $WORK_DIR/tty && cat $WORK_DIR/pipe && exit"
tmux last-pane

tty=$(cat "$WORK_DIR/tty")
log "Running with tty = $tty"
cargo run -- --tui-stdout $tty || true

log "Finished, sending EOL to $WORK_DIR/pipe"
echo "\n" > "$WORK_DIR/pipe"

log "All done, exiting ..."
