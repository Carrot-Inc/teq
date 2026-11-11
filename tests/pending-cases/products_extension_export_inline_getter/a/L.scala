package lib
class Ops(val s: String):
  inline def h2(b: Int = 4, inline a: Int = 3): Int = b * 10 + a + s.length * 100
  inline def r(a: Int)(inline b: Int = 1, inline c: Int = a + 2): Int = a * 100 + b * 10 + c + s.length * 1000
@annotation.nowarn("msg=should not have a default argument")
class WCG[X](val x: X):
  extension (s: String)
    def ops: Ops = Ops(s.concat(x.toString))
    export ops.{h2, r}
