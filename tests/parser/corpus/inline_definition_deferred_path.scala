// A deferred call in a copy of a stored inline body takes the copy's paths: `id[y.type]`'s type
// argument names the fresh `y` of each copy, not the stored body's; the census checks the
// copies' deferred records.
inline def id[A](x: A): A = x
inline def f(x: AnyRef) = {
  val y = x
  id[y.type](y)
}
@main def run(): Unit = println(f("s"))
