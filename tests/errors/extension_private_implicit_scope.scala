// expect: 12:18: error: value f is not a member of A
// A `private[p]` extension of a companion is a candidate of the implicit scope inside `p` alone.
package p {
  class A
  object A:
    extension (a: A) private[p] def f: Int = 1
  object InP:
    val ok = (new A).f
}
package q {
  object Use:
    val result = (new p.A).f
}
