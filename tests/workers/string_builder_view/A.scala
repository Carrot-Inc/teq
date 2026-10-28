// The push pass's reduction of the view abort in `base_type`'s cache: the std's `StringBuilder`, whose JDK
// constructors the loader's lock holder absorbs, built in two methods, one typed against `CharSequence`. scalac
// accepts it.
object A {
  def f: CharSequence = new java.lang.StringBuilder("world")
  def g = new java.lang.StringBuilder("x")
}
