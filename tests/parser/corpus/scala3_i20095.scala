// Adapted from scala3 tests/run/i20095.scala (Apache-2.0, see tests/scala3/README.md).
inline def twice(inline thunk: =>Unit): Unit =
  thunk
  thunk

inline def pipe(inline t: =>Unit, inline f: (=>Unit) => Unit): Unit = f(t)

@main def Test =
  pipe((), twice)
  pipe(println("foo"), twice)
