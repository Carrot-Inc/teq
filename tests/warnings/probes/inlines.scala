// An import inside an inline definition is never reported; an outer import used by an inline
// body is used.
object Lib { val a = 1; val b = 2 }
object Inline {
  inline def f: Int = {
    import Lib.a
    1
  }
}
object Outer {
  import Lib.b
  inline def g: Int = b
}
