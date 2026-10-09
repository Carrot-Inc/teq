// expect: 14:18: error: value f is not a member of String
// `private[p]` names the package `p` around the definition, not every package called `p`:
// `q.p` is outside it.
package p {
  object E:
    extension (s: String) private[p] def f: Int = 1
  object InP:
    import E.*
    val ok = "".f
}
package q.p {
  object Use:
    import _root_.p.E.*
    val result = "".f
}
