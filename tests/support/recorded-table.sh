# --record writes tests/<table>/{expected.tsv,pending.txt} from scalac and TEQ.
# --check holds TEQ to those records. --update removes pending legs that now match scalac.
here=$(cd "$(dirname "$0")" && pwd)
table=${0##*/}
table=${table%.sh}
table_mode=compare
head=
case ${1:-} in
  --record|--check|--update)
    table_mode=${1#--}
    [[ $# -le 2 ]] || { echo "usage: $0 $1 [empty output directory]" >&2; exit 2; }
    base=$(realpath "${TEQ:-$here/../../target/release/teq}")
    if [[ $# == 2 ]]; then
      out=$(realpath -m "$2")
    else
      mkdir -p "$here/../../out/$table"
      out=$(mktemp -d "$here/../../out/$table/run.XXXXXXXX")
      out=$(realpath "$out")
      trap 'rm -rf -- "${out:?}"' EXIT
    fi
    ;;
  *)
    base=$(realpath "${1:?base teq}")
    out=$(realpath -m "${2:?empty output directory}")
    head=${3:+$(realpath "$3")}
    ;;
esac
[[ -f $base && -x $base ]] || { echo "no executable at $base" >&2; exit 2; }
mkdir -p "$out"
[[ -z $(ls -A "$out") ]] || { echo "$table: output directory must be empty: $out" >&2; exit 2; }

table_judge() {
  python3 -B "$here/recorded_table.py" "$table" "$1" "$out"
}
