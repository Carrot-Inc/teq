// A copy of a stored inline body takes fresh binders with complete signatures: `local`'s
// parameter and `z` are fresh at each copy, their types `A` substituted, the paths of the
// renamed terms moved; the census checks the copies' trees, signatures
// and types.
inline def make[A](x: A) = {
  def local(y: A): A = x
  (z: A) => local(z)
}
@main def run(): Unit = println(make[Int](3)(4))
