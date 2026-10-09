# Sourced by the bench scripts: `phases` reads what `teq --time` printed and gives its phases
# as one line of name and ms pairs, `lines 91659 read 20.31 parse 15.72 type 2781.45 total
# 3477.13`; it fails where the output has none (the command failed). A binary built before
# 2026-09-28 printed them on one line with a unit per value (`checked N lines in T (read ..)`,
# `N lines: read .., total ..`), which a comparison against such a binary still reads; the
# units there include `µs`, hence the C locale.
phases() {
  LC_ALL=C awk '
    function ms(t,    n, unit) {
      gsub(/[,()]/, "", t)
      n = t + 0
      unit = t
      sub(/^[0-9.]+/, "", unit)
      if (unit == "s") return n * 1000
      if (unit == "ms") return n
      if (unit == "ns") return n / 1000000
      return n / 1000
    }
    function pairs(from,    i, name) {
      for (i = from; i < NF; i++) {
        name = $i
        gsub(/[()]/, "", name)
        if (name ~ /^(read|parse|type|reach|emit|write|run|total)$/) line = line name " " sprintf("%.2f", ms($(i + 1))) " "
      }
    }
    /^phases$/ { on = 1; next }
    /^[^ ]/ { on = 0 }
    on && $1 == "lines" { gsub(",", "", $2); line = line "lines " $2 " "; next }
    on && $3 == "ms" { line = line $1 " " $2 " " }
    /^checked [0-9]+ lines in / { line = "lines " $2 " "; pairs(6); line = line "total " sprintf("%.2f", ms($5)) " " }
    /^[0-9]+ lines: read / { line = "lines " $1 " "; pairs(3) }
    END { if (line == "") exit 1; sub(/ $/, "", line); print line }
  '
}
