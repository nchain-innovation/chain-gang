#!/bin/sh
set -eu
: "${BSV_CONTAINER:?Choose a fresh task-owned container name}"
set -- -regtest -server -listen=0 -dnsseed=0 -discover=0 -connect=0 -rpcuser=cgtest -rpcpassword=local-regtest-only -txindex=1 -genesisactivationheight=1 -chronicleactivationheight=200 -excessiveblocksize=100000000 -minminingtxfee=0.000005
case "${BSV_PROFILE:-flexible}" in
 flexible) set -- "$@" -maxstackmemoryusageconsensus=1000000000 -maxstackmemoryusagepolicy=1000000000 -maxscriptnumlengthpolicy=0 -maxscriptsizepolicy=0 -maxtxsizepolicy=100000000 -maxstdtxvalidationduration=10000 -maxnonstdtxvalidationduration=20000 -maxtxnvalidatorasynctasksrunduration=30000 -acceptnonstdtxn=1 ;;
 legacy) set -- "$@" -genesisactivationheight=150 -acceptnonstdtxn=1 ;;
 audit) set -- "$@" -acceptnonstdtxn=1 -checknonfinalfreq=1000 ;;
 disabled) set -- "$@" -maxmempoolnonfinal=0 ;;
 standard) set -- "$@" -acceptnonstdtxn=0 -acceptnonstdoutputs=0 ;;
 default) set -- "$@" -acceptnonstdtxn=0 ;;
 rate) set -- "$@" -mempoolnonfinalmaxreplacementrate=2 -mempoolnonfinalmaxreplacementrateperiod=60 ;;
 *) echo 'BSV_PROFILE must be flexible, legacy, audit, disabled, standard, default or rate' >&2; exit 1 ;;
esac
container run -d --name "$BSV_CONTAINER" --arch amd64 --rosetta --cpus 2 --memory 3G docker.io/bitcoinsv/bitcoin-sv@sha256:d96fe4786a00e5fc10bc9e0e76a6006376f9472d4eb74d07e4c1e0d019493d33 bitcoind "$@"
# Container startup does not imply RPC readiness. Fail explicitly on startup
# trouble; do not count a connection error as a transaction rejection.
attempt=0
until container exec "$BSV_CONTAINER" bitcoin-cli -regtest -rpcuser=cgtest -rpcpassword=local-regtest-only -rpcclienttimeout=1 getblockcount >/dev/null 2>&1; do
 attempt=$((attempt + 1))
 if [ "$attempt" -ge 30 ]; then echo 'Node RPC did not become ready' >&2; exit 1; fi
 sleep 1
done
