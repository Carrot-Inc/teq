package icba

// An inline method whose match's wildcard case is a block (`case _ => 0`, the typer's `{ 0 }`). Each expansion copies
// the stored body and walks the copy; the walk writes only what the expansion made, never the stored body (dotty's
// TreeTypeMap returns new trees, TreeTypeMap.scala:110, 117-123), so a/'s products are the same whether b/ is built with
// a/ (whole, b/'s expansions running before a/'s pickle is written) or over a/'s products (split). scalac pickles the
// case as `CASEDEF(IDENT _, BLOCK(INTconst 0))`.
object M:
  inline def first(t: Any): Int = t match
    case a: Int => a
    case _ => 0
