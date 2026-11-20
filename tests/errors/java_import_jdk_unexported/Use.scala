// An import of a package under the JDK's roots that the JDK's index (ct.sym) does not hold, `jdk.internal.misc`, which
// the reference reads from the runtime image: no problem at the import, an unused one included; the name stays the
// import's, so neither the package's own `p.Unsafe` nor the on-demand import's `q.Unsafe` takes it, and its use is
// refused where it stands (this reader does not see the package), whatever overload a caller would pick.
// expect: J.java:6:26: error: Not found: type Unsafe
object Use { def f = p.J.f(new p.Unsafe) }
