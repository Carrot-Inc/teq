// An export of a case class's synthetic `apply` from its companion forwards the getters under `<init>`'s name
// (`foreachDefaultGetterOf`'s `useConstructor`, Namer.scala:1249-1262), and a call's default is the getter of the
// method's own name (`findDefaultGetter`, Applications.scala:382-402): the forwarder `apply`, or `mk2` renamed, has
// none, so an omitted argument is missing. scalac 3.8.4: E171 at both calls.
// expect: 14:18: error: missing argument for parameter x of method apply in object Z: (x: Int, y: Int): Pt
// expect: 15:17: error: missing argument for parameter x of method mk2 in object Z2: (x: Int, y: Int): Pt
// expect: 2 errors found
case class Pt(x: Int = 1, y: Int = 2)
object Z:
  export Pt.apply
object Z2:
  export Pt.{apply as mk2}
@main def run(): Unit =
  println(Z.apply(y = 5))
  println(Z2.mk2(y = 5))
