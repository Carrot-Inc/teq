// jars: scala-library zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// ZIO's tag derivation pairs the type parameters that `Symbol.typeMembers` lists for a class with
// the arguments of its type, and refuses an intersection among them with the library's own
// message, as it does under scalac.
// expect: 9:44: error: You must not use an intersection type, yet have provided Wax & Wick
trait Wax
trait Wick

val tag = summon[zio.Tag[List[Wax & Wick]]]
