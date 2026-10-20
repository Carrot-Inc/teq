// A default getter's expected type is `Namer.expectedDefaultArgType`: the parameter's type with each occurrence of a
// type parameter a wildcard of its bounds (`wildApprox`), which `ConstraintHandling.legalBound` turns into a fresh type
// variable where it bounds the default's own, interpolated with the default's application: `List[T => Unit] =
// List.empty` is a `List[Any => Unit]`, `List[T] = List.empty` a `List[Nothing]`, `(T, T) = (1, 2)` an `(Int, Int)`,
// `Map[String, T => Unit] = Map.empty` a `Map[String, Any => Unit]`, and `Map[T, T] = Map(1 -> "a")` a
// `Map[Int, String]`, which the call then cannot fit to `Map[T, T]` (scalac 3.8.4: tests/pos/infer1.scala).
// expect: M.scala:19:12: error: type mismatch: found Map[Int, String], required Map[T, T]
// expect: 1 error found
object G:
  def f[T](xs: List[T => Unit] = List.empty): List[T => Unit] = xs
  def g[T](xs: List[T] = List.empty): List[T] = xs
  def h[T](m: Map[T, T] = Map(1 -> "a")): Map[T, T] = m
  def k[T](p: (T, T) = (1, 2)): (T, T) = p
  def m[T](h: Map[String, T => Unit] = Map.empty): Map[String, T => Unit] = h
  val rf: List[Any => Unit] = f()
  val rg: List[Nothing] = g()
  val rk: (Int, Int) = k()
  val rm: Map[String, Any => Unit] = m()
  val rh = h()
