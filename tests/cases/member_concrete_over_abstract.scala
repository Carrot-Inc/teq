// A class's member is the concrete definition where one parent declares it abstract and a parent
// further down the linearisation defines it (dotty's mergeDenot takes the concrete of the two),
// and a name imported from an object is selected anew on the object, its owner's type
// arguments given by the object's base type, as a selection of it is.
package bundle:
  trait Util[B, O <: F, F]:
    def tag(s: String): List[(B, O, F)] = List((null.asInstanceOf[B], s.asInstanceOf[O], s.asInstanceOf[F]))
  trait TagFactory:
    def tag(s: String): Any
  trait Bundle[B, O <: F, F]:
    type U = Util[B, O, F]
  object Text extends Bundle[Int, String, String]:
    trait Cap extends U, TagFactory

package app:
  object T extends bundle.Text.Cap
  object App:
    import T.*
    def a: List[(Int, String, String)] = tag("x")
    def b: List[(Int, String, String)] = T.tag("y")
    def c: Any = (T: bundle.TagFactory).tag("z")

  @main def run(): Unit =
    println(App.a.map(_._2))
    println(App.b.map(_._3))
    println(App.c)
