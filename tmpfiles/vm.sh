#!/usr/bin/env bash
# Edit on the Mac, build on the GCP VM `warp-dev`.
#
#   tmpfiles/vm.sh sync          mirror this checkout to ~/src/warp on the VM
#   tmpfiles/vm.sh run <cmd...>  sync, then run <cmd> in ~/src/warp on the VM
#   tmpfiles/vm.sh ssh [cmd...]  run <cmd> on the VM without syncing (or open a shell)
#
# The Mac checkout is the source of truth; the VM copy is a build mirror
# (`--delete`, excluding build outputs), so never edit files on the VM.
set -euo pipefail

INSTANCE=${WARP_VM_INSTANCE:-warp-dev}
ZONE=${WARP_VM_ZONE:-us-central1-a}
REMOTE_DIR=src/warp
export PATH="$HOME/google-cloud-sdk/bin:$PATH"

repo=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
user=$(gcloud config get-value account 2>/dev/null | sed 's/[@.]/_/g')
ip=$(gcloud compute instances describe "$INSTANCE" --zone "$ZONE" \
  --format='get(networkInterfaces[0].accessConfigs[0].natIP)')
id=$(gcloud compute instances describe "$INSTANCE" --zone "$ZONE" --format='get(id)')
ssh_opts=(-i "$HOME/.ssh/google_compute_engine"
  -o UserKnownHostsFile="$HOME/.ssh/google_compute_known_hosts"
  -o HostKeyAlias="compute.$id" -o ServerAliveInterval=30)

vm_ssh() { ssh "${ssh_opts[@]}" "$user@$ip" "$@"; }

vm_sync() {
  vm_ssh "mkdir -p $REMOTE_DIR"
  rsync -az --delete -e "ssh ${ssh_opts[*]}" \
    --exclude /target/ --exclude node_modules/ --exclude /.tmpfiles/ \
    "$repo/" "$user@$ip:$REMOTE_DIR/"
}

case "${1:-}" in
  sync) vm_sync ;;
  run) shift; vm_sync; vm_ssh -t "cd $REMOTE_DIR && source ~/.cargo/env 2>/dev/null; $*" ;;
  ssh) shift; if (($#)); then vm_ssh "source ~/.cargo/env 2>/dev/null; $*"; else vm_ssh -t; fi ;;
  *) sed -n '2,9p' "$0"; exit 1 ;;
esac
