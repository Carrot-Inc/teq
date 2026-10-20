// expect: 9:11: error: Maximal number of successive inlines (32) exceeded
// expect: 10:11: error: Maximal number of successive inlines (32) exceeded
// expect: 2 errors found
// Each outermost call's overflow is its own error, at that call (`Inlines.inlineCall`'s position
// `(tree :: enclosingInlineds).last.srcPos`, `stopInlining` reset once the root is left): the second
// root's, through the same inlined place, is no repeat of the first's.
inline def recur(inline n: Int): Int = inline if n == 0 then 0 else recur(n - 1)
object O:
  val a = recur(33)
  val b = recur(33)
