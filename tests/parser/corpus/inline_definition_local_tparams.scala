// A copy of a stored inline body makes its local methods' type parameters afresh: `local`'s `B`
// is a type parameter of each copy, bounded by what stands for `A` there, and `pair`'s `C` by
// the copy's `B`; the census checks their identities and bounds.
inline def f[A](x: A) = {
  def local[B <: A](y: B): B = y
  def pair[B <: A, C <: B](b: B, c: C): (B, C) = (b, c)
  (local(x), pair(x, x))
}
@main def run(): Unit = println(f(3))
