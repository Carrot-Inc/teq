// The objects and classes an inline body nests in one another are copied with their bodies at each expansion, each
// once: a nested definition's statement in its owner's template is renewed after the copy's bodies are filled
// (TreeTypeMap's transformDefs after mapSymbols), so no copy is typed again at the site. scalac prints the lines of
// the .expected file.
class P(private val p: Int):
  inline def nested: Int = { object O1 { object O2 { val n = p * 10 }; val m = O2.n + p }; O1.m + O1.O2.n }
  inline def deep: Int =
    object A:
      object B:
        object C:
          val c = p + 1
        val b = C.c * 2
      class K { def k = B.b + B.C.c }
      val a = new K().k
    A.a
  inline def classes: Int = { class Q { class R { def r = p + 3 }; def q = new R().r * 2 }; new Q().q }
  inline def three: Int =
    object A:
      val a = p + 1
      object B:
        val b = a * 2
        object C:
          val c = b + p
    A.B.C.c
@main def run(): Unit =
  val x = P(2)
  println(x.nested)
  println(x.nested + P(5).nested)
  println(x.deep)
  println(x.classes)
  println(x.three)
