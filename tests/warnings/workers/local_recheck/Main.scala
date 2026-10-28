object Old { @deprecated("old", "1") def v: Int = 1 }
class Box { val k = 2 }
object A {
 def f(): Int = {
   val first = g()
   lazy val source = new Box
   import source.k
   inline def g(): Int = { Old.v; k }
   first + g()
 }
}
