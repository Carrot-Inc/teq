// An inline val's declared type may be the singleton of another inline val whose own declared type is a singleton
// path (`m.type` over `Int.MaxValue.type`, `c.type` over `t.type`): its constant is the path's, widened through each
// val's declared type, and the reads are folded without initialising the object.
object O:
  println("init")
  inline val m: Int.MaxValue.type = Int.MaxValue
  inline val a: m.type = m
  final val t = true
  inline val c: t.type = t
  inline val d: c.type = c
@main def run(): Unit = println((O.a, O.c, O.d))
