object Lib:
  inline def forwarded(inline x: Int): String = M.loc(M.ident(x))
  inline def quoted(inline x: Int): String = M.loc(M.wrap(x))
