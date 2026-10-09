// expect: 11:18: error: value f is not a member of String
// A top-level `private[p]` extension is a candidate inside `p` alone.
package p {
  extension (s: String) private[p] def f: Int = 1
  object InP:
    val ok = "".f
}
package q {
  object Use:
    import p.*
    val result = "".f
}
